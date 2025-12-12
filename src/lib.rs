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
//!     SmsService, SmsServiceConfig, SmsServiceTrait,
//!     providers::sms_activate::SmsActivateProvider,
//!     RetryableProvider, RetryConfig,
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
//!     let retryable = RetryableProvider::new(provider);
//!
//!     // Create service
//!     let config = SmsServiceConfig {
//!         wait_sms_code_timeout: Duration::from_secs(120),
//!         poll_interval: Duration::from_secs(3),
//!     };
//!     let service = SmsService::new(retryable, config);
//!
//!     // Get a phone number
//!     let result = service.get_number(CountryCode::USA).await?;
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
//! SmsService<P>
//!         │
//!         ▼
//! RetryableProvider<P>  (optional retry wrapper)
//!         │
//!         ▼
//!     Provider          (trait: SmsActivateProvider, etc.)
//! ```
//!
//! ## Features
//!
//! - `sms-activate` - SMS Activate provider support (enabled by default)
//! - `tracing` - OpenTelemetry tracing instrumentation (enabled by default)

pub mod errors;
pub mod provider;
pub mod providers;
pub mod retry;
pub mod service;
pub mod types;

// Re-export commonly used types at the crate root
pub use errors::RetryableError;
pub use provider::{Provider, RetryableProvider};
pub use retry::RetryConfig;
pub use service::{ServiceError, SmsService, SmsServiceConfig, SmsServiceTrait};
pub use types::{DialCode, FullNumber, Number, SmsCode, SmsTaskResult, TaskId};
