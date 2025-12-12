//! SMS verification service with polling and timeout handling.

use crate::errors::RetryableError;
use crate::provider::Provider;
use crate::types::{DialCode, Number, SmsCode, SmsTaskResult, TaskId};
use isocountry::CountryCode;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::error::Error as StdError;
use std::fmt::{Debug, Display};
use std::future::Future;
use std::time::{Duration, Instant};
use thiserror::Error;

#[cfg(feature = "tracing")]
use tracing::{error, info, warn};

/// Service-level errors that wrap provider errors.
#[derive(Debug, Error)]
pub enum ServiceError<E: StdError + 'static> {
    /// Error from the underlying provider.
    #[error("SMS provider error: {source}")]
    Provider {
        #[source]
        source: E,
        /// Whether the same task can be retried.
        is_retryable: bool,
        /// Whether a fresh operation might succeed.
        should_retry_operation: bool,
    },

    /// No phone number available for the requested country.
    #[error("No phone numbers available for country {country}")]
    NoNumbersAvailable { country: CountryCode },

    /// Invalid dial code for the country.
    #[error("Invalid dial code '{dial_code}' for country {country}")]
    InvalidDialCode {
        dial_code: String,
        country: CountryCode,
    },

    /// Failed to parse the phone number.
    #[error("Failed to parse phone number '{full_number}': {message}")]
    NumberParse {
        full_number: String,
        message: String,
    },

    /// Timeout waiting for SMS code.
    #[error(
        "Timeout waiting for SMS code after {:.1}s; Task id: {task_id}",
        timeout.as_secs_f64()
    )]
    SmsTimeout { timeout: Duration, task_id: TaskId },
}

impl<E: StdError + 'static> RetryableError for ServiceError<E> {
    fn is_retryable(&self) -> bool {
        match self {
            ServiceError::Provider { is_retryable, .. } => *is_retryable,
            // Can't retry the same task after timeout
            ServiceError::SmsTimeout { .. } => false,
            // These are configuration/logic errors
            ServiceError::NoNumbersAvailable { .. }
            | ServiceError::InvalidDialCode { .. }
            | ServiceError::NumberParse { .. } => false,
        }
    }

    fn should_retry_operation(&self) -> bool {
        match self {
            ServiceError::Provider {
                should_retry_operation,
                ..
            } => *should_retry_operation,
            // A fresh task attempt might succeed after timeout
            ServiceError::SmsTimeout { .. } => true,
            // No numbers might become available
            ServiceError::NoNumbersAvailable { .. } => true,
            // Configuration errors won't fix themselves
            ServiceError::InvalidDialCode { .. } | ServiceError::NumberParse { .. } => false,
        }
    }
}

/// Trait for SMS verification service implementations.
///
/// This trait abstracts the service interface, allowing different
/// service implementations to be used interchangeably.
pub trait SmsServiceTrait: Send + Sync {
    /// The error type for this service.
    type Error: StdError + RetryableError;

    /// The service type for phone number requests.
    type Service: Clone + Send + Sync;

    /// Get a phone number for the specified country and service.
    fn get_number(
        &self,
        country: CountryCode,
        service: Self::Service,
    ) -> impl Future<Output = Result<SmsTaskResult, Self::Error>> + Send;

    /// Wait for an SMS code to be received.
    fn wait_for_sms_code(
        &self,
        task_id: &TaskId,
    ) -> impl Future<Output = Result<SmsCode, Self::Error>> + Send;
}

/// Configuration for SMS Service.
#[derive(Debug, Clone)]
pub struct SmsServiceConfig {
    /// Maximum time to wait for SMS code before timing out.
    pub wait_sms_code_timeout: Duration,
    /// Interval between polling attempts when waiting for SMS.
    pub poll_interval: Duration,
}

impl Default for SmsServiceConfig {
    fn default() -> Self {
        Self {
            wait_sms_code_timeout: Duration::from_secs(120),
            poll_interval: Duration::from_secs(3),
        }
    }
}

