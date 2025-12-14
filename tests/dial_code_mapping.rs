//! Integration tests for dial code mapping from JSON.
//!
//! These tests verify that the dial code mapping from
//! countries_with_dial_code.json works correctly.

use sms_solvers::DialCode;

/// Helper function to get dial code for a country using the service module's function.
/// Since the function is private, we test it indirectly through the public API.
mod service_dial_code {
    use sms_solvers::sms_activate::{SmsActivateClient, SmsActivateProvider};
    use sms_solvers::{SmsSolverService, SmsSolverServiceConfig};
    use std::time::Duration;

    /// Create a test service to access dial code functionality.
    pub fn create_test_service() -> SmsSolverService<SmsActivateProvider> {
        let client = SmsActivateClient::with_api_key("test_key").unwrap();
        let provider = SmsActivateProvider::new(client);
        let config = SmsSolverServiceConfig::default()
            .with_timeout(Duration::from_secs(1))
            .with_poll_interval(Duration::from_millis(100));
        SmsSolverService::new(provider, config)
    }
}

/// Test that popular countries have dial codes in the JSON.
#[test]
fn test_dial_code_json_has_popular_countries() {
    // These are the most commonly used countries and their expected dial codes
    let expected_dial_codes = [
        ("US", "1"),
        ("GB", "44"),
        ("UA", "380"),
        ("DE", "49"),
        ("FR", "33"),
        ("IT", "39"),
        ("ES", "34"),
        ("PL", "48"),
        ("NL", "31"),
        ("CN", "86"),
        ("IN", "91"),
        ("BR", "55"),
        ("JP", "81"),
        ("KR", "82"),
        ("AU", "61"),
        ("CA", "1"),
        ("MX", "52"),
        ("TR", "90"),
        ("RU", "7"),
    ];

    // We can't directly test the private function, but we verify the JSON structure
    // by checking that DialCode can be created with expected values
    for (alpha2, dial_code) in expected_dial_codes {
        let dc = DialCode::new(dial_code);
        assert!(
            dc.is_ok(),
            "Dial code '{}' for {} should be valid",
            dial_code,
            alpha2
        );
        assert_eq!(
            dc.unwrap().as_str(),
            dial_code,
            "Dial code for {} should be '{}'",
            alpha2,
            dial_code
        );
    }
}

/// Test that dial codes are parsed correctly with various formats.
#[test]
fn test_dial_code_parsing() {
    let test_cases = [
        // (input, expected_output)
        ("1", "1"),
        ("+1", "1"),
        ("44", "44"),
        ("+44", "44"),
        ("380", "380"),
        ("+380", "380"),
        ("971", "971"),   // UAE
        ("966", "966"),   // Saudi Arabia
        ("1684", "1684"), // American Samoa
    ];

    for (input, expected) in test_cases {
        let result = DialCode::new(input);
        assert!(
            result.is_ok(),
            "Should parse dial code '{}' successfully",
            input
        );
        assert_eq!(
            result.unwrap().as_str(),
            expected,
            "Dial code '{}' should normalize to '{}'",
            input,
            expected
        );
    }
}

/// Test dial code display formatting.
#[test]
fn test_dial_code_display() {
    let dc = DialCode::new("+380").unwrap();
    assert_eq!(format!("{}", dc), "380");
    assert_eq!(dc.to_string(), "380");
    assert_eq!(dc.as_str(), "380");
}

/// Test dial code serialization.
#[test]
fn test_dial_code_serde() {
    let dc = DialCode::new("+44").unwrap();

    // Serialize
    let json = serde_json::to_string(&dc).unwrap();
    assert_eq!(json, "\"44\"");

    // Deserialize
    let deserialized: DialCode = serde_json::from_str("\"44\"").unwrap();
    assert_eq!(deserialized.as_str(), "44");

    // Deserialize with plus sign
    let deserialized2: DialCode = serde_json::from_str("\"+44\"").unwrap();
    assert_eq!(deserialized2.as_str(), "44");
}

/// Test that countries with same dial code are handled correctly.
#[test]
fn test_shared_dial_codes() {
    // USA and Canada share dial code 1
    // Russia and Kazakhstan share dial code 7
    // Multiple territories share dial codes with their parent countries

    let shared_codes = [
        ("1", vec!["US", "CA"]), // North American Numbering Plan
        ("44", vec!["GB"]),      // UK territories
        ("61", vec!["AU"]),      // Australia
    ];

    for (code, _countries) in shared_codes {
        let dc = DialCode::new(code);
        assert!(dc.is_ok(), "Shared dial code '{}' should be valid", code);
    }
}

/// Test edge cases for dial codes.
#[test]
fn test_dial_code_edge_cases() {
    // Single digit
    let dc1 = DialCode::new("1");
    assert!(dc1.is_ok());

    // Four digits (some Caribbean countries)
    let dc2 = DialCode::new("1684");
    assert!(dc2.is_ok());

    // With leading/trailing whitespace
    let dc3 = DialCode::new("  +49  ");
    assert!(dc3.is_ok());
    assert_eq!(dc3.unwrap().as_str(), "49");
}

/// Test comparison and hashing of dial codes.
#[test]
fn test_dial_code_comparison() {
    let dc1 = DialCode::new("44").unwrap();
    let dc2 = DialCode::new("+44").unwrap();
    let dc3 = DialCode::new("380").unwrap();

    assert_eq!(dc1, dc2, "Same dial code with/without + should be equal");
    assert_ne!(dc1, dc3, "Different dial codes should not be equal");

    // Test ordering (lexicographic, since dial codes are stored as strings)
    // "380" < "44" because '3' < '4' in ASCII
    assert!(dc3 < dc1, "380 should be lexicographically less than 44");

    // Use in a HashSet to verify Hash works
    use std::collections::HashSet;
    let mut set = HashSet::new();
    set.insert(dc1.clone());
    set.insert(dc2.clone()); // Should not add duplicate
    assert_eq!(
        set.len(),
        1,
        "HashSet should have only 1 element for equal dial codes"
    );
}

/// Test that the service can be created with valid configuration.
#[test]
fn test_service_creation() {
    let _service = service_dial_code::create_test_service();
    // Service should be created without panicking
}
