//! Example demonstrating SMS.online country code mapping.
//!
//! This example shows how to convert between ISO country codes
//! and SMS.online numeric IDs.
//!
//! # Running
//!
//! ```bash
//! cargo run --example sms_online_country_mapping
//! ```

use sms_solvers::sms_online::{SmsOnline, SmsOnlineCountryExt, SmsOnlineProvider};
use sms_solvers::{Alpha2, Country, DialCode, Provider};

fn main() {
    println!("=== SMS.online Country Code Mapping Demo ===\n");

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

    println!("ISO Code -> SMS.online ID mapping:\n");
    println!("{:<20} {:<10} {:<15}", "Country", "ISO", "SMS.online ID");
    println!("{}", "-".repeat(45));

    for alpha2 in countries {
        let country = alpha2.to_country();
        match country.sms_online_id() {
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

    let sms_ids = [1, 16, 43, 78, 182, 187];

    println!("SMS.online ID -> ISO Country:\n");
    println!("{:<15} {:<20} {:<10}", "SMS.online ID", "Country", "ISO");
    println!("{}", "-".repeat(45));

    for sms_id in sms_ids {
        match Country::from_sms_online_id(sms_id) {
            Ok(country) => {
                println!(
                    "{:<15} {:<20} {:?}",
                    sms_id,
                    country.iso_short_name(),
                    country.alpha2()
                );
            }
            Err(e) => {
                println!("{:<15} Error: {}", sms_id, e);
            }
        }
    }

    println!("\n=== Provider Blacklist Demo ===\n");

    use std::collections::HashSet;

    let client = SmsOnline::with_api_key("demo_key").unwrap();
    let blacklist: HashSet<DialCode> = ["33", "49"]
        .iter()
        .map(|s| DialCode::new(s).unwrap())
        .collect();
    let provider = SmsOnlineProvider::with_blacklist(client, blacklist);

    let test_codes = ["1", "33", "44", "49", "380"];
    println!("Blacklisted dial codes: 33 (France), 49 (Germany)\n");

    for code in test_codes {
        let dial_code = DialCode::new(code).unwrap();
        let supported = provider.is_dial_code_supported(&dial_code);
        let status = if supported { "supported" } else { "BLOCKED" };
        println!("  Dial code +{}: {}", code, status);
    }
}
