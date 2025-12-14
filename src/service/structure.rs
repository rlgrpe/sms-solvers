//! Main service implementation.

use super::config::{SmsSolverServiceConfig, SmsSolverServiceConfigBuilder};
use super::error::SmsSolverServiceError;
use super::traits::SmsSolverServiceTrait;
use crate::errors::RetryableError;
use crate::providers::traits::Provider;
use crate::types::{Number, SmsCode, SmsTaskResult, TaskId};
use crate::utils::dial_code::country_to_dial_code;
use isocountry::CountryCode;
use std::error::Error as StdError;
use std::fmt::{Debug, Display};
use std::time::Instant;

#[cfg(feature = "tracing")]
use tracing::{debug, error, info, warn};

/// Generic SMS service that works with any Provider implementation.
///
/// This service handles high-level SMS operations like:
/// - Getting a phone number from the provider
/// - Polling for SMS codes with timeout
/// - Managing activation lifecycle (finish/cancel)
///
/// The actual SMS provider logic is abstracted behind the `Provider` trait.
///
/// # Type Parameters
///
/// - `P`: The provider implementation (e.g., `SmsActivateProvider`)
///
/// # Example
///
/// ```rust,ignore
/// use sms_solvers::{SmsSolverService, SmsSolverServiceConfig, SmsSolverServiceTrait};
/// use sms_solvers::sms_activate::{SmsActivateClient, SmsActivateProvider, Service};
/// use std::time::Duration;
/// use isocountry::CountryCode;
///
/// // Create provider and service
/// let client = SmsActivateClient::with_api_key("api_key")?;
/// let provider = SmsActivateProvider::new(client);
/// let config = SmsSolverServiceConfig::default();
/// let service = SmsSolverService::new(provider, config);
///
/// // Get a phone number for WhatsApp
/// let result = service.get_number(CountryCode::USA, Service::Whatsapp).await?;
/// println!("Got number: {} (task_id: {})", result.full_number, result.task_id);
///
/// // Wait for SMS code
/// let code = service.wait_for_sms_code(&result.task_id).await?;
/// println!("Got code: {}", code);
/// ```
#[derive(Debug, Clone)]
pub struct SmsSolverService<P: Provider> {
    provider: P,
    config: SmsSolverServiceConfig,
}

impl<P: Provider> SmsSolverService<P>
where
    P::Error: Debug + Display + RetryableError,
{
    /// Create a new SMS service with a custom provider and configuration.
    pub fn new(provider: P, config: SmsSolverServiceConfig) -> Self {
        Self { provider, config }
    }

    /// Create a new SMS service with default configuration.
    pub fn with_provider(provider: P) -> Self {
        Self::new(provider, SmsSolverServiceConfig::default())
    }

    /// Create a new builder for SmsSolverService.
    pub fn builder(provider: P) -> SmsSolverServiceBuilder<P> {
        SmsSolverServiceBuilder::new(provider)
    }

    /// Get reference to the underlying provider.
    pub fn provider(&self) -> &P {
        &self.provider
    }

    /// Get mutable reference to the underlying provider.
    pub fn provider_mut(&mut self) -> &mut P {
        &mut self.provider
    }

    /// Get reference to the service configuration.
    pub fn config(&self) -> &SmsSolverServiceConfig {
        &self.config
    }

    /// Get mutable reference to the service configuration.
    pub fn config_mut(&mut self) -> &mut SmsSolverServiceConfig {
        &mut self.config
    }

    /// Update the service configuration.
    pub fn set_config(&mut self, config: SmsSolverServiceConfig) {
        self.config = config;
    }
}

