//! Hero SMS provider implementation.
//!
//! This module provides integration with the Hero SMS service
//! for phone number verification.
//!
//! # Example
//!
//! ```rust,ignore
//! use sms_solvers::providers::hero_sms::{HeroSmsProvider, HeroSms, Service};
//! use sms_solvers::{SmsService, SmsServiceConfig, SmsServiceTrait, RetryableProvider};
//! use std::time::Duration;
//! use isocountry::CountryCode;
//!
//! // Create client and provider for WhatsApp verification
//! let client = HeroSms::with_api_key("your_api_key")?;
//! let provider = HeroSmsProvider::new(client, Service::Whatsapp);
//!
//! // Wrap with retry logic
//! let retryable = RetryableProvider::new(provider);
//!
//! // Create service
//! let config = SmsServiceConfig {
//!     wait_sms_code_timeout: Duration::from_secs(120),
//!     poll_interval: Duration::from_secs(3),
//! };
//! let service = SmsService::new(retryable, config);
//!
//! // Get a phone number
//! let result = service.get_number(CountryCode::TUR).await?;
//! println!("Got number: {}", result.full_number);
//!
//! // Wait for SMS code
//! let code = service.wait_for_sms_code(&result.task_id).await?;
//! println!("Got code: {}", code);
//! ```

pub mod client;
pub mod countries;
pub mod errors;
pub mod provider;
mod response;
pub mod services;
pub mod types;

// Re-export commonly used types
pub use client::HeroSms;
pub use countries::SmsCountryExt;
pub use errors::HeroSmsError;
pub use provider::HeroSmsProvider;
pub use services::Service;