impl SmsServiceConfig {
    /// Create a new builder for SmsServiceConfig.
    pub fn builder() -> SmsServiceConfigBuilder {
        SmsServiceConfigBuilder::default()
    }

    /// Create a new config with a custom timeout.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.wait_sms_code_timeout = timeout;
        self
    }

    /// Create a new config with a custom poll interval.
    pub fn with_poll_interval(mut self, interval: Duration) -> Self {
        self.poll_interval = interval;
        self
    }
}

/// Builder for SmsServiceConfig.
///
/// Provides a fluent API for configuring the SMS service.
///
/// # Example
///
/// ```rust
/// use sms_solvers::SmsServiceConfig;
/// use std::time::Duration;
///
/// let config = SmsServiceConfig::builder()
///     .timeout(Duration::from_secs(180))
///     .poll_interval(Duration::from_secs(5))
///     .build();
///
/// assert_eq!(config.wait_sms_code_timeout, Duration::from_secs(180));
/// assert_eq!(config.poll_interval, Duration::from_secs(5));
/// ```
#[derive(Debug, Clone)]
pub struct SmsServiceConfigBuilder {
    timeout: Duration,
    poll_interval: Duration,
}

impl Default for SmsServiceConfigBuilder {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(120),
            poll_interval: Duration::from_secs(3),
        }
    }
}

impl SmsServiceConfigBuilder {
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

    /// Build the SmsServiceConfig.
    pub fn build(self) -> SmsServiceConfig {
        SmsServiceConfig {
            wait_sms_code_timeout: self.timeout,
            poll_interval: self.poll_interval,
        }
    }
}

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
/// use sms_solvers::{SmsService, SmsServiceConfig, SmsServiceTrait};
/// use sms_solvers::providers::sms_activate::{SmsActivateClient, SmsActivateProvider, Service};
/// use std::time::Duration;
/// use isocountry::CountryCode;
///
/// // Create provider and service
/// let client = SmsActivateClient::with_api_key("api_key")?;
/// let provider = SmsActivateProvider::new(client);
/// let config = SmsServiceConfig {
///     wait_sms_code_timeout: Duration::from_secs(120),
///     poll_interval: Duration::from_secs(3),
/// };
/// let service = SmsService::new(provider, config);
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
pub struct SmsService<P: Provider> {
    provider: P,
    config: SmsServiceConfig,
}

impl<P: Provider> SmsService<P>
where
    P::Error: Debug + Display + RetryableError,
{
    /// Create a new SMS service with a custom provider and configuration.
    pub fn new(provider: P, config: SmsServiceConfig) -> Self {
        Self { provider, config }
    }

    /// Create a new SMS service with default configuration.
    pub fn with_provider(provider: P) -> Self {
        Self::new(provider, SmsServiceConfig::default())
    }

    /// Create a new builder for SmsService.
    pub fn builder(provider: P) -> SmsServiceBuilder<P> {
        SmsServiceBuilder::new(provider)
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
    pub fn config(&self) -> &SmsServiceConfig {
        &self.config
    }

    /// Get mutable reference to the service configuration.
    pub fn config_mut(&mut self) -> &mut SmsServiceConfig {
        &mut self.config
    }

    /// Update the service configuration.
    pub fn set_config(&mut self, config: SmsServiceConfig) {
        self.config = config;
    }
}

/// Builder for SmsService.
///
/// Provides a fluent API for constructing an SMS service with a provider
/// and custom configuration.
///
/// # Example
///
/// ```rust,ignore
/// use sms_solvers::{SmsService, Provider};
/// use std::time::Duration;
///
/// let service = SmsService::builder(provider)
///     .timeout(Duration::from_secs(180))
///     .poll_interval(Duration::from_secs(5))
///     .build();
/// ```
#[derive(Debug, Clone)]
pub struct SmsServiceBuilder<P: Provider> {
    provider: P,
    config_builder: SmsServiceConfigBuilder,
}

impl<P: Provider> SmsServiceBuilder<P>
where
    P::Error: Debug + Display + RetryableError,
{
    /// Create a new builder with the given provider.
    pub fn new(provider: P) -> Self {
        Self {
            provider,
            config_builder: SmsServiceConfigBuilder::default(),
        }
    }

    /// Set the timeout for waiting for SMS codes.
    ///
    /// Default: 120 seconds
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.config_builder = self.config_builder.timeout(timeout);
        self
    }

    /// Set the polling interval when waiting for SMS codes.
    ///
    /// Default: 3 seconds
    pub fn poll_interval(mut self, interval: Duration) -> Self {
        self.config_builder = self.config_builder.poll_interval(interval);
        self
    }

    /// Set the full configuration.
    pub fn config(mut self, config: SmsServiceConfig) -> Self {
        self.config_builder = SmsServiceConfigBuilder {
            timeout: config.wait_sms_code_timeout,
            poll_interval: config.poll_interval,
        };
        self
    }

    /// Build the SmsService.
    pub fn build(self) -> SmsService<P> {
        SmsService::new(self.provider, self.config_builder.build())
    }
}

