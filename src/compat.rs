use anyhow::{anyhow, Result};
use std::future::Future;
use std::time::Duration;

// --- Sleep ---

#[cfg(not(feature = "worker"))]
/// Platform-agnostic sleep function.
pub async fn sleep(duration: Duration) {
    use tokio::time;

    time::sleep(duration).await;
}

#[cfg(feature = "worker")]
/// Platform-agnostic sleep function.
pub async fn sleep(duration: Duration) {
    worker::Delay::from(duration).await;
}

// --- Timeout ---

#[cfg(not(feature = "worker"))]
/// Platform-agnostic timeout function.
pub async fn timeout<F, T>(duration: Duration, future: F) -> Result<T>
where
    F: Future<Output = Result<T>> + Send, // Future must be Send for tokio
    T: Send + 'static,                    // Result must be Send for tokio
{
    tokio::time::timeout(duration, future)
        .await
        .map_err(|_| anyhow!("Operation timed out after {:?}", duration))? // Convert TimeoutElapsed to anyhow::Error
}

#[cfg(feature = "worker")]
/// Platform-agnostic timeout function.
pub async fn timeout<F, T>(duration: Duration, future: F) -> Result<T>
where
    F: Future<Output = Result<T>> + 'static, // 'static required for select
    T: 'static,                              // 'static required for select
{
    use futures_util::future::{select, Either};

    let delay = worker::Delay::from(duration);

    // Pin the futures on the stack
    futures_util::pin_mut!(future);
    futures_util::pin_mut!(delay);

    match select(future, delay).await {
        Either::Left((result, _)) => {
            // The future completed first
            result
        }
        Either::Right((_, _)) => {
            // The delay completed first
            Err(anyhow!("Operation timed out after {:?}", duration))
        }
    }
}

// --- Logging ---
// Simple console logging for Wasm if needed, or use tracing-wasm/log-wasm later
#[cfg(all(feature = "worker", feature = "log-native"))] // Example: Use log if enabled, even in worker
#[macro_export]
macro_rules! platform_log {
    (warn, $($t:tt)*) => (worker::console_warn!($($t)*))
    // Add other levels (info, error, debug, trace) if needed
}

#[cfg(all(not(feature = "worker"), feature = "log-native"))]
#[macro_export]
macro_rules! platform_log {
    (warn, $($t:tt)*) => (log::warn!($($t)*))
    // Add other levels
}

#[cfg(not(feature = "log-native"))] // No-op if log feature is disabled
#[macro_export]
macro_rules! platform_log {
    ($level:ident, $($t:tt)*) => {};
}
