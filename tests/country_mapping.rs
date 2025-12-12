//! Integration tests for country code mapping functionality.
//!
//! These tests verify that the country mapping works correctly across
//! the SMS Activate countries JSON and the dial codes JSON.

use isocountry::CountryCode;
use sms_solvers::DialCode;
use sms_solvers::providers::sms_activate::SmsCountryExt;

/// Test that popular countries have valid SMS Activate mappings.
#[test]
fn test_popular_countries_have_sms_mapping() {
    let popular_countries = [
        CountryCode::USA,
        CountryCode::GBR,
        CountryCode::UKR,
        CountryCode::DEU,
        CountryCode::FRA,
        CountryCode::ITA,
        CountryCode::ESP,
        CountryCode::POL,
        CountryCode::NLD,
        CountryCode::CHN,
        CountryCode::IND,
        CountryCode::BRA,
        CountryCode::IDN,
        CountryCode::TUR,
        CountryCode::JPN,
        CountryCode::AUS,
        CountryCode::CAN,
        CountryCode::MEX,
        CountryCode::ARG,
        // Note: KOR (South Korea) is not in SMS Activate countries list
    ];

    for country in popular_countries {
        let result = country.sms_id();
        assert!(
            result.is_ok(),
            "Popular country {} ({}) should have SMS mapping, but got error: {:?}",
            country.name(),
            country.alpha2(),
            result.err()
        );
    }
}

/// Test that we can do round-trip conversions: CountryCode -> SMS ID -> CountryCode.
#[test]
fn test_country_sms_id_round_trip() {
    let test_countries = [
        CountryCode::UKR,
        CountryCode::GBR,
        CountryCode::DEU,
        CountryCode::FRA,
        CountryCode::POL,
    ];

    for original in test_countries {
        let sms_id = original
            .sms_id()
            .unwrap_or_else(|_| panic!("Failed to get SMS ID for {}", original.name()));

        let converted = CountryCode::from_sms_id(sms_id)
            .unwrap_or_else(|_| panic!("Failed to convert SMS ID {} back to CountryCode", sms_id));

        assert_eq!(
            original,
            converted,
            "Round-trip conversion failed for {} (SMS ID: {})",
            original.name(),
            sms_id
        );
    }
}

/// Test specific known SMS Activate IDs.
#[test]
fn test_known_sms_activate_ids() {
    // These IDs are from sms_activate_countries.json
    let known_mappings = [
        (1, CountryCode::UKR),   // "1": "Ukraine"
        (16, CountryCode::GBR),  // "16": "United Kingdom"
        (43, CountryCode::DEU),  // "43": "Germany"
        (78, CountryCode::FRA),  // "78": "France"
        (187, CountryCode::USA), // "187": "USA"
    ];

    for (sms_id, expected_country) in known_mappings {
        let result = CountryCode::from_sms_id(sms_id);
        assert!(result.is_ok(), "SMS ID {} should map to a country", sms_id);
        assert_eq!(
            result.unwrap(),
            expected_country,
            "SMS ID {} should map to {}",
            sms_id,
            expected_country.name()
        );
    }
}

/// Test that unknown SMS IDs return an error.
#[test]
fn test_unknown_sms_id_returns_error() {
    let unknown_ids: [u16; 3] = [9999, 50000, 60000];

    for id in unknown_ids {
        let result = CountryCode::from_sms_id(id);
        assert!(
            result.is_err(),
            "Unknown SMS ID {} should return an error",
            id
        );
    }
}

/// Test that countries without SMS service return an error.
#[test]
fn test_unsupported_countries_return_error() {
    // Antarctica and similar territories don't have SMS service
    let unsupported = [
        CountryCode::ATA, // Antarctica
        CountryCode::BVT, // Bouvet Island
    ];

    for country in unsupported {
        let result = country.sms_id();
        assert!(
            result.is_err(),
            "Country {} should not have SMS mapping",
            country.name()
        );
    }
}

/// Test dial code creation from various formats.
#[test]
fn test_dial_code_creation() {
    // Valid dial codes
    let valid_cases = [
        ("1", "1"),
        ("+1", "1"),
        ("44", "44"),
        ("+44", "44"),
        ("380", "380"),
        ("+380", "380"),
        ("  +49  ", "49"), // with whitespace
    ];

    for (input, expected) in valid_cases {
        let result = DialCode::new(input);
        assert!(result.is_ok(), "Dial code '{}' should be valid", input);
        assert_eq!(
            result.unwrap().as_str(),
            expected,
            "Dial code '{}' should normalize to '{}'",
            input,
            expected
        );
    }
}

/// Test invalid dial codes.
#[test]
fn test_invalid_dial_codes() {
    let invalid_cases = [
        "",      // empty
        "+",     // only plus
        "abc",   // letters
        "12a34", // mixed
        "+abc",  // plus with letters
    ];

    for input in invalid_cases {
        let result = DialCode::new(input);
        assert!(result.is_err(), "Dial code '{}' should be invalid", input);
    }
}

/// Test that the total number of mapped countries is reasonable.
#[test]
fn test_reasonable_country_count() {
    // Count how many countries have SMS mappings
    let mut mapped_count = 0;
    for cc in CountryCode::iter() {
        if cc.sms_id().is_ok() {
            mapped_count += 1;
        }
    }

    // We should have at least 100 countries mapped
    assert!(
        mapped_count >= 100,
        "Expected at least 100 mapped countries, but got {}",
        mapped_count
    );

    // But not more than 300 (sanity check)
    assert!(
        mapped_count <= 300,
        "Expected at most 300 mapped countries, but got {}",
        mapped_count
    );

    println!("Total mapped countries: {}", mapped_count);
}

/// Test name override countries are correctly mapped.
#[test]
fn test_name_override_countries() {
    // These countries have name differences between SMS-Activate and ISO
    let override_countries = [
        (187, CountryCode::USA), // "USA" vs "United States of America"
        (16, CountryCode::GBR),  // "United Kingdom" matches
        (95, CountryCode::ARE),  // "UAE" vs "United Arab Emirates"
        (63, CountryCode::CZE),  // "Czech" vs "Czechia"
        (27, CountryCode::CIV),  // "Ivory Coast" vs "Côte d'Ivoire"
    ];

    for (sms_id, expected_country) in override_countries {
        let result = CountryCode::from_sms_id(sms_id);
        assert!(
            result.is_ok(),
            "Override country {} (SMS ID {}) should be mapped",
            expected_country.name(),
            sms_id
        );
        assert_eq!(
            result.unwrap(),
            expected_country,
            "SMS ID {} should map to {}",
            sms_id,
            expected_country.name()
        );
    }
}
