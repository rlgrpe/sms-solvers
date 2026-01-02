//! Example demonstrating country code mapping functionality.
//!
//! This example shows how to use the country code mapping to convert
//! between ISO country codes and SMS Activate IDs.
//!
//! # Running
//!
//! ```bash
//! cargo run --example country_mapping
//! ```

use sms_solvers::hero_sms::{HeroSms, HeroSmsProvider, SmsCountryExt};
use sms_solvers::{Alpha2, Country, DialCode, Provider};

fn main() {
    println!("=== Country Code Mapping Demo ===\n");

    // List of countries to demonstrate
    let countries = [
        Alpha2::US,
        Alpha2::GB,
        Alpha2::UA,
        Alpha2::DE,
        Alpha2::FR,
        Alpha2::JP,
        Alpha2::BR,
        Alpha2::IN,
        Alpha2::CN,
        Alpha2::TR,
    ];

    println!("ISO Code -> SMS Activate ID mapping:\n");
    println!("{:<20} {:<10} {:<15}", "Country", "ISO", "SMS ID");
    println!("{}", "-".repeat(45));

    for alpha2 in countries {
        let country = alpha2.to_country();
        match country.sms_id() {
            Ok(sms_id) => {
                println!(
                    "{:<20} {:?}        {:<15}",
                    country.iso_short_name(),
                    country.alpha2(),
                    sms_id
                );
            }
            Err(e) => {
                println!(
                    "{:<20} {:?}        Error: {}",
                    country.iso_short_name(),
                    country.alpha2(),
                    e
                );
            }
        }
    }

    println!("\n=== Reverse Mapping Demo ===\n");

    // Demonstrate reverse mapping (SMS ID -> ISO Country)
    let sms_ids = [1, 16, 43, 78, 182, 187];

    println!("SMS Activate ID -> ISO Country:\n");
    println!("{:<10} {:<20} {:<10}", "SMS ID", "Country", "ISO");
    println!("{}", "-".repeat(40));

    for sms_id in sms_ids {
        match Country::from_sms_id(sms_id) {
            Ok(country) => {
                println!(
                    "{:<10} {:<20} {:?}",
                    sms_id,
                    country.iso_short_name(),
                    country.alpha2()
                );
            }
            Err(e) => {
                println!("{:<10} Error: {}", sms_id, e);
            }
        }
    }

    println!("\n=== Dial Code Demo ===\n");

    // Demonstrate dial code creation and validation
    let dial_codes = ["1", "+44", "380", "+49", "invalid"];

    println!("Dial Code Validation:\n");
    for code in dial_codes {
        match DialCode::new(code) {
            Ok(dc) => println!("  '{}' -> valid: {}", code, dc),
            Err(e) => println!("  '{}' -> invalid: {}", code, e),
        }
    }

    println!("\n=== Provider Blacklist Demo ===\n");

    // Demonstrate dial code blacklisting
    use std::collections::HashSet;

    let client = HeroSms::with_api_key("demo_key").unwrap();

    // Create provider with blacklisted dial codes
    let blacklist: HashSet<DialCode> = ["33", "49"]
        .iter()
        .map(|s| DialCode::new(s).unwrap())
        .collect();
    let provider = HeroSmsProvider::with_blacklist(client, blacklist);

    let test_codes = ["1", "33", "44", "49", "380"];
    println!("Blacklisted dial codes: 33 (France), 49 (Germany)\n");

    for code in test_codes {
        let dial_code = DialCode::new(code).unwrap();
        let supported = provider.is_dial_code_supported(&dial_code);
        let status = if supported { "supported" } else { "BLOCKED" };
        println!("  Dial code +{}: {}", code, status);
    }
}
