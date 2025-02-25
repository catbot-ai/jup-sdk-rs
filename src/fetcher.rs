use anyhow::{anyhow, Result};
use reqwest;
use serde::de::DeserializeOwned;
use tokio::time::{timeout, Duration};

#[cfg(test)]
use log::warn;

/// Helper function to calculate exponential backoff delay.
fn exponential_backoff(retries: u32, base_backoff: Duration) -> Duration {
    base_backoff * 2u32.pow(retries)
}

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
            base_backoff: Duration::from_secs(2),
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

pub struct Fetcher {
    settings: RetrySettings,
}

impl Default for Fetcher {
    fn default() -> Self {
        Self::new()
    }
}

impl Fetcher {
    pub fn new() -> Self {
        Self {
            settings: RetrySettings::default(),
        }
    }

    pub fn with_settings(settings: RetrySettings) -> Self {
        Self { settings }
    }

    pub async fn fetch_with_retry<F, R, T>(&self, url: &str, processor: F) -> Result<T>
    where
        R: DeserializeOwned,
        F: Fn(R) -> Result<T>,
    {
        let mut retries = 0;

        loop {
            match timeout(self.settings.request_timeout, reqwest::get(url)).await {
                Ok(response) => {
                    let response = response?;
                    let api_response = response.json::<R>().await?;
                    return processor(api_response);
                }
                Err(e) => {
                    retries += 1;
                    if retries >= self.settings.max_retries {
                        return Err(anyhow!("Request failed after {} retries: {}", retries, e));
                    }

                    #[cfg(test)]
                    warn!("Request failed (attempt {}): {}", retries, e);

                    let delay = exponential_backoff(retries as u32, self.settings.base_backoff);
                    tokio::time::sleep(delay).await;
                }
            }
        }
    }
}
