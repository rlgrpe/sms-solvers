//! Example demonstrating retry callbacks.
//!
//! This example shows how to use the `with_on_retry` callback to get
//! notified when retries occur, enabling custom logging or metrics.
//!
//! # Running
//!
//! ```bash
//! SMS_ACTIVATE_API_KEY=your_api_key cargo run --example retry_callbacks
//! ```

use sms_solvers::sms_activate::{Service, SmsActivateClient, SmsActivateProvider};
use sms_solvers::{
    Alpha2, RetryConfig, SmsRetryableProvider, SmsSolverService, SmsSolverServiceConfig,
    SmsSolverServiceTrait,
};
use std::env;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get API key from environment
    let api_key = env::var("SMS_ACTIVATE_API_KEY")
        .expect("SMS_ACTIVATE_API_KEY environment variable must be set");

    // Create the SMS Activate client and provider
    let client = SmsActivateClient::with_api_key(&api_key)?;
    let provider = SmsActivateProvider::new(client);

    // Track retry count across all operations
    let retry_count = Arc::new(AtomicU32::new(0));
    let retry_count_clone = Arc::clone(&retry_count);

    // Configure retry behavior
    let retry_config = RetryConfig::default()
        .with_min_delay(Duration::from_secs(1))
        .with_max_delay(Duration::from_secs(10))
        .with_max_retries(5);

    // Wrap provider with retry logic AND a callback
    let retryable_provider = SmsRetryableProvider::with_config(provider, retry_config)
        .with_on_retry(move |error, duration| {
            // Increment retry counter
            let count = retry_count_clone.fetch_add(1, Ordering::SeqCst) + 1;

            // Log the retry (you could send to a metrics system instead)
            println!(
                "[RETRY #{}] Error: {} | Next retry in: {:.1}s",
                count,
                error,
                duration.as_secs_f64()
            );
        });

    // Create service with fast preset for testing
    let config = SmsSolverServiceConfig::fast();
    let service = SmsSolverService::new(retryable_provider, config);

    // Request a phone number
    println!("Requesting phone number (retries will be logged)...\n");

    match service
        .get_number(Alpha2::US.to_country(), Service::Whatsapp)
        .await
    {
        Ok(result) => {
            println!(
                "\nGot phone number: {}",
                result.full_number.with_plus_prefix()
            );
            println!("Task ID: {}", result.task_id);

            // Wait for SMS code
            println!("\nWaiting for SMS code...\n");

            match service.wait_for_sms_code(&result.task_id).await {
                Ok(code) => {
                    println!("\nReceived SMS code: {}", code);
                }
                Err(e) => {
                    println!("\nFailed to get SMS code: {}", e);
                }
            }
        }
        Err(e) => {
            println!("\nFailed to get phone number: {}", e);
        }
    }

    // Report total retries
    let total_retries = retry_count.load(Ordering::SeqCst);
    println!("\n=== Summary ===");
    println!("Total retry attempts: {}", total_retries);

    Ok(())
}
