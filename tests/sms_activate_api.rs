//! Integration tests for SMS Activate API.
//!
//! These tests make real API calls and require a valid API key.
//! They are ignored by default and should be run manually.
//!
//! # Setup
//!
//! 1. Copy the example env file:
//!    ```bash
//!    cp tests/.env.example tests/.env
//!    ```
//!
//! 2. Edit `tests/.env` and add your API key
//!
//! 3. Run the tests:
//!    ```bash
//!    cargo test --test sms_activate_api -- --ignored
//!    ```
//!
//! Alternatively, pass the API key directly:
//! ```bash
//! SMS_ACTIVATE_API_KEY=your_key cargo test --test sms_activate_api -- --ignored
//! ```
//!
//! **WARNING**: These tests will consume API credits!

use sms_solvers::sms_activate::{
    Service, SmsActivateClient, SmsActivateError, SmsActivateProvider, SmsCountryExt,
};
use sms_solvers::{
    CountryCode, Provider, RetryConfig, SmsRetryableProvider, SmsSolverService,
    SmsSolverServiceConfig, SmsSolverServiceTrait,
};
use std::env;
use std::time::Duration;

/// Service to use for testing.
const TEST_SERVICE: Service = Service::InstagramThreads;

/// Helper to check if error is "no numbers available".
fn is_no_numbers_error(err: &SmsActivateError) -> bool {
    use sms_solvers::sms_activate::SmsActivateError as E;
    matches!(err, E::Service(e) if e.code.code_name() == "NO_NUMBERS")
}

/// Helper to check if error is authentication related.
fn is_auth_error(err: &SmsActivateError) -> bool {
    matches!(err, E::Service(e) if e.code.code_name() == "BAD_KEY" || e.code.code_name() == "BAD_ACTION")
}

/// Get API key from environment or .env file.
fn get_api_key() -> String {
    // Try to load from tests/.env file
    dotenvy::dotenv().ok();

    env::var("SMS_ACTIVATE_API_KEY").expect(
        "SMS_ACTIVATE_API_KEY environment variable must be set.\n\
         Either:\n\
         1. Copy tests/.env.example to tests/.env and add your API key\n\
         2. Run with: SMS_ACTIVATE_API_KEY=your_key cargo test --test sms_activate_api -- --ignored",
    )
}

/// Create a test client with the API key from environment.
fn create_client() -> SmsActivateClient {
    let api_key = get_api_key();
    SmsActivateClient::with_api_key(&api_key).expect("Failed to create client")
}

/// Create a test provider.
fn create_provider() -> SmsActivateProvider {
    SmsActivateProvider::new(create_client())
}

/// Create a test service with default config.
fn create_service() -> SmsSolverService<SmsActivateProvider> {
    let provider = create_provider();
    let config = SmsSolverServiceConfig::default()
        .with_timeout(Duration::from_secs(60))
        .with_poll_interval(Duration::from_secs(5));
    SmsSolverService::new(provider, config)
}

/// Create a service with retry wrapper.
fn create_retryable_service() -> SmsSolverService<SmsRetryableProvider<SmsActivateProvider>> {
    let provider = create_provider();
    let retry_config = RetryConfig::default()
        .with_min_delay(Duration::from_millis(500))
        .with_max_delay(Duration::from_secs(5))
        .with_max_retries(3);
    let retryable = SmsRetryableProvider::with_config(provider, retry_config);

    let config = SmsSolverServiceConfig::default()
        .with_timeout(Duration::from_secs(60))
        .with_poll_interval(Duration::from_secs(5));
    SmsSolverService::new(retryable, config)
}

// =============================================================================
// Client Tests
// =============================================================================

/// Test that the client can be created with valid API key.
#[test]
#[ignore = "requires API key"]
fn test_client_creation() {
    let _client = create_client();
}

// =============================================================================
// Provider Tests - Get Phone Number
// =============================================================================

/// Test getting a phone number for Ukraine.
#[tokio::test]
#[ignore = "requires API key and consumes credits"]
async fn test_get_phone_number_ukraine() {
    let provider = create_provider();

    let result = provider
        .get_phone_number(CountryCode::UKR, TEST_SERVICE)
        .await;

    match result {
        Ok((task_id, full_number)) => {
            println!("Successfully got number:");
            println!("  Task ID: {}", task_id);
            println!("  Full number: {}", full_number);

            // Verify task_id is not empty
            assert!(!task_id.as_ref().is_empty(), "Task ID should not be empty");

            // Verify number starts with Ukraine dial code
            assert!(
                full_number.as_ref().starts_with("380"),
                "Ukraine number should start with 380, got: {}",
                full_number
            );

            // Cancel the activation to not waste credits
            let cancel_result = provider.cancel_activation(&task_id).await;
            println!("  Cancelled: {:?}", cancel_result.is_ok());
        }
        Err(ref e) if is_no_numbers_error(e) => {
            println!("No numbers available for Ukraine (this is expected sometimes)");
        }
        Err(e) => {
            panic!("Unexpected error: {:?}", e);
        }
    }
}

