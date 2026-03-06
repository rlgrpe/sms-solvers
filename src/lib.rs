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
//! | Hero SMS | `hero-sms` (default) | <https://hero-sms.com> |
//! | SMS.online | `sms-online` (default) | <https://sms.online> |
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use sms_solvers::{
//!     SmsSolverService, SmsSolverServiceTrait, Alpha2,
//!     hero_sms::{HeroSms, HeroSmsProvider, Service},
//!     SmsRetryableProvider,
//! };
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create client and provider
//!     let client = HeroSms::with_api_key("your_api_key")?;
//!     let provider = HeroSmsProvider::new(client);
//!
//!     // Wrap with retry logic
//!     let retryable = SmsRetryableProvider::new(provider);
//!
//!     // Create service with validated config
//!     let service = SmsSolverService::builder(retryable)
//!         .timeout(std::time::Duration::from_secs(120))
//!         .poll_interval(std::time::Duration::from_secs(3))
//!         .try_build()?;
//!
//!     // Get a phone number and wait for SMS (using ActivationHandle)
//!     let activation = service.activate(Alpha2::US.to_country(), Service::Whatsapp).await?;
//!     println!("Got number: {}", activation.full_number());
//!
//!     let code = activation.wait_for_code().await?;
//!     println!("Got code: {}", code);
//!
//!     activation.finish().await?;
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
//!     Provider          (trait: HeroSmsProvider, etc.)
//! ```
//!
//! ## Features
//!
//! - `hero-sms` - Hero SMS provider support (enabled by default)
//! - `sms-online` - SMS.online provider support (enabled by default)
//! - `tracing` - OpenTelemetry tracing instrumentation (enabled by default)
//! - `metrics` - OpenTelemetry metrics (counters, histograms)

mod errors;
mod providers;
mod service;
mod types;
mod utils;

// Re-export error types
pub use errors::RetryableError;

// Re-export provider types
pub use providers::{Provider, ProviderCapabilities, SmsRetryableProvider};

// Re-export service types
pub use service::{
    ActivationHandle, ConfigError, SmsSolverService, SmsSolverServiceBuilder,
    SmsSolverServiceConfig, SmsSolverServiceConfigBuilder, SmsSolverServiceError,
    SmsSolverServiceTrait,
};

// Re-export CancellationToken for cancellable operations
pub use tokio_util::sync::CancellationToken;

// Re-export core types
pub use types::{
    DialCode, DialCodeError, FullNumber, Number, NumberError, SmsCode, SmsTaskResult, TaskId,
};

// Re-export utility types
pub use utils::RetryConfig;

// Re-export keshvar so users don't need to add it as a separate dependency
pub use keshvar::{Alpha2, Country};
// Re-export utility types
pub use types::DialCodeToCountryError;

/// Hero SMS provider types.
///
/// This module provides integration with the Hero SMS service
/// for phone number verification.
///
/// # Example
///
/// ```rust,ignore
/// use sms_solvers::hero_sms::{HeroSms, HeroSmsProvider, Service};
/// use sms_solvers::{SmsSolverService, SmsRetryableProvider, Alpha2};
///
/// let client = HeroSms::with_api_key("your_api_key")?;
/// let provider = HeroSmsProvider::new(client);
/// let service = SmsSolverService::with_provider(SmsRetryableProvider::new(provider));
///
/// let activation = service.activate(Alpha2::TR.to_country(), Service::Whatsapp).await?;
/// let code = activation.wait_for_code().await?;
/// activation.finish().await?;
/// ```
#[cfg(feature = "hero-sms")]
pub mod hero_sms {
    pub use crate::providers::hero_sms::{
        GetNumberOptions, HeroSms, HeroSmsError, HeroSmsProvider, Service, SmsCountryExt,
    };
}

/// SMS.online provider types.
#[cfg(feature = "sms-online")]
pub mod sms_online {
    pub use crate::providers::sms_online::{
        ActivationType, GetNumberOptions, ProviderSelection, Service, SmsOnline,
        SmsOnlineCountryExt, SmsOnlineError, SmsOnlineProvider,
    };
}
