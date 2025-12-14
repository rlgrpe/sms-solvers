//! Basic usage example for SMS Solvers.
//!
//! This example demonstrates how to create an SMS service and use it
//! to get a phone number and wait for an SMS code.
//!
//! # Running
//!
//! ```bash
//! SMS_ACTIVATE_API_KEY=your_api_key cargo run --example basic_usage
//! ```

use isocountry::CountryCode;
use sms_solvers::sms_activate::{Service, SmsActivateClient, SmsActivateProvider};
use sms_solvers::{SmsSolverService, SmsSolverServiceConfig, SmsSolverServiceTrait};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get API key from environment
    let api_key = env::var("SMS_ACTIVATE_API_KEY")
        .expect("SMS_ACTIVATE_API_KEY environment variable must be set");

    // Create the SMS Activate client
    let client = SmsActivateClient::with_api_key(&api_key)?;

    // Create the provider (service-agnostic)
    let provider = SmsActivateProvider::new(client);

    // Use the balanced preset (default) - 120s timeout, 3s poll interval
    // Other presets available: SmsSolverServiceConfig::fast(), SmsSolverServiceConfig::patient()
    let config = SmsSolverServiceConfig::balanced();

    // Validate the config (optional but recommended for custom configs)
    config.validate()?;

    // Create the service
    let service = SmsSolverService::new(provider, config);

    // Request a phone number for Ukraine for Instagram verification
    println!("Requesting phone number for Ukraine (Instagram)...");
    let result = service
        .get_number(CountryCode::UKR, Service::InstagramThreads)
        .await?;

    println!("Got phone number:");
    println!("  Task ID: {}", result.task_id);
    // Use with_plus_prefix() for international format
    println!("  Full number: {}", result.full_number.with_plus_prefix());
    println!("  Dial code: +{}", result.dial_code);
    println!("  Number: {}", result.number);
    println!(
        "  Country: {} ({})",
        result.country.name(),
        result.country.alpha2()
    );

    // Wait for SMS code
    println!("\nWaiting for SMS code...");
    let code = service.wait_for_sms_code(&result.task_id).await?;

    println!("Received SMS code: {}", code);

    Ok(())
}