/// Test getting a phone number for USA.
#[tokio::test]
#[ignore = "requires API key and consumes credits"]
async fn test_get_phone_number_usa() {
    let provider = create_provider();

    let result = provider
        .get_phone_number(CountryCode::USA, TEST_SERVICE)
        .await;

    match result {
        Ok((task_id, full_number)) => {
            println!("Successfully got USA number:");
            println!("  Task ID: {}", task_id);
            println!("  Full number: {}", full_number);

            // USA numbers start with 1
            assert!(
                full_number.as_ref().starts_with("1"),
                "USA number should start with 1, got: {}",
                full_number
            );

            // Cancel the activation
            let _ = provider.cancel_activation(&task_id).await;
        }
        Err(ref e) if is_no_numbers_error(e) => {
            println!("No numbers available for USA");
        }
        Err(e) => {
            panic!("Unexpected error: {:?}", e);
        }
    }
}

/// Test getting a phone number for Germany.
#[tokio::test]
#[ignore = "requires API key and consumes credits"]
async fn test_get_phone_number_germany() {
    let provider = create_provider();

    let result = provider
        .get_phone_number(CountryCode::DEU, TEST_SERVICE)
        .await;

    match result {
        Ok((task_id, full_number)) => {
            println!("Successfully got Germany number:");
            println!("  Task ID: {}", task_id);
            println!("  Full number: {}", full_number);

            // Germany numbers start with 49
            assert!(
                full_number.as_ref().starts_with("49"),
                "Germany number should start with 49, got: {}",
                full_number
            );

            // Cancel the activation
            let _ = provider.cancel_activation(&task_id).await;
        }
        Err(ref e) if is_no_numbers_error(e) => {
            println!("No numbers available for Germany");
        }
        Err(e) => {
            panic!("Unexpected error: {:?}", e);
        }
    }
}

// =============================================================================
// Service Tests
// =============================================================================

/// Test the full service flow: get number -> (optionally wait for code) -> cancel.
#[tokio::test]
#[ignore = "requires API key and consumes credits"]
async fn test_service_get_number() {
    let service = create_service();

    let result = service.get_number(CountryCode::UKR, TEST_SERVICE).await;

    match result {
        Ok(sms_result) => {
            println!("Service successfully got number:");
            println!("  Task ID: {}", sms_result.task_id);
            println!("  Full number: {}", sms_result.full_number);
            println!("  Dial code: {}", sms_result.dial_code);
            println!("  Number: {}", sms_result.number);
            println!(
                "  Country: {} ({})",
                sms_result.country.name(),
                sms_result.country.alpha2()
            );

            // Verify the parsed components
            assert_eq!(sms_result.dial_code.as_str(), "380");
            assert_eq!(sms_result.country, CountryCode::UKR);

            // Verify full_number = dial_code + number
            let expected_full = format!("{}{}", sms_result.dial_code, sms_result.number);
            assert_eq!(sms_result.full_number.as_ref(), &expected_full);

            // Cancel via provider (service exposes provider())
            let provider = service.provider();
            let _ = provider.cancel_activation(&sms_result.task_id).await;
        }
        Err(e) => {
            // NoNumbersAvailable is acceptable
            let err_str = format!("{:?}", e);
            if err_str.contains("NoNumbers") {
                println!("No numbers available (expected)");
            } else {
                panic!("Unexpected error: {:?}", e);
            }
        }
    }
}

/// Test service with retry wrapper.
#[tokio::test]
#[ignore = "requires API key and consumes credits"]
async fn test_service_with_retry() {
    let service = create_retryable_service();

    let result = service.get_number(CountryCode::GBR, TEST_SERVICE).await;

    match result {
        Ok(sms_result) => {
            println!("Got UK number with retry service:");
            println!("  Full number: +{}", sms_result.full_number);

            assert!(
                sms_result.full_number.as_ref().starts_with("44"),
                "UK number should start with 44"
            );

            // For retryable service, we need to create a new provider to cancel
            let cancel_provider = create_provider();
            let _ = cancel_provider.cancel_activation(&sms_result.task_id).await;
        }
        Err(e) => {
            let err_str = format!("{:?}", e);
            if err_str.contains("NoNumbers") {
                println!("No numbers available for UK");
            } else {
                panic!("Unexpected error: {:?}", e);
            }
        }
    }
}

