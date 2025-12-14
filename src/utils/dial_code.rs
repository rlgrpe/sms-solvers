//! Dial code mapping utilities.

use crate::types::DialCode;
use isocountry::CountryCode;
use once_cell::sync::Lazy;
use std::collections::HashMap;

/// Raw JSON entry for country dial code data.
#[derive(Debug, serde::Deserialize)]
struct CountryDialCodeEntry {
    #[allow(dead_code)]
    name: String,
    #[allow(dead_code)]
    flag: String,
    code: String,
    dial_code: String,
}

/// Dial codes JSON embedded at compile time.
static DIAL_CODES_JSON: &str = include_str!("../../assets/countries_with_dial_code.json");

/// Mapping from ISO alpha-2 code to dial code string.
static ALPHA2_TO_DIAL_CODE: Lazy<HashMap<String, String>> = Lazy::new(|| {
    let entries: Vec<CountryDialCodeEntry> =
        serde_json::from_str(DIAL_CODES_JSON).expect("countries_with_dial_code.json is invalid");

    let mut map = HashMap::with_capacity(entries.len());
    for entry in entries {
        map.insert(entry.code.to_uppercase(), entry.dial_code);
    }
    map
});

/// Convert a country code to its dial code.
pub(crate) fn country_to_dial_code(country: CountryCode) -> Option<DialCode> {
    let alpha2 = country.alpha2();
    let dial_code_str = ALPHA2_TO_DIAL_CODE.get(alpha2)?;
    DialCode::new(dial_code_str).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_country_to_dial_code() {
        assert_eq!(
            country_to_dial_code(CountryCode::USA).map(|dc| dc.to_string()),
            Some("1".to_string())
        );
        assert_eq!(
            country_to_dial_code(CountryCode::UKR).map(|dc| dc.to_string()),
            Some("380".to_string())
        );
        assert_eq!(
            country_to_dial_code(CountryCode::GBR).map(|dc| dc.to_string()),
            Some("44".to_string())
        );
        assert_eq!(
            country_to_dial_code(CountryCode::TUR).map(|dc| dc.to_string()),
            Some("90".to_string())
        );
    }
}
