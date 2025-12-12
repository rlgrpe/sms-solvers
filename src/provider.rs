//! Provider trait and retry wrapper for SMS operations.

use crate::errors::RetryableError;
use crate::retry::RetryConfig;
use crate::types::{DialCode, FullNumber, SmsCode, TaskId};
use backon::Retryable;
use isocountry::CountryCode;
use std::error::Error as StdError;
use std::fmt::Debug;
use std::future::Future;

#[cfg(feature = "tracing")]
use tracing::debug;

/// Core trait that all SMS providers must implement.
///
/// This trait defines the essential operations needed from any SMS provider:
/// - Getting a phone number for a specific country and service
/// - Checking if an SMS code has been received
/// - Finishing/completing an activation
/// - Cancelling an activation
///
/// # Type Parameters
///
/// - `Error`: The error type for this provider
/// - `Service`: The service type for phone number requests (e.g., WhatsApp, Instagram)
///
/// # Example
///
/// ```rust,ignore
/// use sms_solvers::{Provider, TaskId, FullNumber, SmsCode};
/// use isocountry::CountryCode;
///
/// #[derive(Clone)]
/// struct MyProvider { /* ... */ }
///
/// #[derive(Clone)]
/// struct MyService(String);
///
/// impl Provider for MyProvider {
///     type Error = MyError;
///     type Service = MyService;
///
///     async fn get_phone_number(&self, country: CountryCode, service: Self::Service) -> Result<(TaskId, FullNumber), Self::Error> {
///         // Get a phone number from the provider for the specified service
///     }
///
///     async fn get_sms_code(&self, task_id: &TaskId) -> Result<Option<SmsCode>, Self::Error> {
///         // Poll for SMS code
///     }
///
///     async fn finish_activation(&self, task_id: &TaskId) -> Result<(), Self::Error> {
///         // Mark activation as complete
///     }
///
///     async fn cancel_activation(&self, task_id: &TaskId) -> Result<(), Self::Error> {
///         // Cancel the activation
///     }
/// }
/// ```
pub trait Provider: Send + Sync + Clone {
    /// Error type returned by provider operations.
    type Error: StdError + RetryableError + Send + Sync + 'static;

    /// Service type for phone number requests.
    /// Each provider can define its own service type (e.g., WhatsApp, Instagram, etc.)
    type Service: Clone + Send + Sync;

    /// Get a phone number for the specified country and service.
    ///
    /// # Arguments
    /// * `country` - ISO country code for the desired phone number
    /// * `service` - The service to get a number for (e.g., WhatsApp verification)
    ///
    /// # Returns
    /// * `task_id` - Unique identifier for this activation
    /// * `full_number` - The full phone number with country code
    fn get_phone_number(
        &self,
        country: CountryCode,
        service: Self::Service,
    ) -> impl Future<Output = Result<(TaskId, FullNumber), Self::Error>> + Send;

    /// Check if SMS code has been received for the given task.
    ///
    /// # Arguments
    /// * `task_id` - The activation identifier from `get_phone_number`
    ///
    /// # Returns
    /// * `Some(SmsCode)` - SMS code if received
    /// * `None` - SMS not yet received, caller should poll again
    fn get_sms_code(
        &self,
        task_id: &TaskId,
    ) -> impl Future<Output = Result<Option<SmsCode>, Self::Error>> + Send;

