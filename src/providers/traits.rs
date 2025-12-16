//! Provider trait definition.

use crate::errors::RetryableError;
use crate::types::{DialCode, FullNumber, SmsCode, TaskId};
use keshvar::Country;
use std::error::Error as StdError;
use std::future::Future;

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
/// # Note on async methods
///
/// All async methods in this trait return `Send` futures, making them
/// compatible with multi-threaded executors.
///
/// # Example
///
/// ```rust,ignore
/// use sms_solvers::{Provider, TaskId, FullNumber, SmsCode, Country};
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
///     async fn get_phone_number(&self, country: Country, service: Self::Service) -> Result<(TaskId, FullNumber), Self::Error> {
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
#[allow(async_fn_in_trait)]
pub trait Provider: Send + Sync + Clone {
    /// Error type returned by provider operations.
    type Error: StdError + RetryableError + Send + Sync + 'static;

    /// Service type for phone number requests.
    /// Each provider can define its own service type (e.g., WhatsApp, Instagram, etc.)
    type Service: Clone + Send + Sync;

    /// Get a phone number for the specified country and service.
    ///
    /// # Arguments
    /// * `country` - Country for the desired phone number
    /// * `service` - The service to get a number for (e.g., WhatsApp verification)
    ///
    /// # Returns
    /// * `task_id` - Unique identifier for this activation
    /// * `full_number` - The full phone number with country code
    fn get_phone_number(
        &self,
        country: Country,
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

    /// Check if the provider supports the given service.
    ///
    /// This method allows checking if a specific service (e.g., WhatsApp, Instagram)
    /// is supported by the provider before attempting to acquire a number.
    ///
    /// Default implementation returns true, assuming all services are supported.
    fn supports_service(&self, service: &Self::Service) -> bool {
        let _ = service;
        true
    }

    /// Get the list of countries where the given service is available.
    ///
    /// This method returns a list of countries where phone numbers
    /// can be acquired for the specified service.
    ///
    /// Default implementation returns an empty list, indicating that
    /// available countries should be determined through other means
    /// (e.g., trying to get a number and handling errors).
    ///
    /// Providers can override this to provide a static or dynamic list
    /// of supported countries.
    fn available_countries(&self, service: &Self::Service) -> Vec<Country> {
        let _ = service;
        Vec::new()
    }

    /// Get the list of all services supported by this provider.
    ///
    /// Default implementation returns an empty list. Providers should
    /// override this to return their supported services.
    fn supported_services(&self) -> Vec<Self::Service> {
        Vec::new()
    }
}
