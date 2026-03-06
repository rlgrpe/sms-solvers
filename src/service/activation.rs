//! Activation handle for managing SMS activation lifecycle.

use super::core::SmsSolverService;
use super::error::SmsSolverServiceError;
use crate::errors::RetryableError;
use crate::providers::traits::Provider;
use crate::types::{DialCode, FullNumber, Number, SmsCode, SmsTaskResult, TaskId};
use keshvar::Country;
use std::fmt::{Debug, Display};
use tokio_util::sync::CancellationToken;

/// A handle to an in-progress SMS activation.
///
/// Created by [`SmsSolverService::activate`]. Bundles the activation data
/// with the service reference so callers can drive the entire lifecycle
/// through a single object.
///
/// # Example
///
/// ```rust,ignore
/// use sms_solvers::{SmsSolverService, SmsSolverServiceTrait, Alpha2};
/// use sms_solvers::hero_sms::{HeroSms, HeroSmsProvider, Service};
///
/// let client = HeroSms::with_api_key("api_key")?;
/// let provider = HeroSmsProvider::new(client);
/// let service = SmsSolverService::try_new(provider, Default::default())?;
///
/// let activation = service.activate(Alpha2::US.to_country(), Service::Whatsapp).await?;
/// println!("Got number: {}", activation.full_number());
///
/// let code = activation.wait_for_code().await?;
/// println!("Got code: {}", code);
///
/// activation.finish().await?;
/// ```
pub struct ActivationHandle<P: Provider> {
    service: SmsSolverService<P>,
    result: SmsTaskResult,
}

impl<P: Provider> ActivationHandle<P>
where
    P::Error: Debug + Display + RetryableError + Send + Sync + 'static,
{
    pub(crate) fn new(service: SmsSolverService<P>, result: SmsTaskResult) -> Self {
        Self { service, result }
    }

    /// The unique task identifier for this activation.
    pub fn task_id(&self) -> &TaskId {
        &self.result.task_id
    }

    /// The full phone number (with dial code).
    pub fn full_number(&self) -> &FullNumber {
        &self.result.full_number
    }

    /// The national phone number (without dial code).
    pub fn number(&self) -> &Number {
        &self.result.number
    }

    /// The country dial code.
    pub fn dial_code(&self) -> &DialCode {
        &self.result.dial_code
    }

    /// The country.
    pub fn country(&self) -> &Country {
        &self.result.country
    }

    /// Consume the handle and return the underlying [`SmsTaskResult`].
    pub fn into_result(self) -> SmsTaskResult {
        self.result
    }

    /// Get a reference to the underlying [`SmsTaskResult`].
    pub fn result(&self) -> &SmsTaskResult {
        &self.result
    }

    /// Wait for an SMS code to be received.
    ///
    /// Polls the provider until a code arrives or the service timeout is reached.
    /// On timeout or permanent error, the activation is automatically cancelled.
    pub async fn wait_for_code(&self) -> Result<SmsCode, SmsSolverServiceError> {
        use super::traits::SmsSolverServiceTrait;
        self.service.wait_for_sms_code(&self.result.task_id).await
    }

    /// Wait for an SMS code with cancellation support.
    ///
    /// Same as [`wait_for_code`](Self::wait_for_code) but also respects the
    /// given [`CancellationToken`].
    pub async fn wait_for_code_cancellable(
        &self,
        cancel_token: CancellationToken,
    ) -> Result<SmsCode, SmsSolverServiceError> {
        use super::traits::SmsSolverServiceTrait;
        self.service
            .wait_for_sms_code_cancellable(&self.result.task_id, cancel_token)
            .await
    }

    /// Mark the activation as successfully completed.
    ///
    /// Call this after you have used the SMS code.
    /// Consumes the handle to prevent further use.
    pub async fn finish(self) -> Result<(), SmsSolverServiceError> {
        self.service
            .provider
            .finish_activation(&self.result.task_id)
            .await
            .map_err(|e| {
                let is_retryable = e.is_retryable();
                let should_retry_operation = e.should_retry_operation();
                SmsSolverServiceError::Provider {
                    source: Box::new(e),
                    is_retryable,
                    should_retry_operation,
                }
            })
    }

    /// Cancel the activation.
    ///
    /// Call this when you no longer need the phone number.
    /// Consumes the handle to prevent further use.
    pub async fn cancel(self) -> Result<(), SmsSolverServiceError> {
        self.service
            .provider
            .cancel_activation(&self.result.task_id)
            .await
            .map_err(|e| {
                let is_retryable = e.is_retryable();
                let should_retry_operation = e.should_retry_operation();
                SmsSolverServiceError::Provider {
                    source: Box::new(e),
                    is_retryable,
                    should_retry_operation,
                }
            })
    }
}

impl<P: Provider> Debug for ActivationHandle<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ActivationHandle")
            .field("task_id", &self.result.task_id)
            .field("country", &self.result.country.iso_short_name())
            .finish_non_exhaustive()
    }
}
