//! Service configuration types.

use std::time::Duration;

/// Configuration for the SMS Solver Service.
///
/// Controls timeout and polling behavior when waiting for SMS codes.
#[derive(Debug, Clone)]
pub struct SmsSolverServiceConfig {
    /// Maximum time to wait for SMS code before timing out.
    pub timeout: Duration,
    /// Interval between polling attempts when waiting for SMS.
    pub poll_interval: Duration,
}

impl Default for SmsSolverServiceConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(120),
            poll_interval: Duration::from_secs(3),
        }
    }
}

impl SmsSolverServiceConfig {
    /// Create a new builder for SmsSolverServiceConfig.
    ///
    /// # Example
    ///
    /// ```rust
    /// use sms_solvers::SmsSolverServiceConfig;
    /// use std::time::Duration;
    ///
    /// let config = SmsSolverServiceConfig::builder()
    ///     .timeout(Duration::from_secs(180))
    ///     .poll_interval(Duration::from_secs(5))
    ///     .build();
    ///
    /// assert_eq!(config.timeout, Duration::from_secs(180));
    /// assert_eq!(config.poll_interval, Duration::from_secs(5));
    /// ```
    pub fn builder() -> SmsSolverServiceConfigBuilder {
        SmsSolverServiceConfigBuilder::default()
    }

    /// Create a new config with a custom timeout.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Create a new config with a custom poll interval.
    pub fn with_poll_interval(mut self, interval: Duration) -> Self {
        self.poll_interval = interval;
        self
    }
}

/// Builder for SmsSolverServiceConfig.
///
/// Provides a fluent API for configuring the SMS service.
#[derive(Debug, Clone)]
pub struct SmsSolverServiceConfigBuilder {
    pub(crate) timeout: Duration,
    pub(crate) poll_interval: Duration,
}

impl Default for SmsSolverServiceConfigBuilder {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(120),
            poll_interval: Duration::from_secs(3),
        }
    }
}

impl SmsSolverServiceConfigBuilder {
    /// Create a new builder with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the timeout for waiting for SMS codes.
    ///
    /// Default: 120 seconds
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set the polling interval when waiting for SMS codes.
    ///
    /// Default: 3 seconds
    pub fn poll_interval(mut self, interval: Duration) -> Self {
        self.poll_interval = interval;
        self
    }

    /// Build the SmsSolverServiceConfig.
    pub fn build(self) -> SmsSolverServiceConfig {
        SmsSolverServiceConfig {
            timeout: self.timeout,
            poll_interval: self.poll_interval,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_config_default() {
        let config = SmsSolverServiceConfig::default();
        assert_eq!(config.timeout, Duration::from_secs(120));
        assert_eq!(config.poll_interval, Duration::from_secs(3));
    }

    #[test]
    fn test_config_builder() {
        let config = SmsSolverServiceConfig::builder()
            .timeout(Duration::from_secs(180))
            .poll_interval(Duration::from_secs(5))
            .build();

        assert_eq!(config.timeout, Duration::from_secs(180));
        assert_eq!(config.poll_interval, Duration::from_secs(5));
    }

    #[test]
    fn test_config_builder_default() {
        let config = SmsSolverServiceConfigBuilder::new().build();
        assert_eq!(config.timeout, Duration::from_secs(120));
        assert_eq!(config.poll_interval, Duration::from_secs(3));
    }

    #[test]
    fn test_config_with_methods() {
        let config = SmsSolverServiceConfig::default()
            .with_timeout(Duration::from_secs(60))
            .with_poll_interval(Duration::from_secs(1));

        assert_eq!(config.timeout, Duration::from_secs(60));
        assert_eq!(config.poll_interval, Duration::from_secs(1));
    }
}