impl<P: Provider> SmsServiceTrait for SmsService<P>
where
    P::Error: Debug + Display + RetryableError + 'static,
{
    type Error = ServiceError<P::Error>;
    type Service = P::Service;

    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(
            name = "SmsService::get_number",
            skip_all,
            fields(country = %country)
        )
    )]
    async fn get_number(
        &self,
        country: CountryCode,
        service: Self::Service,
    ) -> Result<SmsTaskResult, Self::Error> {
        // Get phone number from provider
        let (task_id, full_number) = self
            .provider
            .get_phone_number(country, service)
            .await
            .map_err(|e| {
                let is_retryable = e.is_retryable();
                let should_retry_operation = e.should_retry_operation();
                ServiceError::Provider {
                    source: e,
                    is_retryable,
                    should_retry_operation,
                }
            })?;

        // Get dial code for the country
        let dial_code =
            country_to_dial_code(country).ok_or_else(|| ServiceError::InvalidDialCode {
                dial_code: "unknown".to_string(),
                country,
            })?;

        // Parse the number
        let number = Number::from_full_number(&full_number, &dial_code).map_err(|e| {
            ServiceError::NumberParse {
                full_number: full_number.to_string(),
                message: e.to_string(),
            }
        })?;

        #[cfg(feature = "tracing")]
        info!(
            task_id = %task_id,
            dial_code = %dial_code,
            country = %country.alpha2(),
            "Successfully acquired phone number"
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
            name = "SmsService::wait_for_sms_code",
            skip_all,
            fields(task_id = %task_id)
        )
    )]
    async fn wait_for_sms_code(&self, task_id: &TaskId) -> Result<SmsCode, Self::Error> {
        let timeout = self.config.wait_sms_code_timeout;
        let poll_interval = self.config.poll_interval;
        let start = Instant::now();

        loop {
            if start.elapsed() >= timeout {
                #[cfg(feature = "tracing")]
                warn!(
                    timeout_secs = %timeout.as_secs_f64(),
                    "SMS code timeout - cancelling activation"
                );

                // Try to cancel the activation
                if let Err(_e) = self.provider.cancel_activation(task_id).await {
                    #[cfg(feature = "tracing")]
                    warn!(
                        error = %_e,
                        "Failed to cancel activation after timeout"
                    );
                }

                return Err(ServiceError::SmsTimeout {
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
                        "SMS code received successfully"
                    );
                    return Ok(code);
                }
                Ok(None) => {
                    // SMS not yet received, continue polling
                }
                Err(e) if !e.is_retryable() => {
                    // Permanent error - cancel and return
                    let should_retry_operation = e.should_retry_operation();

                    #[cfg(feature = "tracing")]
                    error!(
                        error = %e,
                        "Permanent error while polling for SMS code"
                    );

                    // Try to cancel the activation
                    if let Err(_cancel_err) = self.provider.cancel_activation(task_id).await {
                        #[cfg(feature = "tracing")]
                        warn!(
                            error = %_cancel_err,
                            "Failed to cancel activation after permanent error"
                        );
                    }

                    return Err(ServiceError::Provider {
                        source: e,
                        is_retryable: false,
                        should_retry_operation,
                    });
                }
                Err(_e) => {
                    // Transient error - log and continue
                    #[cfg(feature = "tracing")]
                    warn!(
                        error = %_e,
                        "Transient error while polling for SMS code"
                    );
                }
            }

            tokio::time::sleep(poll_interval).await;
        }
    }
}

