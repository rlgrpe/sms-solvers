//! # SMS Solvers
//!
//! A generic SMS verification library with provider abstraction and fluent builder pattern.
//!
//! This library provides a unified interface for working with different SMS
//! verification services. It supports phone number acquisition, SMS code polling,
//! and activation management.
//!
//! ## Supported Providers
//!
//! | Provider | Feature | Website |
//! |----------|---------|---------|
//! | SMS Activate | `sms-activate` (default) | <https://sms-activate.org> |
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use sms_solvers::{
//!     SmsSolverService, SmsSolverServiceConfig, SmsSolverServiceTrait,
//!     sms_activate::{SmsActivateProvider, Service},
//!     SmsRetryableProvider, RetryConfig,
//! };
//! use std::time::Duration;
//! use isocountry::CountryCode;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create provider with API key
//!     let provider = SmsActivateProvider::new("your_api_key")?;
//!
//!     // Wrap with retry logic
//!     let retryable = SmsRetryableProvider::new(provider);
//!
//!     // Create service
//!     let service = SmsSolverService::builder(retryable)
//!         .timeout(Duration::from_secs(120))
//!         .poll_interval(Duration::from_secs(3))
//!         .build();
//!
//!     // Get a phone number
//!     let result = service.get_number(CountryCode::USA, Service::Whatsapp).await?;
//!     println!("Got number: {}", result.full_number);
//!
//!     // Wait for SMS code
//!     let code = service.wait_for_sms_code(&result.task_id).await?;
//!     println!("Got code: {}", code);
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Architecture
//!
//! ```text
//! SmsSolverService<P>
//!         │
//!         ▼
//! SmsRetryableProvider<P>  (optional retry wrapper)
//!         │
//!         ▼
//!     Provider          (trait: SmsActivateProvider, etc.)
//! ```
//!
//! ## Features
//!
//! - `sms-activate` - SMS Activate provider support (enabled by default)
//! - `tracing` - OpenTelemetry tracing instrumentation (enabled by default)

mod errors;
mod providers;
mod service;
mod types;
mod utils;

// Re-export error types
pub use errors::RetryableError;

// Re-export provider types
pub use providers::{Provider, SmsRetryableProvider};

// Re-export service types
pub use service::{
    ConfigError, SmsSolverService, SmsSolverServiceBuilder, SmsSolverServiceConfig,
    SmsSolverServiceConfigBuilder, SmsSolverServiceError, SmsSolverServiceTrait,
};

// Re-export CancellationToken for cancellable operations
pub use tokio_util::sync::CancellationToken;

// Re-export core types
pub use types::{DialCode, FullNumber, Number, SmsCode, SmsTaskResult, TaskId};

// Re-export utility types
pub use utils::RetryConfig;

// Re-export isocountry so users don't need to add it as a separate dependency
pub use isocountry::CountryCode;

// Re-export country to dial code utility
pub use utils::dial_code::country_to_dial_code;

/// SMS Activate provider types.
///
/// This module provides integration with the SMS Activate service
/// for phone number verification.
///
/// # Example
///
/// ```rust,ignore
/// use sms_solvers::sms_activate::{SmsActivateProvider, SmsActivateClient, Service};
/// use sms_solvers::{SmsSolverService, SmsSolverServiceTrait, SmsRetryableProvider};
/// use isocountry::CountryCode;
///
/// let client = SmsActivateClient::with_api_key("your_api_key")?;
/// let provider = SmsActivateProvider::new(client);
/// let service = SmsSolverService::with_provider(SmsRetryableProvider::new(provider));
///
/// let result = service.get_number(CountryCode::TUR, Service::Whatsapp).await?;
/// let code = service.wait_for_sms_code(&result.task_id).await?;
/// ```
#[cfg(feature = "sms-activate")]
pub mod sms_activate {
    pub use crate::providers::sms_activate::{
        Service, SmsActivateClient, SmsActivateError, SmsActivateProvider, SmsCountryExt,
    };
}
