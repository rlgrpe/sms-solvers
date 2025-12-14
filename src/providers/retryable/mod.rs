//! Retryable provider wrapper.

use super::traits::Provider;
use crate::errors::RetryableError;
use crate::types::{DialCode, FullNumber, SmsCode, TaskId};
use crate::utils::retry::RetryConfig;
use backon::Retryable;
use isocountry::CountryCode;
use std::fmt::Debug;

#[cfg(feature = "tracing")]
use tracing::debug;

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
#[derive(Debug, Clone)]
pub struct SmsRetryableProvider<P: Provider> {
    inner: P,
    retry_config: RetryConfig,
}

impl<P: Provider> SmsRetryableProvider<P> {
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

impl<P: Provider> Provider for SmsRetryableProvider<P>
where
    P::Error: Debug,
{
    type Error = P::Error;
    type Service = P::Service;

    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(
            name = "retryable_provider.get_phone_number",
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
                "Retrying get_phone_number"
            );
        })
        .await
    }

    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(
            name = "retryable_provider.get_sms_code",
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

    fn available_countries(&self, service: &Self::Service) -> Vec<CountryCode> {
        self.inner.available_countries(service)
    }

    fn supported_services(&self) -> Vec<Self::Service> {
        self.inner.supported_services()
    }
}
