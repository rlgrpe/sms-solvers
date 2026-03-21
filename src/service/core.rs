//! Core SmsSolverService implementation.

use super::activation::ActivationHandle;
use super::builder::SmsSolverServiceBuilder;
use super::config::SmsSolverServiceConfig;
use super::error::SmsSolverServiceError;
use super::traits::SmsSolverServiceTrait;
use crate::errors::RetryableError;
use crate::providers::traits::Provider;
use crate::types::{Number, SmsCode, SmsTaskResult, TaskId};
use keshvar::Country;
use std::error::Error as StdError;
use std::fmt::{Debug, Display};
use std::time::Instant;
use tokio_util::sync::CancellationToken;

#[cfg(feature = "tracing")]
use crate::utils::error_chain::ErrorChain;
#[cfg(feature = "tracing")]
use crate::utils::span_status::{set_span_error, set_span_ok};
#[cfg(feature = "tracing")]
use tracing::{error, info, warn};

#[cfg(feature = "metrics")]
use super::telemetry::ServiceMetrics;
#[cfg(feature = "metrics")]
use opentelemetry::KeyValue;

use crate::DialCode;

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
/// - `P`: The provider implementation (e.g., `HeroSmsProvider`)
///
/// # Example
///
/// ```rust,ignore
/// use sms_solvers::{SmsSolverService, SmsSolverServiceConfig, SmsSolverServiceTrait, Alpha2};
/// use sms_solvers::hero_sms::{HeroSms, HeroSmsProvider, Service};
///
/// let client = HeroSms::with_api_key("api_key")?;
/// let provider = HeroSmsProvider::new(client);
/// let service = SmsSolverService::try_new(provider, SmsSolverServiceConfig::default())?;
///
/// let result = service.get_number(Alpha2::US.to_country(), Service::Whatsapp).await?;
/// let code = service.wait_for_sms_code(&result.task_id).await?;
/// ```
#[derive(Debug, Clone)]
pub struct SmsSolverService<P: Provider> {
    pub(crate) provider: P,
    pub(crate) config: SmsSolverServiceConfig,
}