// =============================================================================
// Country Mapping Tests with Real API
// =============================================================================

/// Test that country SMS IDs work with the real API.
#[tokio::test]
#[ignore = "requires API key and consumes credits"]
async fn test_country_mapping_with_api() {
    let provider = create_provider();

    // Test a few countries with known SMS IDs
    let test_countries = [
        (CountryCode::UKR, "380"), // ID: 1
        (CountryCode::DEU, "49"),  // ID: 43
        (CountryCode::FRA, "33"),  // ID: 78
    ];

    for (country, expected_prefix) in test_countries {
        // Verify SMS ID exists
        let sms_id = country.sms_id();
        assert!(
            sms_id.is_ok(),
            "Country {} should have SMS ID",
            country.name()
        );
        println!("{}: SMS ID = {}", country.name(), sms_id.unwrap());

        // Try to get a number (may fail if no numbers available)
        let result = provider.get_phone_number(country, TEST_SERVICE).await;
        match result {
            Ok((task_id, full_number)) => {
                println!("  Got number: {}", full_number);
                assert!(
                    full_number.as_ref().starts_with(expected_prefix),
                    "Number should start with {}, got: {}",
                    expected_prefix,
                    full_number
                );
                let _ = provider.cancel_activation(&task_id).await;
            }
            Err(ref e) if is_no_numbers_error(e) => {
                println!("  No numbers available");
            }
            Err(e) => {
                println!("  Error: {:?}", e);
            }
        }
    }
}

// =============================================================================
// Error Handling Tests
// =============================================================================

/// Test error handling for invalid API key.
#[tokio::test]
#[ignore = "tests error handling"]
async fn test_invalid_api_key() {
    let client = SmsActivateClient::with_api_key("invalid_key_12345").unwrap();
    let provider = SmsActivateProvider::new(client);

    let result = provider
        .get_phone_number(CountryCode::UKR, TEST_SERVICE)
        .await;

    assert!(result.is_err(), "Should fail with invalid API key");

    let err = result.unwrap_err();
    println!("Error with invalid API key: {:?}", err);

    // Should be an authentication error or similar
    if is_auth_error(&err) {
        println!("Got expected auth error");
    } else {
        // Other errors might also occur depending on the API response
        println!("Got error (may be acceptable): {:?}", err);
    }
}

/// Test getting SMS status (without waiting for actual SMS).
#[tokio::test]
#[ignore = "requires API key and consumes credits"]
async fn test_get_sms_status() {
    let provider = create_provider();

    // Get a number
    let result = provider
        .get_phone_number(CountryCode::UKR, TEST_SERVICE)
        .await;

    if let Ok((task_id, full_number)) = result {
        println!("Got number: {} (task: {})", full_number, task_id);

        // Check SMS status (should be None since no SMS was sent)
        let sms_result = provider.get_sms_code(&task_id).await;

        match sms_result {
            Ok(None) => {
                println!("No SMS received yet (expected)");
            }
            Ok(Some(code)) => {
                println!("Unexpectedly received SMS: {}", code);
            }
            Err(e) => {
                println!("Error checking SMS: {:?}", e);
            }
        }

        // Clean up
        let _ = provider.cancel_activation(&task_id).await;
    }
}

// =============================================================================
// Performance / Load Tests
// =============================================================================

/// Test multiple sequential number requests.
#[tokio::test]
#[ignore = "requires API key and consumes significant credits"]
async fn test_multiple_number_requests() {
    let provider = create_provider();
    let mut successes = 0;
    let mut failures = 0;
    let mut task_ids = Vec::new();

    // Request 3 numbers
    for i in 0..3 {
        println!("Request {}/3...", i + 1);

        let result = provider
            .get_phone_number(CountryCode::UKR, TEST_SERVICE)
            .await;

        match result {
            Ok((task_id, full_number)) => {
                println!("  Got: {}", full_number);
                task_ids.push(task_id);
                successes += 1;
            }
            Err(e) => {
                println!("  Failed: {:?}", e);
                failures += 1;
            }
        }

        // Small delay between requests
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    println!("\nResults: {} successes, {} failures", successes, failures);

    // Clean up all activations
    for task_id in task_ids {
        let _ = provider.cancel_activation(&task_id).await;
    }
}
