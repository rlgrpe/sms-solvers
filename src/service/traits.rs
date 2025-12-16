//! Service trait definition.

use crate::errors::RetryableError;
use crate::types::{SmsCode, SmsTaskResult, TaskId};
use keshvar::Country;
use std::error::Error as StdError;
use std::future::Future;
use tokio_util::sync::CancellationToken;

/// Trait for SMS verification service implementations.
///
/// This trait abstracts the service interface, allowing different
/// service implementations to be used interchangeably.
///
/// # Note on async methods
///
/// All async methods in this trait return `Send` futures, making them
/// compatible with multi-threaded executors.
#[allow(async_fn_in_trait)]
pub trait SmsSolverServiceTrait: Send + Sync {
    /// The error type for this service.
    type Error: StdError + RetryableError + Send;

    /// The service type for phone number requests (e.g., WhatsApp, Instagram).
    type Service: Clone + Send + Sync;

    /// Get a phone number for the specified country and service.
    ///
    /// # Arguments
    ///
    /// * `country` - Country for the desired phone number
    /// * `service` - The service to get a number for (e.g., WhatsApp verification)
    ///
    /// # Returns
    ///
    /// The SMS task result containing the phone number and task ID.
    fn get_number(
        &self,
        country: Country,
        service: Self::Service,
    ) -> impl Future<Output = Result<SmsTaskResult, Self::Error>> + Send;

    /// Wait for an SMS code to be received.
    ///
    /// This method polls the provider until an SMS code is received
    /// or the timeout is reached.
    ///
    /// # Arguments
    ///
    /// * `task_id` - The task identifier from `get_number`
    ///
    /// # Returns
    ///
    /// The received SMS code.
    fn wait_for_sms_code(
        &self,
        task_id: &TaskId,
    ) -> impl Future<Output = Result<SmsCode, Self::Error>> + Send;

    /// Wait for an SMS code with cancellation support.
    ///
    /// This method polls the provider until an SMS code is received,
    /// the timeout is reached, or cancellation is requested.
    ///
    /// # Arguments
    ///
    /// * `task_id` - The task identifier from `get_number`
    /// * `cancel_token` - Token to signal cancellation
    ///
    /// # Returns
    ///
    /// The received SMS code, or an error if cancelled/timed out.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use tokio_util::sync::CancellationToken;
    ///
    /// let cancel_token = CancellationToken::new();
    /// let token_clone = cancel_token.clone();
    ///
    /// // Spawn a task to cancel after 30 seconds
    /// tokio::spawn(async move {
    ///     tokio::time::sleep(Duration::from_secs(30)).await;
    ///     token_clone.cancel();
    /// });
    ///
    /// match service.wait_for_sms_code_cancellable(&task_id, cancel_token).await {
    ///     Ok(code) => println!("Got code: {}", code),
    ///     Err(SmsSolverServiceError::Cancelled { .. }) => println!("Cancelled by user"),
    ///     Err(e) => println!("Error: {}", e),
    /// }
    /// ```
    fn wait_for_sms_code_cancellable(
        &self,
        task_id: &TaskId,
        cancel_token: CancellationToken,
    ) -> impl Future<Output = Result<SmsCode, Self::Error>> + Send;
}
