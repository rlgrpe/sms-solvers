//! Service trait definition.

use crate::errors::RetryableError;
use crate::types::{SmsCode, SmsTaskResult, TaskId};
use isocountry::CountryCode;
use std::error::Error as StdError;

/// Trait for SMS verification service implementations.
///
/// This trait abstracts the service interface, allowing different
/// service implementations to be used interchangeably.
pub trait SmsSolverServiceTrait: Send + Sync {
    /// The error type for this service.
    type Error: StdError + RetryableError;

    /// The service type for phone number requests (e.g., WhatsApp, Instagram).
    type Service: Clone + Send + Sync;

    /// Get a phone number for the specified country and service.
    ///
    /// # Arguments
    ///
    /// * `country` - ISO country code for the desired phone number
    /// * `service` - The service to get a number for (e.g., WhatsApp verification)
    ///
    /// # Returns
    ///
    /// The SMS task result containing the phone number and task ID.
    async fn get_number(
        &self,
        country: CountryCode,
        service: Self::Service,
    ) -> Result<SmsTaskResult, Self::Error>;

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
    async fn wait_for_sms_code(&self, task_id: &TaskId) -> Result<SmsCode, Self::Error>;
}
