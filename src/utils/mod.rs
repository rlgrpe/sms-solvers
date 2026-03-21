//! Internal utilities.

pub(crate) mod error_chain;
pub(crate) mod retry;
#[cfg(feature = "tracing")]
pub(crate) mod span_status;

pub use retry::RetryConfig;

pub(crate) const REDACTED: &str = "[REDACTED]";
