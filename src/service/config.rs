//! Service configuration types.

use std::time::Duration;
use thiserror::Error;

/// Error when validating service configuration.
#[derive(Debug, Clone, Error)]
pub enum ConfigError {
    /// Timeout is too short.
    #[error("Timeout ({timeout:?}) must be at least {min:?}")]
    TimeoutTooShort {
        /// The configured timeout.
        timeout: Duration,
        /// The minimum allowed timeout.
        min: Duration,
    },
    /// Poll interval is too short.
    #[error("Poll interval ({poll_interval:?}) must be at least {min:?}")]
    PollIntervalTooShort {
        /// The configured poll interval.
        poll_interval: Duration,
        /// The minimum allowed poll interval.
        min: Duration,
    },
    /// Poll interval is longer than timeout.
    #[error("Poll interval ({poll_interval:?}) must be less than timeout ({timeout:?})")]
    PollIntervalExceedsTimeout {
        /// The configured poll interval.
        poll_interval: Duration,
        /// The configured timeout.
        timeout: Duration,
    },
}

/// Minimum allowed timeout (10 seconds).
pub const MIN_TIMEOUT: Duration = Duration::from_secs(10);

/// Minimum allowed poll interval (100ms).
pub const MIN_POLL_INTERVAL: Duration = Duration::from_millis(100);

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
        Self::balanced()
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

    /// Fast configuration preset.
    ///
    /// Uses a shorter timeout (60s) and aggressive polling (1s).
    /// Good for development and testing.
    ///
    /// - Timeout: 60 seconds
    /// - Poll interval: 1 second
    pub fn fast() -> Self {
        Self {
            timeout: Duration::from_secs(60),
            poll_interval: Duration::from_secs(1),
        }
    }

    /// Balanced configuration preset (default).
    ///
    /// Uses moderate timeout and polling interval.
    /// Good for most production use cases.
    ///
    /// - Timeout: 120 seconds
    /// - Poll interval: 3 seconds
    pub fn balanced() -> Self {
        Self {
            timeout: Duration::from_secs(120),
            poll_interval: Duration::from_secs(3),
        }
    }

    /// Patient configuration preset.
    ///
    /// Uses a longer timeout (300s) and relaxed polling (5s).
    /// Good for slow providers or unreliable networks.
    ///
    /// - Timeout: 300 seconds (5 minutes)
    /// - Poll interval: 5 seconds
    pub fn patient() -> Self {
        Self {
            timeout: Duration::from_secs(300),
            poll_interval: Duration::from_secs(5),
        }
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

    /// Validate the configuration.
    ///
    /// Returns an error if:
    /// - Timeout is less than 10 seconds
    /// - Poll interval is less than 100ms
    /// - Poll interval is greater than or equal to timeout
    ///
    /// # Example
    ///
    /// ```rust
    /// use sms_solvers::SmsSolverServiceConfig;
    /// use std::time::Duration;
    ///
    /// // Valid config
    /// let config = SmsSolverServiceConfig::default();
    /// assert!(config.validate().is_ok());
    ///
    /// // Invalid: poll interval >= timeout
    /// let config = SmsSolverServiceConfig::builder()
    ///     .timeout(Duration::from_secs(10))
    ///     .poll_interval(Duration::from_secs(15))
    ///     .build();
    /// assert!(config.validate().is_err());
    /// ```
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.timeout < MIN_TIMEOUT {
            return Err(ConfigError::TimeoutTooShort {
                timeout: self.timeout,
                min: MIN_TIMEOUT,
            });
        }

        if self.poll_interval < MIN_POLL_INTERVAL {
            return Err(ConfigError::PollIntervalTooShort {
                poll_interval: self.poll_interval,
                min: MIN_POLL_INTERVAL,
            });
        }

        if self.poll_interval >= self.timeout {
            return Err(ConfigError::PollIntervalExceedsTimeout {
                poll_interval: self.poll_interval,
                timeout: self.timeout,
            });
        }

        Ok(())
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
        let config = SmsSolverServiceConfig::balanced();
        Self {
            timeout: config.timeout,
            poll_interval: config.poll_interval,
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
    ///
    /// Note: This does not validate the configuration. Use `try_build()`
    /// to validate the configuration before building.
    pub fn build(self) -> SmsSolverServiceConfig {
        SmsSolverServiceConfig {
            timeout: self.timeout,
            poll_interval: self.poll_interval,
        }
    }

    /// Build and validate the SmsSolverServiceConfig.
    ///
    /// Returns an error if the configuration is invalid.
    ///
    /// # Example
    ///
    /// ```rust
    /// use sms_solvers::SmsSolverServiceConfig;
    /// use std::time::Duration;
    ///
    /// // Valid config
    /// let config = SmsSolverServiceConfig::builder()
    ///     .timeout(Duration::from_secs(60))
    ///     .poll_interval(Duration::from_secs(2))
    ///     .try_build()
    ///     .expect("valid config");
    ///
    /// // Invalid config: poll interval > timeout
    /// let result = SmsSolverServiceConfig::builder()
    ///     .timeout(Duration::from_secs(10))
    ///     .poll_interval(Duration::from_secs(20))
    ///     .try_build();
    /// assert!(result.is_err());
    /// ```
    pub fn try_build(self) -> Result<SmsSolverServiceConfig, ConfigError> {
        let config = self.build();
        config.validate()?;
        Ok(config)
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
    fn test_config_presets() {
        let fast = SmsSolverServiceConfig::fast();
        assert_eq!(fast.timeout, Duration::from_secs(60));
        assert_eq!(fast.poll_interval, Duration::from_secs(1));

        let balanced = SmsSolverServiceConfig::balanced();
        assert_eq!(balanced.timeout, Duration::from_secs(120));
        assert_eq!(balanced.poll_interval, Duration::from_secs(3));

        let patient = SmsSolverServiceConfig::patient();
        assert_eq!(patient.timeout, Duration::from_secs(300));
        assert_eq!(patient.poll_interval, Duration::from_secs(5));
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

    #[test]
    fn test_config_validation_success() {
        let config = SmsSolverServiceConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validation_timeout_too_short() {
        let config = SmsSolverServiceConfig::builder()
            .timeout(Duration::from_secs(5))
            .build();
        assert!(matches!(
            config.validate(),
            Err(ConfigError::TimeoutTooShort { .. })
        ));
    }

    #[test]
    fn test_config_validation_poll_interval_too_short() {
        let config = SmsSolverServiceConfig::builder()
            .poll_interval(Duration::from_millis(50))
            .build();
        assert!(matches!(
            config.validate(),
            Err(ConfigError::PollIntervalTooShort { .. })
        ));
    }

    #[test]
    fn test_config_validation_poll_exceeds_timeout() {
        let config = SmsSolverServiceConfig::builder()
            .timeout(Duration::from_secs(30))
            .poll_interval(Duration::from_secs(60))
            .build();
        assert!(matches!(
            config.validate(),
            Err(ConfigError::PollIntervalExceedsTimeout { .. })
        ));
    }

    #[test]
    fn test_try_build_success() {
        let config = SmsSolverServiceConfig::builder()
            .timeout(Duration::from_secs(60))
            .poll_interval(Duration::from_secs(2))
            .try_build();
        assert!(config.is_ok());
    }

    #[test]
    fn test_try_build_failure() {
        let result = SmsSolverServiceConfig::builder()
            .timeout(Duration::from_secs(10))
            .poll_interval(Duration::from_secs(20))
            .try_build();
        assert!(result.is_err());
    }
}