// =============================================================================
// Dial Code Mapping from JSON
// =============================================================================

/// Raw JSON entry for country dial code data.
#[derive(Debug, serde::Deserialize)]
struct CountryDialCodeEntry {
    #[allow(dead_code)]
    name: String,
    #[allow(dead_code)]
    flag: String,
    code: String,
    dial_code: String,
}

/// Dial codes JSON embedded at compile time.
static DIAL_CODES_JSON: &str = include_str!("../assets/countries_with_dial_code.json");

/// Mapping from ISO alpha-2 code to dial code string.
/// Built from the countries_with_dial_code.json file at startup.
static ALPHA2_TO_DIAL_CODE: Lazy<HashMap<String, String>> = Lazy::new(|| {
    let entries: Vec<CountryDialCodeEntry> =
        serde_json::from_str(DIAL_CODES_JSON).expect("countries_with_dial_code.json is invalid");

    let mut map = HashMap::with_capacity(entries.len());
    for entry in entries {
        map.insert(entry.code.to_uppercase(), entry.dial_code);
    }
    map
});

/// Convert a country code to its dial code.
///
/// Reads dial codes from assets/countries_with_dial_code.json file.
/// The mapping uses ISO alpha-2 country codes.
fn country_to_dial_code(country: CountryCode) -> Option<DialCode> {
    let alpha2 = country.alpha2();
    let dial_code_str = ALPHA2_TO_DIAL_CODE.get(alpha2)?;
    DialCode::new(dial_code_str).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_country_to_dial_code() {
        assert_eq!(
            country_to_dial_code(CountryCode::USA).map(|dc| dc.to_string()),
            Some("1".to_string())
        );
        assert_eq!(
            country_to_dial_code(CountryCode::UKR).map(|dc| dc.to_string()),
            Some("380".to_string())
        );
        assert_eq!(
            country_to_dial_code(CountryCode::GBR).map(|dc| dc.to_string()),
            Some("44".to_string())
        );
        assert_eq!(
            country_to_dial_code(CountryCode::TUR).map(|dc| dc.to_string()),
            Some("90".to_string())
        );
    }

    #[test]
    fn test_service_config_default() {
        let config = SmsServiceConfig::default();
        assert_eq!(config.wait_sms_code_timeout, Duration::from_secs(120));
        assert_eq!(config.poll_interval, Duration::from_secs(3));
    }

    #[test]
    fn test_config_builder() {
        let config = SmsServiceConfig::builder()
            .timeout(Duration::from_secs(180))
            .poll_interval(Duration::from_secs(5))
            .build();

        assert_eq!(config.wait_sms_code_timeout, Duration::from_secs(180));
        assert_eq!(config.poll_interval, Duration::from_secs(5));
    }

    #[test]
    fn test_config_builder_default() {
        let config = SmsServiceConfigBuilder::new().build();
        assert_eq!(config.wait_sms_code_timeout, Duration::from_secs(120));
        assert_eq!(config.poll_interval, Duration::from_secs(3));
    }

    #[test]
    fn test_config_with_methods() {
        let config = SmsServiceConfig::default()
            .with_timeout(Duration::from_secs(60))
            .with_poll_interval(Duration::from_secs(1));

        assert_eq!(config.wait_sms_code_timeout, Duration::from_secs(60));
        assert_eq!(config.poll_interval, Duration::from_secs(1));
    }
}
