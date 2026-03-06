//! Hero SMS provider implementation.
//!
//! This module provides integration with the Hero SMS service
//! for phone number verification.
//!
//! # Example
//!
//! ```rust,ignore
//! use sms_solvers::hero_sms::{HeroSms, HeroSmsProvider, Service};
//! use sms_solvers::{SmsSolverService, SmsRetryableProvider, SmsSolverServiceTrait, Alpha2};
//!
//! // Create client and provider
//! let client = HeroSms::with_api_key("your_api_key")?;
//! let provider = HeroSmsProvider::new(client);
//!
//! // Wrap with retry logic
//! let retryable = SmsRetryableProvider::new(provider);
//!
//! // Create service
//! let service = SmsSolverService::with_provider(retryable);
//!
//! // Get a phone number for WhatsApp
//! let result = service.get_number(Alpha2::TR.to_country(), Service::Whatsapp).await?;
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
pub use types::GetNumberOptions;
