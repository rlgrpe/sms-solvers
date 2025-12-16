//! Service-level error types.

use crate::errors::RetryableError;
use crate::types::{DialCode, TaskId};
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
        "Timeout waiting for SMS code after {:.1}s (polled {} times); Task id: {task_id}",
        elapsed.as_secs_f64(),
        poll_count
    )]
    SmsTimeout {
        /// Configured timeout duration.
        timeout: Duration,
        /// Actual elapsed time.
        elapsed: Duration,
        /// Number of poll attempts made.
        poll_count: u32,
        /// The task ID that timed out.
        task_id: TaskId,
    },

    /// Cancellation was requested.
    #[error("Operation cancelled after {:.1}s (polled {} times); Task id: {task_id}", elapsed.as_secs_f64(), poll_count
    )]
    Cancelled {
        /// Elapsed time before cancellation.
        elapsed: Duration,
        /// Number of poll attempts made.
        poll_count: u32,
        /// The task ID that was cancelled.
        task_id: TaskId,
    },

    /// Failed to cancel activation after error/timeout.
    #[error("Failed to cancel activation for task {task_id}: {message}")]
    CancelFailed {
        /// The task ID that failed to cancel.
        task_id: TaskId,
        /// Error message from the cancellation attempt.
        message: String,
    },

    /// The dial code is blacklisted.
    #[error("Dial code +{dial_code} is blacklisted")]
    DialCodeBlacklisted {
        /// The blacklisted dial code.
        dial_code: DialCode,
        /// The task ID that was cancelled due to blacklist.
        task_id: TaskId,
    },

    /// No available dial codes after filtering.
    #[error("No available dial codes after filtering")]
    NoAvailableDialCodes,
}

impl RetryableError for SmsSolverServiceError {
    fn is_retryable(&self) -> bool {
        match self {
            SmsSolverServiceError::Provider { is_retryable, .. } => *is_retryable,
            SmsSolverServiceError::SmsTimeout { .. }
            | SmsSolverServiceError::Cancelled { .. }
            | SmsSolverServiceError::CancelFailed { .. }
            | SmsSolverServiceError::NoNumbersAvailable { .. }
            | SmsSolverServiceError::InvalidDialCode { .. }
            | SmsSolverServiceError::NumberParse { .. }
            | SmsSolverServiceError::DialCodeBlacklisted { .. }
            | SmsSolverServiceError::NoAvailableDialCodes => false,
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
            SmsSolverServiceError::Cancelled { .. }
            | SmsSolverServiceError::CancelFailed { .. }
            | SmsSolverServiceError::InvalidDialCode { .. }
            | SmsSolverServiceError::NumberParse { .. }
            | SmsSolverServiceError::DialCodeBlacklisted { .. }
            | SmsSolverServiceError::NoAvailableDialCodes => false,
        }
    }
}