impl<P: Provider> SmsSolverService<P>
where
    P::Error: Debug + Display + RetryableError,
{
    /// Create a new SMS service with a validated configuration.
    ///
    /// Returns an error if the configuration is invalid.
    pub fn try_new(
        provider: P,
        config: SmsSolverServiceConfig,
    ) -> Result<Self, super::config::ConfigError> {
        config.validate()?;
        Ok(Self { provider, config })
    }

    /// Create a new SMS service without configuration validation.
    ///
    /// Prefer [`try_new`](Self::try_new) for validated construction.
    pub fn new_unchecked(provider: P, config: SmsSolverServiceConfig) -> Self {
        Self { provider, config }
    }

    /// Create a new SMS service with a custom provider and configuration.
    ///
    /// This does not validate the configuration. Use [`try_new`](Self::try_new)
    /// for validated construction or [`new_unchecked`](Self::new_unchecked)
    /// to skip validation explicitly.
    #[deprecated(
        note = "Use `try_new` for validated construction or `new_unchecked` to skip validation"
    )]
    pub fn new(provider: P, config: SmsSolverServiceConfig) -> Self {
        Self { provider, config }
    }

    /// Create a new SMS service with default configuration.
    pub fn with_provider(provider: P) -> Self {
        Self {
            provider,
            config: SmsSolverServiceConfig::default(),
        }
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

    /// Update the service configuration.
    ///
    /// Returns an error if the new configuration is invalid.
    pub fn set_config(
        &mut self,
        config: SmsSolverServiceConfig,
    ) -> Result<(), super::config::ConfigError> {
        config.validate()?;
        self.config = config;
        Ok(())
    }

    /// Filter dial codes to only include those supported by the provider.
    pub fn filter_supported_dial_codes(&self, dial_codes: Vec<DialCode>) -> Vec<DialCode> {
        dial_codes
            .into_iter()
            .filter(|dc| self.provider.is_dial_code_supported(dc))
            .collect()
    }

    /// Select a random dial code from the provided list, filtering out unsupported ones.
    ///
    /// Returns `SmsSolverServiceError::NoAvailableDialCodes` if no supported
    /// dial codes remain after filtering.
    #[cfg(feature = "random")]
    pub fn select_random_dial_code(
        &self,
        dial_codes: Vec<DialCode>,
    ) -> Result<DialCode, SmsSolverServiceError> {
        use rand::seq::SliceRandom;

        let supported = self.filter_supported_dial_codes(dial_codes);

        if supported.is_empty() {
            return Err(SmsSolverServiceError::NoAvailableDialCodes);
        }

        supported
            .choose(&mut rand::thread_rng())
            .cloned()
            .ok_or(SmsSolverServiceError::NoAvailableDialCodes)
    }

    /// Acquire a phone number and return an [`ActivationHandle`] for managing the lifecycle.
    ///
    /// This is the preferred entry point for new code. The handle bundles
    /// the activation data with convenience methods for waiting, finishing,
    /// and cancelling.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let activation = service.activate(country, Service::Whatsapp).await?;
    /// let code = activation.wait_for_code().await?;
    /// activation.finish().await?;
    /// ```
    pub async fn activate(
        &self,
        country: Country,
        service: P::Service,
    ) -> Result<ActivationHandle<P>, SmsSolverServiceError> {
        let result = self.get_number(country, service).await?;
        Ok(ActivationHandle::new(self.clone(), result))
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
            name = "get_number",
            target = "sms.service",
            skip_all,
            fields(country = %country.iso_short_name(), outcome = tracing::field::Empty)
        )
    )]
    async fn get_number(
        &self,
        country: Country,
        service: Self::Service,
    ) -> Result<SmsTaskResult, Self::Error> {
        #[cfg(feature = "metrics")]
        let country_alpha2 = country.alpha2().to_string();

        #[cfg(feature = "metrics")]
        ServiceMetrics::global()
            .numbers_requested
            .add(1, &[KeyValue::new("country", country_alpha2.clone())]);

        let (task_id, full_number, api_dial_code) = self
            .provider
            .get_phone_number(country.clone(), service)
            .await
            .map_err(|e| {
                #[cfg(feature = "metrics")]
                ServiceMetrics::global().errors.add(
                    1,
                    &[
                        KeyValue::new("country", country_alpha2.clone()),
                        KeyValue::new("operation", "get_number"),
                    ],
                );
                let is_retryable = e.is_retryable();
                let should_retry_operation = e.should_retry_operation();
                SmsSolverServiceError::Provider {
                    source: Box::new(e) as Box<dyn StdError + Send + Sync>,
                    is_retryable,
                    should_retry_operation,
                }
            })
            .inspect_err(|e| {
                #[cfg(feature = "tracing")]
                {
                    tracing::Span::current().record("outcome", "error");
                    set_span_error(e);
                }
            })?;

        let dial_code = api_dial_code.unwrap_or_else(|| DialCode::from(&country));

        if !self.provider.is_dial_code_supported(&dial_code) {
            #[cfg(feature = "tracing")]
            warn!(
                task_id = %task_id,
                dial_code = %dial_code,
                "Dial code is blacklisted, cancelling activation"
            );

            if let Err(e) = self.provider.cancel_activation(&task_id).await {
                #[cfg(feature = "tracing")]
                warn!(error = %ErrorChain(&e), "Failed to cancel activation for blacklisted number");

                let err = SmsSolverServiceError::CancelFailed {
                    task_id,
                    message: e.to_string(),
                };
                #[cfg(feature = "tracing")]
                {
                    tracing::Span::current().record("outcome", "error");
                    set_span_error(&err);
                }
                return Err(err);
            }

            let err = SmsSolverServiceError::DialCodeBlacklisted { dial_code, task_id };
            #[cfg(feature = "tracing")]
            {
                tracing::Span::current().record("outcome", "error");
                set_span_error(&err);
            }
            return Err(err);
        }

        let number = Number::from_full_number(&full_number, &dial_code).map_err(|e| {
            SmsSolverServiceError::NumberParse {
                full_number: full_number.to_string(),
                message: e.to_string(),
            }
        });
        let number = number.inspect_err(|e| {
            #[cfg(feature = "tracing")]
            {
                tracing::Span::current().record("outcome", "error");
                set_span_error(e);
            }
        })?;

        #[cfg(feature = "tracing")]
        {
            tracing::Span::current().record("outcome", "success");
            set_span_ok();
            info!(
                task_id = %task_id,
                dial_code = %dial_code,
                country = %country.iso_short_name(),
                "Phone number acquired"
            );
        }

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
            name = "wait_for_sms_code",
            target = "sms.service",
            skip_all,
            fields(task_id = %task_id)
        )
    )]
    async fn wait_for_sms_code(&self, task_id: &TaskId) -> Result<SmsCode, Self::Error> {
        self.wait_for_sms_code_cancellable(task_id, CancellationToken::new())
            .await
    }

    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(
            name = "wait_for_sms_code_cancellable",
            target = "sms.service",
            skip_all,
            fields(task_id = %task_id, outcome = tracing::field::Empty)
        )
    )]
    async fn wait_for_sms_code_cancellable(
        &self,
        task_id: &TaskId,
        cancel_token: CancellationToken,
    ) -> Result<SmsCode, Self::Error> {
        let timeout = self.config.timeout;
        let poll_interval = self.config.poll_interval;
        let start = Instant::now();
        let mut poll_count: u32 = 0;

        loop {
            // Check for cancellation
            if cancel_token.is_cancelled() {
                let elapsed = start.elapsed();

                #[cfg(feature = "tracing")]
                info!(
                    elapsed_secs = %elapsed.as_secs_f64(),
                    poll_count = %poll_count,
                    "Cancellation requested, cancelling activation"
                );

                #[cfg(feature = "metrics")]
                {
                    ServiceMetrics::global().cancellations.add(1, &[]);
                    ServiceMetrics::global().sms_wait_time.record(
                        elapsed.as_secs_f64(),
                        &[KeyValue::new("outcome", "cancelled")],
                    );
                    ServiceMetrics::global()
                        .poll_counts
                        .record(poll_count as u64, &[KeyValue::new("outcome", "cancelled")]);
                }

                if let Err(e) = self.provider.cancel_activation(task_id).await {
                    #[cfg(feature = "tracing")]
                    warn!(error = %ErrorChain(&e), "Failed to cancel activation after cancellation request");

                    let err = SmsSolverServiceError::CancelFailed {
                        task_id: task_id.clone(),
                        message: e.to_string(),
                    };
                    #[cfg(feature = "tracing")]
                    {
                        tracing::Span::current().record("outcome", "error");
                        set_span_error(&err);
                    }
                    return Err(err);
                }

                let err = SmsSolverServiceError::Cancelled {
                    elapsed,
                    poll_count,
                    task_id: task_id.clone(),
                };
                #[cfg(feature = "tracing")]
                {
                    tracing::Span::current().record("outcome", "cancelled");
                    set_span_error(&err);
                }
                return Err(err);
            }

            // Check for timeout
            let elapsed = start.elapsed();
            if elapsed >= timeout {
                #[cfg(feature = "tracing")]
                warn!(
                    timeout_secs = %timeout.as_secs_f64(),
                    elapsed_secs = %elapsed.as_secs_f64(),
                    poll_count = %poll_count,
                    "Timeout reached, cancelling activation"
                );

                #[cfg(feature = "metrics")]
                {
                    ServiceMetrics::global().timeouts.add(1, &[]);
                    ServiceMetrics::global().sms_wait_time.record(
                        elapsed.as_secs_f64(),
                        &[KeyValue::new("outcome", "timeout")],
                    );
                    ServiceMetrics::global()
                        .poll_counts
                        .record(poll_count as u64, &[KeyValue::new("outcome", "timeout")]);
                }

                if let Err(e) = self.provider.cancel_activation(task_id).await {
                    #[cfg(feature = "tracing")]
                    warn!(error = %ErrorChain(&e), "Failed to cancel activation after timeout");

                    let err = SmsSolverServiceError::CancelFailed {
                        task_id: task_id.clone(),
                        message: e.to_string(),
                    };
                    #[cfg(feature = "tracing")]
                    {
                        tracing::Span::current().record("outcome", "error");
                        set_span_error(&err);
                    }
                    return Err(err);
                }

                let err = SmsSolverServiceError::SmsTimeout {
                    timeout,
                    elapsed,
                    poll_count,
                    task_id: task_id.clone(),
                };
                #[cfg(feature = "tracing")]
                {
                    tracing::Span::current().record("outcome", "timeout");
                    set_span_error(&err);
                }
                return Err(err);
            }

            poll_count += 1;

            match self.provider.get_sms_code(task_id).await {
                Ok(Some(code)) => {
                    let elapsed = start.elapsed();

                    #[cfg(feature = "tracing")]
                    {
                        tracing::Span::current().record("outcome", "success");
                        set_span_ok();
                        info!(
                            code_len = code.as_str().len(),
                            elapsed_secs = %elapsed.as_secs_f64(),
                            poll_count = %poll_count,
                            "SMS code received"
                        );
                    }

                    #[cfg(feature = "metrics")]
                    {
                        ServiceMetrics::global().sms_codes_received.add(1, &[]);
                        ServiceMetrics::global().sms_wait_time.record(
                            elapsed.as_secs_f64(),
                            &[KeyValue::new("outcome", "success")],
                        );
                        ServiceMetrics::global()
                            .poll_counts
                            .record(poll_count as u64, &[KeyValue::new("outcome", "success")]);
                    }

                    return Ok(code);
                }
                Ok(None) => {
                    // SMS not yet received, continue polling
                }
                Err(e) if !e.is_retryable() => {
                    let should_retry_operation = e.should_retry_operation();
                    let elapsed = start.elapsed();

                    #[cfg(feature = "tracing")]
                    error!(
                        error = %ErrorChain(&e),
                        elapsed_secs = %elapsed.as_secs_f64(),
                        poll_count = %poll_count,
                        "Permanent error during polling"
                    );

                    #[cfg(feature = "metrics")]
                    {
                        ServiceMetrics::global()
                            .errors
                            .add(1, &[KeyValue::new("operation", "wait_for_sms_code")]);
                        ServiceMetrics::global()
                            .sms_wait_time
                            .record(elapsed.as_secs_f64(), &[KeyValue::new("outcome", "error")]);
                        ServiceMetrics::global()
                            .poll_counts
                            .record(poll_count as u64, &[KeyValue::new("outcome", "error")]);
                    }

                    if let Err(cancel_err) = self.provider.cancel_activation(task_id).await {
                        #[cfg(feature = "tracing")]
                        warn!(error = %ErrorChain(&cancel_err), "Failed to cancel activation after error");

                        let err = SmsSolverServiceError::CancelFailed {
                            task_id: task_id.clone(),
                            message: format!(
                                "original error: {e}; cancel also failed: {cancel_err}"
                            ),
                        };
                        #[cfg(feature = "tracing")]
                        {
                            tracing::Span::current().record("outcome", "error");
                            set_span_error(&err);
                        }
                        return Err(err);
                    }

                    let err = SmsSolverServiceError::Provider {
                        source: Box::new(e) as Box<dyn StdError + Send + Sync>,
                        is_retryable: false,
                        should_retry_operation,
                    };
                    #[cfg(feature = "tracing")]
                    {
                        tracing::Span::current().record("outcome", "error");
                        set_span_error(&err);
                    }
                    return Err(err);
                }
                Err(_e) => {
                    #[cfg(feature = "tracing")]
                    warn!(error = %ErrorChain(&_e), poll_count = %poll_count, "Transient error during polling, continuing");
                }
            }

            tokio::time::sleep(poll_interval).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::RetryableError;
    use crate::types::FullNumber;
    use keshvar::Alpha2;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::time::Duration;
    use thiserror::Error;

    // Mock provider for testing
    #[derive(Clone)]
    #[allow(clippy::type_complexity)]
    struct MockProvider {
        get_number_result: Arc<
            std::sync::Mutex<Option<Result<(TaskId, FullNumber, Option<DialCode>), MockError>>>,
        >,
        sms_code_results: Arc<std::sync::Mutex<Vec<Result<Option<SmsCode>, MockError>>>>,
        cancel_result: Arc<std::sync::Mutex<Option<Result<(), MockError>>>>,
        poll_count: Arc<AtomicU32>,
    }

    #[derive(Debug, Clone, Error)]
    #[allow(dead_code)]
    enum MockError {
        #[error("Mock error: {0}")]
        Generic(String),
        #[error("Transient error")]
        Transient,
    }

    impl RetryableError for MockError {
        fn is_retryable(&self) -> bool {
            matches!(self, MockError::Transient)
        }
    }

    #[derive(Clone)]
    struct MockService;

    impl MockProvider {
        fn new() -> Self {
            Self {
                get_number_result: Arc::new(std::sync::Mutex::new(None)),
                sms_code_results: Arc::new(std::sync::Mutex::new(Vec::new())),
                cancel_result: Arc::new(std::sync::Mutex::new(None)),
                poll_count: Arc::new(AtomicU32::new(0)),
            }
        }

        fn with_number(self, task_id: &str, number: &str) -> Self {
            *self.get_number_result.lock().unwrap() =
                Some(Ok((TaskId::new(task_id), FullNumber::new(number), None)));
            self
        }

        fn with_sms_after_polls(self, polls: u32, code: &str) -> Self {
            {
                let mut results = self.sms_code_results.lock().unwrap();
                for _ in 0..polls {
                    results.push(Ok(None));
                }
                results.push(Ok(Some(SmsCode::new(code))));
            }
            self
        }

        fn with_cancel_success(self) -> Self {
            *self.cancel_result.lock().unwrap() = Some(Ok(()));
            self
        }

        fn with_cancel_error(self, msg: &str) -> Self {
            *self.cancel_result.lock().unwrap() = Some(Err(MockError::Generic(msg.to_string())));
            self
        }
    }

    impl Provider for MockProvider {
        type Error = MockError;
        type Service = MockService;

        async fn get_phone_number(
            &self,
            _country: Country,
            _service: Self::Service,
        ) -> Result<(TaskId, FullNumber, Option<DialCode>), Self::Error> {
            self.get_number_result
                .lock()
                .unwrap()
                .clone()
                .unwrap_or(Err(MockError::Generic("Not configured".to_string())))
        }

        async fn get_sms_code(&self, _task_id: &TaskId) -> Result<Option<SmsCode>, Self::Error> {
            let idx = self.poll_count.fetch_add(1, Ordering::SeqCst) as usize;
            let results = self.sms_code_results.lock().unwrap();
            results.get(idx).cloned().unwrap_or(Ok(None))
        }

        async fn finish_activation(&self, _task_id: &TaskId) -> Result<(), Self::Error> {
            Ok(())
        }

        async fn cancel_activation(&self, _task_id: &TaskId) -> Result<(), Self::Error> {
            self.cancel_result.lock().unwrap().clone().unwrap_or(Ok(()))
        }
    }

    #[tokio::test]
    async fn test_wait_for_sms_code_success() {
        let provider = MockProvider::new()
            .with_number("task123", "380501234567")
            .with_sms_after_polls(2, "123456");

        let config = SmsSolverServiceConfig::builder()
            .timeout(Duration::from_secs(60))
            .poll_interval(Duration::from_millis(10))
            .build();

        let service = SmsSolverService::new_unchecked(provider.clone(), config);

        let result = service
            .get_number(Alpha2::UA.to_country(), MockService)
            .await
            .unwrap();
        assert_eq!(result.task_id.as_ref(), "task123");

        let code = service.wait_for_sms_code(&result.task_id).await.unwrap();
        assert_eq!(code.as_str(), "123456");

        // Should have polled 3 times (2 None + 1 Some)
        assert_eq!(provider.poll_count.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_wait_for_sms_code_timeout() {
        let provider = MockProvider::new()
            .with_number("task123", "380501234567")
            .with_cancel_success();

        // Very short timeout, SMS never arrives
        let config = SmsSolverServiceConfig::builder()
            .timeout(Duration::from_millis(50))
            .poll_interval(Duration::from_millis(10))
            .build();

        let service = SmsSolverService::new_unchecked(provider, config);

        let result = service
            .get_number(Alpha2::UA.to_country(), MockService)
            .await
            .unwrap();

        let err = service
            .wait_for_sms_code(&result.task_id)
            .await
            .unwrap_err();

        match err {
            SmsSolverServiceError::SmsTimeout {
                timeout,
                poll_count,
                task_id,
                ..
            } => {
                assert_eq!(timeout, Duration::from_millis(50));
                assert!(poll_count > 0);
                assert_eq!(task_id.as_ref(), "task123");
            }
            _ => panic!("Expected SmsTimeout error, got {:?}", err),
        }
    }

    #[tokio::test]
    async fn test_wait_for_sms_code_cancellation() {
        let provider = MockProvider::new()
            .with_number("task123", "380501234567")
            .with_cancel_success();

        let config = SmsSolverServiceConfig::builder()
            .timeout(Duration::from_secs(60))
            .poll_interval(Duration::from_millis(10))
            .build();

        let service = SmsSolverService::new_unchecked(provider, config);

        let result = service
            .get_number(Alpha2::UA.to_country(), MockService)
            .await
            .unwrap();

        let cancel_token = CancellationToken::new();
        let token_clone = cancel_token.clone();

        // Cancel immediately
        token_clone.cancel();

        let err = service
            .wait_for_sms_code_cancellable(&result.task_id, cancel_token)
            .await
            .unwrap_err();

        match err {
            SmsSolverServiceError::Cancelled {
                poll_count,
                task_id,
                ..
            } => {
                assert_eq!(poll_count, 0); // Cancelled before any polls
                assert_eq!(task_id.as_ref(), "task123");
            }
            _ => panic!("Expected Cancelled error, got {:?}", err),
        }
    }

    #[tokio::test]
    async fn test_cancel_failure_on_timeout() {
        let provider = MockProvider::new()
            .with_number("task123", "380501234567")
            .with_cancel_error("Cancel failed");

        let config = SmsSolverServiceConfig::builder()
            .timeout(Duration::from_millis(50))
            .poll_interval(Duration::from_millis(10))
            .build();

        let service = SmsSolverService::new_unchecked(provider, config);

        let result = service
            .get_number(Alpha2::UA.to_country(), MockService)
            .await
            .unwrap();

        let err = service
            .wait_for_sms_code(&result.task_id)
            .await
            .unwrap_err();

        match err {
            SmsSolverServiceError::CancelFailed { task_id, message } => {
                assert_eq!(task_id.as_ref(), "task123");
                assert!(message.contains("Cancel failed"));
            }
            _ => panic!("Expected CancelFailed error, got {:?}", err),
        }
    }

    #[tokio::test]
    async fn test_service_builder_try_build() {
        let provider = MockProvider::new().with_number("task123", "380501234567");

        let service = SmsSolverService::builder(provider)
            .timeout(Duration::from_secs(90))
            .poll_interval(Duration::from_secs(5))
            .try_build()
            .unwrap();

        assert_eq!(service.config().timeout, Duration::from_secs(90));
        assert_eq!(service.config().poll_interval, Duration::from_secs(5));
    }

    #[tokio::test]
    async fn test_service_with_config_presets() {
        let provider = MockProvider::new();

        let fast_service =
            SmsSolverService::try_new(provider.clone(), SmsSolverServiceConfig::fast()).unwrap();
        assert_eq!(fast_service.config().timeout, Duration::from_secs(60));
        assert_eq!(fast_service.config().poll_interval, Duration::from_secs(1));

        let patient_service =
            SmsSolverService::try_new(provider.clone(), SmsSolverServiceConfig::patient()).unwrap();
        assert_eq!(patient_service.config().timeout, Duration::from_secs(300));
        assert_eq!(
            patient_service.config().poll_interval,
            Duration::from_secs(5)
        );
    }

    #[test]
    fn test_try_new_validates_config() {
        let provider = MockProvider::new();
        let bad_config = SmsSolverServiceConfig {
            timeout: Duration::from_secs(1),
            poll_interval: Duration::from_secs(1),
        };
        assert!(SmsSolverService::try_new(provider, bad_config).is_err());
    }

    #[test]
    fn test_try_build_validates_config() {
        let provider = MockProvider::new();
        let result = SmsSolverService::builder(provider)
            .timeout(Duration::from_secs(1))
            .poll_interval(Duration::from_secs(5))
            .try_build();
        assert!(result.is_err());
    }
}