impl<P: Provider> SmsSolverServiceTrait for SmsSolverService<P>
where
    P::Error: Debug + Display + RetryableError + Send + Sync + 'static,
{
    type Error = SmsSolverServiceError;
    type Service = P::Service;

    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(
            name = "sms_solver.get_number",
            skip_all,
            fields(country = %country)
        )
    )]
    async fn get_number(
        &self,
        country: CountryCode,
        service: Self::Service,
    ) -> Result<SmsTaskResult, Self::Error> {
        #[cfg(feature = "tracing")]
        debug!("Requesting phone number");

        let (task_id, full_number) = self
            .provider
            .get_phone_number(country, service)
            .await
            .map_err(|e| {
                let is_retryable = e.is_retryable();
                let should_retry_operation = e.should_retry_operation();
                SmsSolverServiceError::Provider {
                    source: Box::new(e) as Box<dyn StdError + Send + Sync>,
                    is_retryable,
                    should_retry_operation,
                }
            })?;

        let dial_code = country_to_dial_code(country).ok_or_else(|| {
            SmsSolverServiceError::InvalidDialCode {
                dial_code: "unknown".to_string(),
                country,
            }
        })?;

        let number = Number::from_full_number(&full_number, &dial_code).map_err(|e| {
            SmsSolverServiceError::NumberParse {
                full_number: full_number.to_string(),
                message: e.to_string(),
            }
        })?;

        #[cfg(feature = "tracing")]
        info!(
            task_id = %task_id,
            dial_code = %dial_code,
            country = %country.alpha2(),
            "Phone number acquired"
        );

        Ok(SmsTaskResult {
            task_id,
            dial_code,
            number,
            full_number,
            country,
        })
    }

    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(
            name = "sms_solver.wait_for_code",
            skip_all,
            fields(task_id = %task_id)
        )
    )]
    async fn wait_for_sms_code(&self, task_id: &TaskId) -> Result<SmsCode, Self::Error> {
        let timeout = self.config.timeout;
        let poll_interval = self.config.poll_interval;
        let start = Instant::now();

        #[cfg(feature = "tracing")]
        debug!(timeout_secs = %timeout.as_secs_f64(), "Starting SMS code polling");

        loop {
            if start.elapsed() >= timeout {
                #[cfg(feature = "tracing")]
                warn!(
                    timeout_secs = %timeout.as_secs_f64(),
                    "Timeout reached, cancelling activation"
                );

                if let Err(_e) = self.provider.cancel_activation(task_id).await {
                    #[cfg(feature = "tracing")]
                    warn!(error = %_e, "Failed to cancel activation after timeout");
                }

                return Err(SmsSolverServiceError::SmsTimeout {
                    timeout,
                    task_id: task_id.clone(),
                });
            }

            match self.provider.get_sms_code(task_id).await {
                Ok(Some(code)) => {
                    #[cfg(feature = "tracing")]
                    info!(
                        code = %code,
                        elapsed_secs = %start.elapsed().as_secs_f64(),
                        "SMS code received"
                    );
                    return Ok(code);
                }
                Ok(None) => {
                    // SMS not yet received, continue polling
                }
                Err(e) if !e.is_retryable() => {
                    let should_retry_operation = e.should_retry_operation();

                    #[cfg(feature = "tracing")]
                    error!(error = %e, "Permanent error during polling");

                    if let Err(_cancel_err) = self.provider.cancel_activation(task_id).await {
                        #[cfg(feature = "tracing")]
                        warn!(error = %_cancel_err, "Failed to cancel activation after error");
                    }

                    return Err(SmsSolverServiceError::Provider {
                        source: Box::new(e) as Box<dyn StdError + Send + Sync>,
                        is_retryable: false,
                        should_retry_operation,
                    });
                }
                Err(_e) => {
                    #[cfg(feature = "tracing")]
                    warn!(error = %_e, "Transient error during polling, continuing");
                }
            }

            tokio::time::sleep(poll_interval).await;
        }
    }
}

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
///     .build();
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

    /// Build the SmsSolverService.
    pub fn build(self) -> SmsSolverService<P> {
        SmsSolverService::new(self.provider, self.config_builder.build())
    }
}
