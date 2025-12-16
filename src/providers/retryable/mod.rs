//! Retryable provider wrapper.

use super::traits::Provider;
use crate::errors::RetryableError;
use crate::types::{DialCode, FullNumber, SmsCode, TaskId};
use crate::utils::retry::RetryConfig;
use backon::Retryable;
use keshvar::Country;
use std::fmt::Debug;
use std::sync::Arc;
use std::time::Duration;

#[cfg(feature = "tracing")]
use tracing::debug;

/// Callback type for retry notifications.
///
/// This callback is invoked each time a retry is attempted.
/// The callback receives the error that caused the retry and the duration
/// until the next retry attempt.
///
/// # Example
///
/// ```rust,ignore
/// use sms_solvers::SmsRetryableProvider;
///
/// let provider = SmsRetryableProvider::new(base_provider)
///     .with_on_retry(|error, duration| {
///         println!("Retrying after {:?} due to: {}", duration, error);
///     });
/// ```
pub type OnRetryCallback<E> = Arc<dyn Fn(&E, Duration) + Send + Sync>;

/// Wrapper that adds automatic retry logic to any Provider.
///
/// This wrapper implements the same `Provider` trait but adds configurable
/// retry behavior based on the error's `is_retryable()` method.
///
/// # Example
///
/// ```rust,ignore
/// use sms_solvers::{Provider, SmsRetryableProvider, RetryConfig};
/// use sms_solvers::sms_activate::SmsActivateProvider;
/// use std::time::Duration;
///
/// let base_provider = SmsActivateProvider::new("api_key")?;
///
/// // With default retry config
/// let provider = SmsRetryableProvider::new(base_provider.clone());
///
/// // With custom retry config
/// let custom_config = RetryConfig::default()
///     .with_max_retries(5)
///     .with_min_delay(Duration::from_millis(500));
/// let provider = SmsRetryableProvider::with_config(base_provider, custom_config);
///
/// // Now all operations automatically retry on transient errors
/// let (task_id, number) = provider.get_phone_number(country, service).await?;
/// ```
pub struct SmsRetryableProvider<P: Provider> {
    inner: Arc<P>,
    retry_config: RetryConfig,
    on_retry: Option<OnRetryCallback<P::Error>>,
}

impl<P: Provider> Clone for SmsRetryableProvider<P> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
            retry_config: self.retry_config.clone(),
            on_retry: self.on_retry.clone(),
        }
    }
}

impl<P: Provider + Debug> Debug for SmsRetryableProvider<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SmsRetryableProvider")
            .field("inner", &self.inner)
            .field("retry_config", &self.retry_config)
            .field("on_retry", &self.on_retry.as_ref().map(|_| "..."))
            .finish()
    }
}

impl<P: Provider> SmsRetryableProvider<P> {
    /// Wrap a provider with default retry logic.
    pub fn new(inner: P) -> Self {
        Self {
            inner: Arc::new(inner),
            retry_config: RetryConfig::default(),
            on_retry: None,
        }
    }

    /// Wrap a provider with custom retry configuration.
    pub fn with_config(inner: P, retry_config: RetryConfig) -> Self {
        Self {
            inner: Arc::new(inner),
            retry_config,
            on_retry: None,
        }
    }

    /// Set a callback to be invoked on each retry attempt.
    ///
    /// The callback receives the error that caused the retry and the duration
    /// until the next retry attempt.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let provider = SmsRetryableProvider::new(base_provider)
    ///     .with_on_retry(|error, duration| {
    ///         println!("Retrying after {:?} due to: {}", duration, error);
    ///     });
    /// ```
    pub fn with_on_retry<F>(mut self, callback: F) -> Self
    where
        F: Fn(&P::Error, Duration) + Send + Sync + 'static,
    {
        self.on_retry = Some(Arc::new(callback));
        self
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

impl<P: Provider> Provider for SmsRetryableProvider<P>
where
    P::Error: Debug,
{
    type Error = P::Error;
    type Service = P::Service;

    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(
            name = "SmsRetryableProvider::get_phone_number",
            skip_all,
            fields(country = %country.iso_short_name())
        )
    )]
    async fn get_phone_number(
        &self,
        country: Country,
        service: Self::Service,
    ) -> Result<(TaskId, FullNumber), Self::Error> {
        let inner = Arc::clone(&self.inner);
        let on_retry = self.on_retry.clone();
        let country_name = country.iso_short_name().to_string();
        (|| {
            let inner = Arc::clone(&inner);
            let svc = service.clone();
            let c = country.clone();
            async move { inner.get_phone_number(c, svc).await }
        })
        .retry(self.retry_config.build_strategy())
        .when(|err: &Self::Error| err.is_retryable())
        .notify(move |err, duration| {
            // Call user callback if set
            if let Some(ref callback) = on_retry {
                callback(err, duration);
            }

            #[cfg(feature = "tracing")]
            debug!(
                error = ?err,
                country = %country_name,
                retry_after_secs = %duration.as_secs_f64(),
                "Retrying get_phone_number"
            );
        })
        .await
    }

    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(
            name = "SmsRetryableProvider::get_sms_code",
            skip_all,
            fields(task_id = %task_id)
        )
    )]
    async fn get_sms_code(&self, task_id: &TaskId) -> Result<Option<SmsCode>, Self::Error> {
        let inner = Arc::clone(&self.inner);
        let task_id_owned = task_id.clone();
        let task_id_for_notify = task_id.clone();
        let on_retry = self.on_retry.clone();
        (|| {
            let inner = Arc::clone(&inner);
            let task_id = task_id_owned.clone();
            async move { inner.get_sms_code(&task_id).await }
        })
        .retry(self.retry_config.build_strategy())
        .when(|err: &Self::Error| err.is_retryable())
        .notify(move |err, duration| {
            // Call user callback if set
            if let Some(ref callback) = on_retry {
                callback(err, duration);
            }

            #[cfg(feature = "tracing")]
            debug!(
                error = ?err,
                task_id = %task_id_for_notify,
                retry_after_secs = %duration.as_secs_f64(),
                "Retrying get_sms_code"
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

    fn supports_service(&self, service: &Self::Service) -> bool {
        self.inner.supports_service(service)
    }

    fn available_countries(&self, service: &Self::Service) -> Vec<Country> {
        self.inner.available_countries(service)
    }

    fn supported_services(&self) -> Vec<Self::Service> {
        self.inner.supported_services()
    }
}