    /// Mark the activation as successfully completed.
    ///
    /// Call this after successfully using the SMS code.
    fn finish_activation(
        &self,
        task_id: &TaskId,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;

    /// Cancel the activation.
    ///
    /// Call this when:
    /// - SMS timeout occurs
    /// - Permanent error during polling
    /// - No longer need the number
    fn cancel_activation(
        &self,
        task_id: &TaskId,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;

    /// Check if the provider supports the given dial code.
    ///
    /// This method allows providers to implement their own filtering logic,
    /// such as blacklists or specific country support.
    ///
    /// Default implementation returns true, allowing all dial codes.
    fn is_dial_code_supported(&self, dial_code: &DialCode) -> bool {
        let _ = dial_code;
        true
    }
}

/// Wrapper that adds automatic retry logic to any Provider.
///
/// This wrapper implements the same `Provider` trait but adds configurable
/// retry behavior based on the error's `is_retryable()` method.
///
/// # Example
///
/// ```rust,ignore
/// use sms_solvers::{Provider, RetryableProvider, RetryConfig};
/// use sms_solvers::providers::sms_activate::SmsActivateProvider;
/// use std::time::Duration;
///
/// let base_provider = SmsActivateProvider::new("api_key")?;
///
/// // With default retry config
/// let provider = RetryableProvider::new(base_provider.clone());
///
/// // With custom retry config
/// let custom_config = RetryConfig::default()
///     .with_max_retries(5)
///     .with_min_delay(Duration::from_millis(500));
/// let provider = RetryableProvider::with_config(base_provider, custom_config);
///
/// // Now all operations automatically retry on transient errors
/// let (task_id, number) = provider.get_phone_number(country).await?;
/// ```
#[derive(Debug, Clone)]
pub struct RetryableProvider<P: Provider> {
    inner: P,
    retry_config: RetryConfig,
}

impl<P: Provider> RetryableProvider<P> {
    /// Wrap a provider with default retry logic.
    pub fn new(inner: P) -> Self {
        Self {
            inner,
            retry_config: RetryConfig::default(),
        }
    }

    /// Wrap a provider with custom retry configuration.
    pub fn with_config(inner: P, retry_config: RetryConfig) -> Self {
        Self {
            inner,
            retry_config,
        }
    }

    /// Get reference to the inner provider.
    pub fn inner(&self) -> &P {
        &self.inner
    }

    /// Get reference to the retry configuration.
    pub fn retry_config(&self) -> &RetryConfig {
        &self.retry_config
    }
}

impl<P: Provider> Provider for RetryableProvider<P>
where
    P::Error: Debug,
{
    type Error = P::Error;
    type Service = P::Service;

    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(
            name = "RetryableProvider::get_phone_number",
            skip_all,
            fields(country = %country)
        )
    )]
    async fn get_phone_number(
        &self,
        country: CountryCode,
        service: Self::Service,
    ) -> Result<(TaskId, FullNumber), Self::Error> {
        let inner = self.inner.clone();
        (|| {
            let inner = inner.clone();
            let svc = service.clone();
            async move { inner.get_phone_number(country, svc).await }
        })
        .retry(self.retry_config.build_strategy())
        .when(|err: &Self::Error| err.is_retryable())
        .notify(|err, duration| {
            let _ = (err, duration);
            #[cfg(feature = "tracing")]
            debug!(
                error = ?err,
                country = %country,
                retry_after_secs = %duration.as_secs_f64(),
                "Retrying get_phone_number after transient error"
            );
        })
        .await
    }

    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(
            name = "RetryableProvider::get_sms_code",
            skip_all,
            fields(task_id = %task_id)
        )
    )]
    async fn get_sms_code(&self, task_id: &TaskId) -> Result<Option<SmsCode>, Self::Error> {
        let inner = self.inner.clone();
        let task_id = task_id.clone();
        (|| async { inner.get_sms_code(&task_id).await })
            .retry(self.retry_config.build_strategy())
            .when(|err: &Self::Error| err.is_retryable())
            .notify(|err, duration| {
                let _ = (err, duration);
                #[cfg(feature = "tracing")]
                debug!(
                    error = ?err,
                    task_id = %task_id,
                    retry_after_secs = %duration.as_secs_f64(),
                    "Retrying get_sms_code after transient error"
                );
            })
            .await
    }

    async fn finish_activation(&self, task_id: &TaskId) -> Result<(), Self::Error> {
        self.inner.finish_activation(task_id).await
    }

    async fn cancel_activation(&self, task_id: &TaskId) -> Result<(), Self::Error> {
        self.inner.cancel_activation(task_id).await
    }

    fn is_dial_code_supported(&self, dial_code: &DialCode) -> bool {
        self.inner.is_dial_code_supported(dial_code)
    }
}
