//! Example demonstrating cancellation functionality.
//!
//! This example shows how to use `wait_for_sms_code_cancellable` to allow
//! cancelling the SMS wait operation from another task.
//!
//! # Running
//!
//! ```bash
//! SMS_ACTIVATE_API_KEY=your_api_key cargo run --example with_cancellation
//! ```

use isocountry::CountryCode;
use sms_solvers::sms_activate::{Service, SmsActivateClient, SmsActivateProvider};
use sms_solvers::{
    CancellationToken, SmsRetryableProvider, SmsSolverService, SmsSolverServiceConfig,
    SmsSolverServiceError, SmsSolverServiceTrait,
};
use std::env;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get API key from environment
    let api_key = env::var("SMS_ACTIVATE_API_KEY")
        .expect("SMS_ACTIVATE_API_KEY environment variable must be set");

    // Create the SMS Activate client and provider
    let client = SmsActivateClient::with_api_key(&api_key)?;
    let provider = SmsActivateProvider::new(client);
    let retryable = SmsRetryableProvider::new(provider);

    // Use the patient preset for longer timeout
    let config = SmsSolverServiceConfig::patient();
    let service = SmsSolverService::new(retryable, config);

    // Request a phone number
    println!("Requesting phone number...");
    let result = service
        .get_number(CountryCode::UKR, Service::InstagramThreads)
        .await?;

    println!(
        "Got phone number: {}",
        result.full_number.with_plus_prefix()
    );
    println!("Task ID: {}", result.task_id);

    // Create a cancellation token
    let cancel_token = CancellationToken::new();
    let token_clone = cancel_token.clone();

    // Spawn a task that will cancel after 30 seconds if no SMS received
    let cancel_handle = tokio::spawn(async move {
        println!("\nPress Ctrl+C or wait 30s to cancel...");
        tokio::time::sleep(Duration::from_secs(30)).await;
        println!("Cancelling operation...");
        token_clone.cancel();
    });

    // Wait for SMS code with cancellation support
    println!("\nWaiting for SMS code (cancellable)...");
    match service
        .wait_for_sms_code_cancellable(&result.task_id, cancel_token)
        .await
    {
        Ok(code) => {
            cancel_handle.abort(); // Stop the cancel timer
            println!("Received SMS code: {}", code);
        }
        Err(SmsSolverServiceError::Cancelled {
            elapsed,
            poll_count,
            ..
        }) => {
            println!(
                "Operation was cancelled after {:.1}s ({} polls)",
                elapsed.as_secs_f64(),
                poll_count
            );
        }
        Err(SmsSolverServiceError::SmsTimeout {
            elapsed,
            poll_count,
            ..
        }) => {
            println!(
                "Timed out after {:.1}s ({} polls)",
                elapsed.as_secs_f64(),
                poll_count
            );
        }
        Err(e) => {
            println!("Error: {}", e);
        }
    }

    Ok(())
}
