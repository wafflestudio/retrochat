use backoff::{backoff::Backoff, ExponentialBackoff};
use std::time::Duration;

use super::errors::{GoogleAiError, RetryError};

#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_attempts: usize,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub multiplier: f64,
    pub jitter: bool,
    pub total_timeout: Duration,
    pub ignore_server_retry_after: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 5,
            initial_delay: Duration::from_millis(500),
            max_delay: Duration::from_secs(30),
            multiplier: 2.0,
            jitter: true,
            total_timeout: Duration::from_secs(300), // 5 minutes total
            ignore_server_retry_after: false,
        }
    }
}

impl RetryConfig {
    pub fn new(max_attempts: usize) -> Self {
        Self {
            max_attempts,
            ..Default::default()
        }
    }

    pub fn with_ignore_server_retry_after(mut self, ignore: bool) -> Self {
        self.ignore_server_retry_after = ignore;
        self
    }

    pub fn with_total_timeout(mut self, timeout: Duration) -> Self {
        self.total_timeout = timeout;
        self
    }

    pub fn with_initial_delay(mut self, delay: Duration) -> Self {
        self.initial_delay = delay;
        self
    }

    pub fn with_max_delay(mut self, delay: Duration) -> Self {
        self.max_delay = delay;
        self
    }

    pub fn aggressive() -> Self {
        Self {
            max_attempts: 10,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
            multiplier: 1.5,
            jitter: true,
            total_timeout: Duration::from_secs(180),
            ignore_server_retry_after: false,
        }
    }

    pub fn conservative() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(60),
            multiplier: 3.0,
            jitter: false,
            total_timeout: Duration::from_secs(600),
            ignore_server_retry_after: false,
        }
    }
}

pub struct RetryHandler {
    config: RetryConfig,
    backoff: ExponentialBackoff,
    start_time: std::time::Instant,
    attempts: usize,
}

impl RetryHandler {
    pub fn new(config: RetryConfig) -> Self {
        let mut backoff = ExponentialBackoff {
            initial_interval: config.initial_delay,
            max_interval: config.max_delay,
            multiplier: config.multiplier,
            max_elapsed_time: Some(config.total_timeout),
            ..Default::default()
        };

        if !config.jitter {
            backoff.randomization_factor = 0.0;
        }

        Self {
            config,
            backoff,
            start_time: std::time::Instant::now(),
            attempts: 0,
        }
    }

    pub async fn retry<F, Fut, T>(&mut self, mut operation: F) -> Result<T, RetryError>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<T, GoogleAiError>>,
    {
        loop {
            self.attempts += 1;

            // Check if we've exceeded max attempts
            if self.attempts > self.config.max_attempts {
                return Err(RetryError::MaxAttemptsExceeded);
            }

            // Check if we've exceeded total timeout
            if self.start_time.elapsed() > self.config.total_timeout {
                return Err(RetryError::TimeoutExceeded);
            }

            match operation().await {
                Ok(result) => return Ok(result),
                Err(error) => {
                    // Check if error is retryable
                    if !error.is_retryable() {
                        return Err(RetryError::NonRetryable { source: error });
                    }

                    // For the last attempt, don't wait
                    if self.attempts >= self.config.max_attempts {
                        return Err(RetryError::MaxAttemptsExceeded);
                    }

                    // Calculate delay
                    let delay = if !self.config.ignore_server_retry_after {
                        if let Some(retry_after) = error.retry_after_seconds() {
                            // Use server-suggested delay if available and not ignored
                            Duration::from_secs(retry_after)
                        } else {
                            // Use exponential backoff
                            self.backoff.next_backoff().unwrap_or(self.config.max_delay)
                        }
                    } else {
                        // Use exponential backoff
                        self.backoff.next_backoff().unwrap_or(self.config.max_delay)
                    };

                    tracing::warn!(
                        "Attempt {} failed with retryable error: {}. Retrying in {:?}",
                        self.attempts,
                        error,
                        delay
                    );

                    tokio::time::sleep(delay).await;
                }
            }
        }
    }

