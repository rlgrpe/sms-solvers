//! Builder for SmsSolverService.

use super::config::{SmsSolverServiceConfig, SmsSolverServiceConfigBuilder};
use super::core::SmsSolverService;
use crate::errors::RetryableError;
use crate::providers::traits::Provider;
use std::fmt::{Debug, Display};

/// Builder for SmsSolverService.
///
/// Provides a fluent API for constructing an SMS service with a provider
/// and custom configuration.
///
/// # Example
///
/// ```rust,ignore
/// use sms_solvers::{SmsSolverService, Provider};
/// use std::time::Duration;
///
/// let service = SmsSolverService::builder(provider)
///     .timeout(Duration::from_secs(180))
///     .poll_interval(Duration::from_secs(5))
///     .try_build()?;
/// ```
#[derive(Debug, Clone)]
pub struct SmsSolverServiceBuilder<P: Provider> {
    provider: P,
    config_builder: SmsSolverServiceConfigBuilder,
}

impl<P: Provider> SmsSolverServiceBuilder<P>
where
    P::Error: Debug + Display + RetryableError,
{
    /// Create a new builder with the given provider.
    pub fn new(provider: P) -> Self {
        Self {
            provider,
            config_builder: SmsSolverServiceConfigBuilder::default(),
        }
    }

    /// Set the timeout for waiting for SMS codes.
    ///
    /// Default: 120 seconds
    pub fn timeout(mut self, timeout: std::time::Duration) -> Self {
        self.config_builder = self.config_builder.timeout(timeout);
        self
    }

    /// Set the polling interval when waiting for SMS codes.
    ///
    /// Default: 3 seconds
    pub fn poll_interval(mut self, interval: std::time::Duration) -> Self {
        self.config_builder = self.config_builder.poll_interval(interval);
        self
    }

    /// Set the full configuration.
    pub fn config(mut self, config: SmsSolverServiceConfig) -> Self {
        self.config_builder = SmsSolverServiceConfigBuilder {
            timeout: config.timeout,
            poll_interval: config.poll_interval,
        };
        self
    }

    /// Build and validate the SmsSolverService.
    ///
    /// Returns an error if the configuration is invalid.
    pub fn try_build(self) -> Result<SmsSolverService<P>, super::config::ConfigError> {
        let config = self.config_builder.try_build()?;
        Ok(SmsSolverService::new_unchecked(self.provider, config))
    }

    /// Build the SmsSolverService without validation.
    ///
    /// Prefer [`try_build`](Self::try_build) for validated construction.
    #[deprecated(note = "Use `try_build` for validated construction")]
    pub fn build(self) -> SmsSolverService<P> {
        SmsSolverService::new_unchecked(self.provider, self.config_builder.build())
    }
}
