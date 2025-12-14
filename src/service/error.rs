//! Service-level error types.

use crate::errors::RetryableError;
use crate::types::TaskId;
use isocountry::CountryCode;
use std::error::Error as StdError;
use std::time::Duration;
use thiserror::Error;

/// Service-level errors that wrap provider errors.
#[derive(Debug, Error)]
pub enum SmsSolverServiceError {
    /// Error from the underlying provider.
    #[error("SMS provider error: {source}")]
    Provider {
        #[source]
        source: Box<dyn StdError + Send + Sync>,
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

impl RetryableError for SmsSolverServiceError {
    fn is_retryable(&self) -> bool {
        match self {
            SmsSolverServiceError::Provider { is_retryable, .. } => *is_retryable,
            SmsSolverServiceError::SmsTimeout { .. } => false,
            SmsSolverServiceError::NoNumbersAvailable { .. }
            | SmsSolverServiceError::InvalidDialCode { .. }
            | SmsSolverServiceError::NumberParse { .. } => false,
        }
    }

    fn should_retry_operation(&self) -> bool {
        match self {
            SmsSolverServiceError::Provider {
                should_retry_operation,
                ..
            } => *should_retry_operation,
            SmsSolverServiceError::SmsTimeout { .. } => true,
            SmsSolverServiceError::NoNumbersAvailable { .. } => true,
            SmsSolverServiceError::InvalidDialCode { .. }
            | SmsSolverServiceError::NumberParse { .. } => false,
        }
    }
}
