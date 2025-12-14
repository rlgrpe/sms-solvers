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
use tokio_util::sync::CancellationToken;

#[cfg(feature = "tracing")]
use tracing::{debug, error, info, warn};

#[cfg(feature = "metrics")]
use opentelemetry::{
    KeyValue, global,
    metrics::{Counter, Histogram},
};

#[cfg(feature = "metrics")]
use std::sync::OnceLock;

/// Metrics for the SMS Solver service.
#[cfg(feature = "metrics")]
struct ServiceMetrics {
    /// Counter for number requests.
    numbers_requested: Counter<u64>,
    /// Counter for successful SMS codes received.
    sms_codes_received: Counter<u64>,
    /// Counter for timeouts.
    timeouts: Counter<u64>,
    /// Counter for cancellations.
    cancellations: Counter<u64>,
    /// Counter for errors.
    errors: Counter<u64>,
    /// Histogram for SMS wait times in seconds.
    sms_wait_time: Histogram<f64>,
    /// Histogram for poll counts.
    poll_counts: Histogram<u64>,
}

#[cfg(feature = "metrics")]
impl ServiceMetrics {
    fn global() -> &'static Self {
        static METRICS: OnceLock<ServiceMetrics> = OnceLock::new();
        METRICS.get_or_init(|| {
            let meter = global::meter("sms_solvers");
            Self {
                numbers_requested: meter
                    .u64_counter("sms_solvers.numbers_requested")
                    .with_description("Number of phone number requests")
                    .build(),
                sms_codes_received: meter
                    .u64_counter("sms_solvers.sms_codes_received")
                    .with_description("Number of SMS codes successfully received")
                    .build(),
                timeouts: meter
                    .u64_counter("sms_solvers.timeouts")
                    .with_description("Number of SMS wait timeouts")
                    .build(),
                cancellations: meter
                    .u64_counter("sms_solvers.cancellations")
                    .with_description("Number of cancelled operations")
                    .build(),
                errors: meter
                    .u64_counter("sms_solvers.errors")
                    .with_description("Number of errors")
                    .build(),
                sms_wait_time: meter
                    .f64_histogram("sms_solvers.sms_wait_time_seconds")
                    .with_description("Time spent waiting for SMS codes")
                    .build(),
                poll_counts: meter
                    .u64_histogram("sms_solvers.poll_counts")
                    .with_description("Number of polls before receiving SMS")
                    .build(),
            }
        })
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

        #[cfg(feature = "metrics")]
        ServiceMetrics::global()
            .numbers_requested
            .add(1, &[KeyValue::new("country", country.alpha2().to_string())]);

        let (task_id, full_number) = self
            .provider
            .get_phone_number(country, service)
            .await
            .map_err(|e| {
                #[cfg(feature = "metrics")]
                ServiceMetrics::global().errors.add(
                    1,
                    &[
                        KeyValue::new("country", country.alpha2().to_string()),
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
        self.wait_for_sms_code_cancellable(task_id, CancellationToken::new())
            .await
    }

    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(
            name = "sms_solver.wait_for_code_cancellable",
            skip_all,
            fields(task_id = %task_id)
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

        #[cfg(feature = "tracing")]
        debug!(timeout_secs = %timeout.as_secs_f64(), "Starting SMS code polling");

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

                // Try to cancel the activation
                if let Err(e) = self.provider.cancel_activation(task_id).await {
                    #[cfg(feature = "tracing")]
                    warn!(error = %e, "Failed to cancel activation after cancellation request");

                    return Err(SmsSolverServiceError::CancelFailed {
                        task_id: task_id.clone(),
                        message: e.to_string(),
                    });
                }

                return Err(SmsSolverServiceError::Cancelled {
                    elapsed,
                    poll_count,
                    task_id: task_id.clone(),
                });
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

                // Try to cancel the activation
                if let Err(e) = self.provider.cancel_activation(task_id).await {
                    #[cfg(feature = "tracing")]
                    warn!(error = %e, "Failed to cancel activation after timeout");

                    return Err(SmsSolverServiceError::CancelFailed {
                        task_id: task_id.clone(),
                        message: e.to_string(),
                    });
                }

                return Err(SmsSolverServiceError::SmsTimeout {
                    timeout,
                    elapsed,
                    poll_count,
                    task_id: task_id.clone(),
                });
            }

            poll_count += 1;

            match self.provider.get_sms_code(task_id).await {
                Ok(Some(code)) => {
                    let elapsed = start.elapsed();

                    #[cfg(feature = "tracing")]
                    info!(
                        code = %code,
                        elapsed_secs = %elapsed.as_secs_f64(),
                        poll_count = %poll_count,
                        "SMS code received"
                    );

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
                        error = %e,
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

                    // Try to cancel the activation
                    if let Err(cancel_err) = self.provider.cancel_activation(task_id).await {
                        #[cfg(feature = "tracing")]
                        warn!(error = %cancel_err, "Failed to cancel activation after error");

                        return Err(SmsSolverServiceError::CancelFailed {
                            task_id: task_id.clone(),
                            message: cancel_err.to_string(),
                        });
                    }

                    return Err(SmsSolverServiceError::Provider {
                        source: Box::new(e) as Box<dyn StdError + Send + Sync>,
                        is_retryable: false,
                        should_retry_operation,
                    });
                }
                Err(_e) => {
                    #[cfg(feature = "tracing")]
                    warn!(error = %_e, poll_count = %poll_count, "Transient error during polling, continuing");
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::RetryableError;
    use crate::types::FullNumber;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::time::Duration;
    use thiserror::Error;

    // Mock provider for testing
    #[derive(Clone)]
    #[allow(clippy::type_complexity)]
    struct MockProvider {
        get_number_result: Arc<std::sync::Mutex<Option<Result<(TaskId, FullNumber), MockError>>>>,
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
                Some(Ok((TaskId::new(task_id), FullNumber::new(number))));
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
            _country: CountryCode,
            _service: Self::Service,
        ) -> Result<(TaskId, FullNumber), Self::Error> {
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

        let service = SmsSolverService::new(provider.clone(), config);

        let result = service
            .get_number(CountryCode::UKR, MockService)
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

        let service = SmsSolverService::new(provider, config);

        let result = service
            .get_number(CountryCode::UKR, MockService)
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

        let service = SmsSolverService::new(provider, config);

        let result = service
            .get_number(CountryCode::UKR, MockService)
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

        let service = SmsSolverService::new(provider, config);

        let result = service
            .get_number(CountryCode::UKR, MockService)
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
    async fn test_service_builder() {
        let provider = MockProvider::new().with_number("task123", "380501234567");

        let service = SmsSolverService::builder(provider)
            .timeout(Duration::from_secs(90))
            .poll_interval(Duration::from_secs(5))
            .build();

        assert_eq!(service.config().timeout, Duration::from_secs(90));
        assert_eq!(service.config().poll_interval, Duration::from_secs(5));
    }

    #[tokio::test]
    async fn test_service_with_config_presets() {
        let provider = MockProvider::new();

        let fast_service = SmsSolverService::new(provider.clone(), SmsSolverServiceConfig::fast());
        assert_eq!(fast_service.config().timeout, Duration::from_secs(60));
        assert_eq!(fast_service.config().poll_interval, Duration::from_secs(1));

        let patient_service =
            SmsSolverService::new(provider.clone(), SmsSolverServiceConfig::patient());
        assert_eq!(patient_service.config().timeout, Duration::from_secs(300));
        assert_eq!(
            patient_service.config().poll_interval,
            Duration::from_secs(5)
        );
    }
}
