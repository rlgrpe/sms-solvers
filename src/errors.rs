//! Error types and traits for SMS verification operations.

/// Trait for errors that can be classified as retryable or permanent.
///
/// This trait provides two levels of retryability classification:
///
/// 1. **Task-level** (`is_retryable`): Whether the same task_id operation should be retried.
///    Use this for transient errors like network timeouts or rate limits.
///
/// 2. **Operation-level** (`should_retry_operation`): Whether a fresh attempt
///    (creating a new task/getting a new number) might succeed. Use this when
///    a specific task failed but trying again with a new task could work.
///
/// # Examples
///
/// ```rust
/// use sms_solvers::RetryableError;
///
/// enum MyError {
///     NetworkTimeout,      // Retry same task
///     NumberBanned,        // Don't retry task, but try fresh operation
///     InvalidApiKey,       // Don't retry at all
///     ZeroBalance,         // Don't retry until account is funded
/// }
///
/// impl RetryableError for MyError {
///     fn is_retryable(&self) -> bool {
///         match self {
///             MyError::NetworkTimeout => true,
///             _ => false,
///         }
///     }
///
///     fn should_retry_operation(&self) -> bool {
///         match self {
///             MyError::NetworkTimeout => true,
///             MyError::NumberBanned => true,  // Fresh attempt might work
///             MyError::InvalidApiKey => false, // Won't ever work
///             MyError::ZeroBalance => false,   // Need to fund account first
///         }
///     }
/// }
/// ```
pub trait RetryableError {
    /// Returns true if this error represents a transient failure
    /// that might succeed on retry with the same task_id.
    ///
    /// Examples: network timeouts, rate limits, temporary service unavailability.
    fn is_retryable(&self) -> bool;

    /// Returns true if a fresh operation (getting a new number) might succeed.
    ///
    /// This is useful when a specific task failed (e.g., number was banned)
    /// but trying again with a completely new number might work.
    ///
    /// Default implementation returns the same as `is_retryable()`.
    ///
    /// Examples where this differs from `is_retryable()`:
    /// - `NumberBanned`: is_retryable=false, should_retry_operation=true
    /// - `TaskTimeout`: is_retryable=false, should_retry_operation=true
    /// - `ZeroBalance`: is_retryable=false, should_retry_operation=false
    fn should_retry_operation(&self) -> bool {
        self.is_retryable()
    }
}
