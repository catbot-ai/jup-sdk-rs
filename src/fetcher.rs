use crate::compat;
use anyhow::{anyhow, Result};
use serde::de::DeserializeOwned;
use std::time::Duration;

// RetrySettings remains the same
#[derive(Debug, Clone)]
pub struct RetrySettings {
    pub max_retries: usize,
    pub request_timeout: Duration,
    pub base_backoff: Duration,
}

impl Default for RetrySettings {
    fn default() -> Self {
        Self {
            max_retries: 3,
            request_timeout: Duration::from_secs(10),
            base_backoff: Duration::from_secs(2), // Start with 2 seconds
        }
    }
}

impl RetrySettings {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn with_max_retries(mut self, max_retries: usize) -> Self {
        self.max_retries = max_retries;
        self
    }
    pub fn with_request_timeout(mut self, timeout: Duration) -> Self {
        self.request_timeout = timeout;
        self
    }
    pub fn with_base_backoff(mut self, backoff: Duration) -> Self {
        self.base_backoff = backoff;
        self
    }
}

// Helper function (remains the same)
fn exponential_backoff(retries: u32, base_backoff: Duration) -> Duration {
    if retries == 0 {
        base_backoff
    } else {
        let exponential_factor = 2u32.pow(retries.saturating_sub(1));
        base_backoff * exponential_factor
    }
}

// --- Unified Fetcher Implementation using reqwest ---
pub struct Fetcher {
    client: reqwest::Client,
    settings: RetrySettings,
}

impl Fetcher {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            settings: RetrySettings::default(),
        }
    }

    pub fn with_settings(settings: RetrySettings) -> Self {
        Self {
            client: reqwest::Client::new(),
            settings,
        }
    }

    pub async fn fetch_with_retry<T: DeserializeOwned + Send + 'static>(
        &self,
        url: &str, // Input URL is still a slice for the public API
    ) -> Result<T> {
        let url_owned = url.to_string(); // Create owned String immediately
        let mut retries = 0;

        loop {
            // Clone the client and URL for this attempt to move into the async block.
            let client_clone = self.client.clone(); // Clone the client handle
            let url_for_attempt = url_owned.clone();

            // Define the request sending future within the loop using async move
            let send_future = async move {
                // Use async move
                client_clone
                    .get(&url_for_attempt) // Use the client clone
                    .send()
                    .await // Send the request
                    .map_err(anyhow::Error::from) // Map reqwest::Error to anyhow::Error
            };

            // Wrap the send future with the timeout
            match compat::timeout(self.settings.request_timeout, send_future).await {
                Ok(response) => {
                    // Single Ok: timeout completed, future succeeded
                    let status = response.status();
                    if status.is_success() {
                        match response.json::<T>().await {
                            Ok(data) => return Ok(data), // Success!
                            Err(e) => {
                                // Deserialization error
                                return Err(anyhow!(
                                    "Failed to deserialize response from {}: {}",
                                    url_owned,
                                    e
                                )
                                .context(format!("Status: {}", status)));
                            }
                        }
                    } else {
                        // Non-success status code
                        let error_body_result = response.text().await;

                        if status.is_server_error() && retries < self.settings.max_retries {
                            retries += 1;
                            crate::platform_log!(
                                warn,
                                "Request to {} failed (attempt {}/{}): Status {}. Retrying...",
                                url_owned,
                                retries,
                                self.settings.max_retries + 1,
                                status
                            );
                            let delay =
                                exponential_backoff(retries as u32, self.settings.base_backoff);
                            compat::sleep(delay).await;
                            continue; // Retry loop
                        } else {
                            // Client error or max retries hit for 5xx
                            let error_body = error_body_result
                                .unwrap_or_else(|e| format!("Failed to read error body: {}", e));
                            return Err(anyhow!(
                                "Request to {} failed: Status {}, Body: {}",
                                url_owned,
                                status,
                                error_body
                            ));
                        }
                    }
                }
                // Timeout completed, but the inner future failed, OR timeout elapsed
                Err(e) => {
                    // e: anyhow::Error (could be timeout or reqwest error)
                    let is_timeout_error = e.to_string().contains("timed out");

                    // Conditionally check for retryable errors based on target
                    let is_underlying_retryable = if !is_timeout_error {
                        #[cfg(not(target_arch = "wasm32"))]
                        {
                            // Native target: Check specific error kinds if possible
                            e.downcast_ref::<reqwest::Error>()
                                .is_some_and(|re| re.is_connect() || re.is_request())
                        }
                        #[cfg(target_arch = "wasm32")]
                        {
                            // WASM target: Assume reqwest::Error here is likely a fetch/network issue and retryable.
                            // We can't reliably use is_connect() or is_request().
                            e.downcast_ref::<reqwest::Error>().is_some()
                        }
                    } else {
                        false // It's a timeout error, handled by is_timeout_error flag
                    };

                    if (is_timeout_error || is_underlying_retryable)
                        && retries < self.settings.max_retries
                    {
                        retries += 1;
                        crate::platform_log!(
                            warn,
                            "Request to {} failed or timed out (attempt {}/{}): {}. Retrying...",
                            url_owned,
                            retries,
                            self.settings.max_retries + 1,
                            e
                        );
                        let delay = exponential_backoff(retries as u32, self.settings.base_backoff);
                        compat::sleep(delay).await;
                        continue; // Retry loop
                    } else {
                        // Max retries reached or non-retryable error
                        let failure_context = if (is_timeout_error || is_underlying_retryable)
                            && retries >= self.settings.max_retries
                        {
                            format!("after {} attempts", self.settings.max_retries + 1)
                        } else if is_timeout_error {
                            "due to timeout".to_string()
                        } else {
                            "due to non-retryable error".to_string()
                        };
                        return Err(e.context(format!(
                            "Request to {} failed {}",
                            url_owned, failure_context
                        )));
                    }
                }
            }
        }
    }
}

