//! Example demonstrating SMS.online provider with retry functionality.
//!
//! This example shows how to wrap the SMS.online provider with automatic
//! retry logic for handling transient failures.
//!
//! # Running
//!
//! ```bash
//! SMS_ONLINE_API_KEY=your_api_key cargo run --example sms_online_with_retry
//! ```

use sms_solvers::sms_online::{Service, SmsOnline, SmsOnlineProvider};
use sms_solvers::{
    Alpha2, RetryConfig, SmsRetryableProvider, SmsSolverService, SmsSolverServiceConfig,
    SmsSolverServiceTrait,
};
use std::env;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get API key from environment
    let api_key = env::var("SMS_ONLINE_API_KEY")
        .expect("SMS_ONLINE_API_KEY environment variable must be set");

    // Create the SMS.online client and provider
    let client = SmsOnline::with_api_key(&api_key)?;
    let provider = SmsOnlineProvider::new(client);

    // Configure retry behavior
    let retry_config = RetryConfig::default()
        .with_min_delay(Duration::from_millis(500))
        .with_max_delay(Duration::from_secs(5))
        .with_factor(2.0)
        .with_max_retries(3);

    // Wrap provider with retry logic
    let retryable_provider = SmsRetryableProvider::with_config(provider, retry_config);

    // Configure the service
    let config = SmsSolverServiceConfig::default()
        .with_timeout(Duration::from_secs(180))
        .with_poll_interval(Duration::from_secs(5));

    let service = SmsSolverService::try_new(retryable_provider, config)?;

    // Request a phone number for UK (WhatsApp verification)
    println!("Requesting phone number for UK (WhatsApp, with retry enabled)...");
    let result = service
        .get_number(Alpha2::GB.to_country(), Service::Whatsapp)
        .await?;

    println!("Got phone number:");
    println!("  Task ID: {}", result.task_id);
    println!("  Full number: {}", result.full_number.with_plus_prefix());
    println!("  Dial code: +{}", result.dial_code);
    println!("  Country: {}", result.country.iso_short_name());

    // Wait for SMS code with automatic retry on transient errors
    println!("\nWaiting for SMS code (with retry on failures)...");
    let code = service.wait_for_sms_code(&result.task_id).await?;

    println!("Received SMS code: {}", code);

    Ok(())
}
