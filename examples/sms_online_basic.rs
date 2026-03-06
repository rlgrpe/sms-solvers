//! Basic usage example for SMS.online provider.
//!
//! This example demonstrates how to create an SMS service using the SMS.online
//! provider and use it to get a phone number and wait for an SMS code.
//!
//! # Running
//!
//! ```bash
//! SMS_ONLINE_API_KEY=your_api_key cargo run --example sms_online_basic
//! ```

use sms_solvers::sms_online::{Service, SmsOnline, SmsOnlineProvider};
use sms_solvers::{Alpha2, SmsSolverService, SmsSolverServiceConfig, SmsSolverServiceTrait};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get API key from environment
    let api_key = env::var("SMS_ONLINE_API_KEY")
        .expect("SMS_ONLINE_API_KEY environment variable must be set");

    // Create the SMS.online client
    let client = SmsOnline::with_api_key(&api_key)?;

    // Create the provider
    let provider = SmsOnlineProvider::new(client);

    // Use the balanced preset - 120s timeout, 3s poll interval
    let config = SmsSolverServiceConfig::balanced();
    let service = SmsSolverService::new(provider, config);

    // Request a phone number for Ukraine for Instagram verification
    println!("Requesting phone number for Ukraine (Instagram)...");
    let result = service
        .get_number(Alpha2::UA.to_country(), Service::InstagramThreads)
        .await?;

    println!("Got phone number:");
    println!("  Task ID: {}", result.task_id);
    println!("  Full number: {}", result.full_number.with_plus_prefix());
    println!("  Dial code: +{}", result.dial_code);
    println!("  Number: {}", result.number);
    println!(
        "  Country: {} ({:?})",
        result.country.iso_short_name(),
        result.country.alpha2()
    );

    // Wait for SMS code
    println!("\nWaiting for SMS code...");
    let code = service.wait_for_sms_code(&result.task_id).await?;

    println!("Received SMS code: {}", code);

    Ok(())
}