impl Default for Fetcher {
    fn default() -> Self {
        Fetcher::new()
    }
}

// --- Tests ---
// Tests might need adjustment if they are ever run in a wasm environment,
// as the error types/messages under retry conditions might differ slightly.
#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))] // Keep tests gated for native for now
mod tests {
    use super::*;
    use serde::Deserialize;

    fn setup() {
        #[cfg(all(feature = "log-native", feature = "native"))]
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[derive(Deserialize, Debug, PartialEq)]
    struct TestTodo {
        #[serde(rename = "userId")]
        user_id: i32,
        id: i32,
        title: String,
        completed: bool,
    }

    #[tokio::test]
    async fn test_fetch_json_with_retry() -> Result<()> {
        setup();
        let fetcher = Fetcher::default();
        let url = "https://jsonplaceholder.typicode.com/todos/1";
        let todo: TestTodo = fetcher.fetch_with_retry(url).await?;
        assert_eq!(todo.id, 1);
        Ok(())
    }

    #[tokio::test]
    async fn test_fetch_timeout_retry() {
        setup();
        let url = "http://10.255.255.1:81"; // Non-routable IP
        let settings = RetrySettings::default()
            .with_request_timeout(Duration::from_millis(500))
            .with_base_backoff(Duration::from_millis(100))
            .with_max_retries(2);
        let fetcher = Fetcher::with_settings(settings);
        let result: Result<()> = fetcher.fetch_with_retry(url).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("after 3 attempts"));
        let root_cause = err.root_cause();
        // Native check for underlying error
        let is_timeout_or_connect = root_cause
            .downcast_ref::<reqwest::Error>()
            .is_some_and(|e| e.is_timeout() || e.is_connect())
            || root_cause.to_string().contains("timed out");
        assert!(
            is_timeout_or_connect,
            "Error should be due to timeout or connection issue"
        );
    }

    #[tokio::test]
    async fn test_fetch_404_no_retry() -> Result<()> {
        setup();
        let fetcher = Fetcher::default();
        let url = "https://jsonplaceholder.typicode.com/todos/999999999";
        let result: Result<TestTodo> = fetcher.fetch_with_retry(url).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Status 404"));
        assert!(!err.to_string().contains("attempts"));
        Ok(())
    }

    #[tokio::test]
    async fn test_fetch_500_with_retry() {
        setup();
        let url = "https://httpbin.org/status/503";
        let settings = RetrySettings::default()
            .with_request_timeout(Duration::from_secs(5))
            .with_base_backoff(Duration::from_millis(100))
            .with_max_retries(2);
        let fetcher = Fetcher::with_settings(settings);
        let result: Result<serde_json::Value> = fetcher.fetch_with_retry(url).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Status 503"));
        assert!(err.to_string().contains("failed: Status 503"));
        assert!(!err.to_string().contains("attempts"));
    }
}