    pub fn attempts(&self) -> usize {
        self.attempts
    }

    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }

    pub fn reset(&mut self) {
        self.attempts = 0;
        self.start_time = std::time::Instant::now();
        self.backoff.reset();
    }
}

#[derive(Debug)]
pub struct RetryMetrics {
    pub total_attempts: usize,
    pub total_duration: Duration,
    pub successful_retries: usize,
    pub failed_retries: usize,
    pub non_retryable_errors: usize,
}

impl RetryMetrics {
    pub fn new() -> Self {
        Self {
            total_attempts: 0,
            total_duration: Duration::ZERO,
            successful_retries: 0,
            failed_retries: 0,
            non_retryable_errors: 0,
        }
    }

    pub fn record_attempt(&mut self, duration: Duration, result: &Result<(), RetryError>) {
        self.total_attempts += 1;
        self.total_duration += duration;

        match result {
            Ok(_) => self.successful_retries += 1,
            Err(RetryError::MaxAttemptsExceeded) | Err(RetryError::TimeoutExceeded) => {
                self.failed_retries += 1;
            }
            Err(RetryError::NonRetryable { .. }) => {
                self.non_retryable_errors += 1;
            }
        }
    }

    pub fn success_rate(&self) -> f64 {
        if self.total_attempts == 0 {
            0.0
        } else {
            self.successful_retries as f64 / self.total_attempts as f64
        }
    }

    pub fn average_duration(&self) -> Duration {
        if self.total_attempts == 0 {
            Duration::ZERO
        } else {
            self.total_duration / self.total_attempts as u32
        }
    }
}

impl Default for RetryMetrics {
    fn default() -> Self {
        Self::new()
    }
}

pub async fn with_retry<F, Fut, T>(config: RetryConfig, operation: F) -> Result<T, RetryError>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, GoogleAiError>>,
{
    let mut handler = RetryHandler::new(config);
    handler.retry(operation).await
}

pub async fn with_default_retry<F, Fut, T>(operation: F) -> Result<T, RetryError>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, GoogleAiError>>,
{
    with_retry(RetryConfig::default(), operation).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_retry_success_after_failures() {
        let attempt_count = Arc::new(AtomicUsize::new(0));
        let attempt_count_clone = attempt_count.clone();

        let config = RetryConfig {
            max_attempts: 3,
            initial_delay: Duration::from_millis(10),
            ignore_server_retry_after: true,
            ..Default::default()
        };

        let result = with_retry(config, move || {
            let count = attempt_count_clone.fetch_add(1, Ordering::SeqCst) + 1;
            async move {
                if count < 3 {
                    Err(GoogleAiError::RateLimitExceeded {
                        message: "Rate limited".to_string(),
                    })
                } else {
                    Ok("success")
                }
            }
        })
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "success");
        assert_eq!(attempt_count.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_retry_non_retryable_error() {
        let config = RetryConfig {
            max_attempts: 3,
            initial_delay: Duration::from_millis(10),
            ignore_server_retry_after: true,
            ..Default::default()
        };

        let result: Result<(), RetryError> = with_retry(config, || async {
            Err(GoogleAiError::AuthenticationFailed {
                message: "Invalid API key".to_string(),
            })
        })
        .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            RetryError::NonRetryable { .. } => (),
            _ => panic!("Expected NonRetryable error"),
        }
    }

    #[tokio::test]
    async fn test_retry_max_attempts_exceeded() {
        let config = RetryConfig {
            max_attempts: 2,
            initial_delay: Duration::from_millis(10),
            ignore_server_retry_after: true,
            ..Default::default()
        };

        let result: Result<(), RetryError> = with_retry(config, || async {
            Err(GoogleAiError::RateLimitExceeded {
                message: "Rate limited".to_string(),
            })
        })
        .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            RetryError::MaxAttemptsExceeded => (),
            _ => panic!("Expected MaxAttemptsExceeded error"),
        }
    }
}
