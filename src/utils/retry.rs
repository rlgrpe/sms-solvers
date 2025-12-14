//! Retry configuration for SMS operations.

use backon::ExponentialBuilder;
use std::time::Duration;

/// Configuration for retry behavior.
///
/// Use the builder pattern to customize retry settings:
///
/// ```rust
/// use sms_solvers::RetryConfig;
/// use std::time::Duration;
///
/// let config = RetryConfig::default()
///     .with_min_delay(Duration::from_millis(500))
///     .with_max_delay(Duration::from_secs(60))
///     .with_factor(1.5)
///     .with_max_retries(5);
/// ```
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Minimum delay between retries (default: 1 second).
    pub min_delay: Duration,
    /// Maximum delay between retries (default: 30 seconds).
    pub max_delay: Duration,
    /// Exponential backoff factor (default: 2.0).
    pub factor: f32,
    /// Maximum number of retry attempts (default: 3).
    pub max_retries: usize,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            min_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(30),
            factor: 2.0,
            max_retries: 3,
        }
    }
}

impl RetryConfig {
    /// Set the minimum delay between retries.
    pub fn with_min_delay(mut self, delay: Duration) -> Self {
        self.min_delay = delay;
        self
    }

    /// Set the maximum delay between retries.
    pub fn with_max_delay(mut self, delay: Duration) -> Self {
        self.max_delay = delay;
        self
    }

    /// Set the exponential backoff factor.
    pub fn with_factor(mut self, factor: f32) -> Self {
        self.factor = factor;
        self
    }

    /// Set the maximum number of retry attempts.
    pub fn with_max_retries(mut self, max_retries: usize) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// Build a backoff strategy from this configuration.
    pub fn build_strategy(&self) -> ExponentialBuilder {
        ExponentialBuilder::default()
            .with_min_delay(self.min_delay)
            .with_max_delay(self.max_delay)
            .with_factor(self.factor)
            .with_max_times(self.max_retries)
    }
}
