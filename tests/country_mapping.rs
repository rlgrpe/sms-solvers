//! Integration tests for country code mapping functionality.
//!
//! These tests verify that the country mapping works correctly across
//! the Hero SMS countries JSON and the dial codes JSON.

use keshvar::CountryIterator;
use sms_solvers::hero_sms::SmsCountryExt;
use sms_solvers::{Alpha2, Country, DialCode};

/// Test that popular countries have valid Hero SMS mappings.
#[test]
fn test_popular_countries_have_sms_mapping() {
    let popular_countries = [
        Alpha2::US,
        Alpha2::GB,
        Alpha2::UA,
        Alpha2::DE,
        Alpha2::FR,
        Alpha2::IT,
        Alpha2::ES,
        Alpha2::PL,
        Alpha2::NL,
        Alpha2::CN,
        Alpha2::IN,
        Alpha2::BR,
        Alpha2::ID,
        Alpha2::TR,
        Alpha2::JP,
        Alpha2::AU,
        Alpha2::CA,
        Alpha2::MX,
        Alpha2::AR,
        // Note: KR (South Korea) may not be in SMS Activate countries list
    ];

    for alpha2 in popular_countries {
        let country = alpha2.to_country();
        let result = country.sms_id();
        assert!(
            result.is_ok(),
            "Popular country {} ({:?}) should have SMS mapping, but got error: {:?}",
            country.iso_short_name(),
            country.alpha2(),
            result.err()
        );
    }
}

/// Test that we can do round-trip conversions: Country -> SMS ID -> Country.
#[test]
fn test_country_sms_id_round_trip() {
    let test_countries = [Alpha2::UA, Alpha2::GB, Alpha2::DE, Alpha2::FR, Alpha2::PL];

    for alpha2 in test_countries {
        let original = alpha2.to_country();
        let sms_id = original
            .sms_id()
            .unwrap_or_else(|_| panic!("Failed to get SMS ID for {}", original.iso_short_name()));

        let converted = Country::from_sms_id(sms_id)
            .unwrap_or_else(|_| panic!("Failed to convert SMS ID {} back to Country", sms_id));

        assert_eq!(
            original.alpha2(),
            converted.alpha2(),
            "Round-trip conversion failed for {} (SMS ID: {})",
            original.iso_short_name(),
            sms_id
        );
    }
}

/// Test specific known Hero SMS IDs.
#[test]
fn test_known_hero_sms_ids() {
    // These IDs are from hero_sms_countries.json
    let known_mappings = [
        (1, Alpha2::UA),   // "1": "Ukraine"
        (16, Alpha2::GB),  // "16": "United Kingdom"
        (43, Alpha2::DE),  // "43": "Germany"
        (78, Alpha2::FR),  // "78": "France"
        (187, Alpha2::US), // "187": "USA"
    ];

    for (sms_id, expected_alpha2) in known_mappings {
        let result = Country::from_sms_id(sms_id);
        assert!(result.is_ok(), "SMS ID {} should map to a country", sms_id);
        assert_eq!(
            result.unwrap().alpha2(),
            expected_alpha2,
            "SMS ID {} should map to {:?}",
            sms_id,
            expected_alpha2
        );
    }
}

/// Test that unknown SMS IDs return an error.
#[test]
fn test_unknown_sms_id_returns_error() {
    let unknown_ids: [u16; 3] = [9999, 50000, 60000];

    for id in unknown_ids {
        let result = Country::from_sms_id(id);
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
        Alpha2::AQ, // Antarctica
        Alpha2::BV, // Bouvet Island
    ];

    for alpha2 in unsupported {
        let country = alpha2.to_country();
        let result = country.sms_id();
        assert!(
            result.is_err(),
            "Country {} should not have SMS mapping",
            country.iso_short_name()
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
    for country in CountryIterator::new() {
        if country.sms_id().is_ok() {
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
    // These countries have name differences between Hero SMS and ISO
    let override_countries = [
        (187, Alpha2::US), // "USA" vs "United States of America"
        (16, Alpha2::GB),  // "United Kingdom" matches
        (95, Alpha2::AE),  // "UAE" vs "United Arab Emirates"
        (63, Alpha2::CZ),  // "Czech" vs "Czechia"
        (27, Alpha2::CI),  // "Ivory Coast" vs "Côte d'Ivoire"
    ];

    for (sms_id, expected_alpha2) in override_countries {
        let result = Country::from_sms_id(sms_id);
        assert!(
            result.is_ok(),
            "Override country {:?} (SMS ID {}) should be mapped",
            expected_alpha2,
            sms_id
        );
        assert_eq!(
            result.unwrap().alpha2(),
            expected_alpha2,
            "SMS ID {} should map to {:?}",
            sms_id,
            expected_alpha2
        );
    }
}
