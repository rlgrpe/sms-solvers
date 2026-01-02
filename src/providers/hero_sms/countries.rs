//! Country code mapping for Hero SMS API.

use keshvar::{Alpha2, Country, CountryIterator};
use once_cell::sync::Lazy;
use serde_json::Value;
use std::collections::HashMap;
use thiserror::Error;

/// Error when mapping country codes.
#[derive(Debug, Clone, Error)]
pub enum CountryMapError {
    /// Unknown Hero SMS ID.
    #[error("Unknown country for Hero SMS id {id}")]
    UnknownSmsId { id: u16 },
    /// No Hero SMS mapping for country.
    #[error("No Hero SMS mapping for country {}", country.iso_short_name())]
    NoSmsMapping { country: Box<Country> },
}

/// Hero SMS countries JSON embedded at compile time.
static COUNTRIES_JSON: &str = include_str!("../../../assets/hero_sms_countries.json");

/// Name normalization for stable comparison.
/// Converts to lowercase and removes punctuation/extra whitespace.
fn norm(s: &str) -> String {
    const PUNCT: &[char] = &[
        '\'', '"', '`', ',', '.', '-', '_', '(', ')', '\u{2018}',
        '\u{2019}', // curly single quotes ' '
        '\u{00B4}', // acute accent ´
    ];
    s.to_ascii_lowercase()
        .replace(PUNCT, "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Overrides: normalized SMS name -> ISO alpha-2 code
/// Used where Hero SMS names differ significantly from ISO standard names.
static NAME_OVERRIDES: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    HashMap::from([
        // Primary mappings
        ("usa", "US"),
        ("united states", "US"),
        ("united kingdom", "GB"),
        ("uae", "AE"),
        // Name differences
        ("vietnam", "VN"),
        ("south korea", "KR"),
        ("north korea", "KP"),
        ("dr congo", "CD"),
        ("ivory coast", "CI"),
        ("czech", "CZ"),
        ("moldova", "MD"),
        ("laos", "LA"),
        ("syria", "SY"),
        ("iran", "IR"),
        ("venezuela", "VE"),
        ("tanzania", "TZ"),
        ("bolivia", "BO"),
        ("bosnia", "BA"),
        ("brunei", "BN"),
        ("palestine", "PS"),
        ("taiwan", "TW"),
        // Alternative/old names
        ("swaziland", "SZ"),
        ("cape verde", "CV"),
        ("north macedonia", "MK"),
        ("timor leste", "TL"),
        ("timorleste", "TL"),
        // Abbreviations
        ("salvador", "SV"),
        ("papua", "PG"),
        // Diacritics removed
        ("reunion", "RE"),
        // Region codes
        ("hong kong", "HK"),
        ("macao", "MO"),
        ("puerto rico", "PR"),
        // Name changes
        ("turkey", "TR"),
    ])
});

/// ISO standard names: normalized ISO name -> Alpha2
/// Built from keshvar at startup.
static ISO_NAME2ALPHA2: Lazy<HashMap<String, Alpha2>> = Lazy::new(|| {
    let mut m = HashMap::new();
    for country in CountryIterator::new() {
        m.insert(norm(country.iso_short_name()), country.alpha2());
    }
    m
});

/// Mapping from Hero SMS country IDs to Country.
/// Built from hero_sms_countries.json at startup.
pub static SMS_ID2COUNTRY: Lazy<HashMap<u16, Country>> = Lazy::new(|| {
    let raw: HashMap<String, Value> =
        serde_json::from_str(COUNTRIES_JSON).expect("hero_sms_countries.json is invalid");

    let mut map = HashMap::with_capacity(raw.len());

    for (id_str, name_val) in raw {
        let Ok(id) = id_str.parse::<u16>() else {
            continue;
        };
        let Some(name) = name_val.as_str() else {
            continue;
        };

        let key = norm(name);

        // 1) First check overrides for known name differences
        if let Some(&alpha2_str) = NAME_OVERRIDES.get(key.as_str())
            && let Ok(country) = Country::try_from(alpha2_str)
        {
            map.insert(id, country);
            continue;
        }

        // 2) Try to match against ISO standard name()
        if let Some(&alpha2) = ISO_NAME2ALPHA2.get(&key) {
            map.insert(id, alpha2.to_country());
            continue;
        }

        // If no match found, skip but could log for debugging
        #[cfg(feature = "tracing")]
        tracing::debug!("No ISO match for SMS country name: '{name}' (id={id})");
    }

    map
});

/// Reverse mapping: Alpha2 string -> Hero SMS ID.
pub static COUNTRY2SMS_ID: Lazy<HashMap<String, u16>> = Lazy::new(|| {
    let mut m = HashMap::with_capacity(SMS_ID2COUNTRY.len());
    for (id, country) in SMS_ID2COUNTRY.iter() {
        m.entry(country.alpha2().to_string()).or_insert(*id);
    }
    m
});

/// Extension trait for country code mapping.
pub trait SmsCountryExt {
    /// Get the Hero SMS country ID for this country.
    fn sms_id(&self) -> Result<u16, CountryMapError>;

    /// Get the Country for a Hero SMS ID.
    fn from_sms_id(id: u16) -> Result<Country, CountryMapError>;
}

impl SmsCountryExt for Country {
    fn sms_id(&self) -> Result<u16, CountryMapError> {
        COUNTRY2SMS_ID
            .get(&self.alpha2().to_string())
            .copied()
            .ok_or_else(|| CountryMapError::NoSmsMapping {
                country: Box::new(self.clone()),
            })
    }

    fn from_sms_id(id: u16) -> Result<Country, CountryMapError> {
        SMS_ID2COUNTRY
            .get(&id)
            .cloned()
            .ok_or(CountryMapError::UnknownSmsId { id })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use keshvar::Alpha2;

    #[test]
    fn test_norm_basic() {
        assert_eq!(norm("Russia"), "russia");
        assert_eq!(norm("United States"), "united states");
        assert_eq!(norm("SOUTH KOREA"), "south korea");
    }

    #[test]
    fn test_norm_removes_punctuation() {
        assert_eq!(norm("Saint-Martin"), "saintmartin");
        assert_eq!(norm("Korea, South"), "korea south");
        assert_eq!(norm("U.S.A."), "usa");
        assert_eq!(norm("People's Republic"), "peoples republic");
    }

    #[test]
    fn test_norm_multiple_spaces() {
        assert_eq!(norm("United   States"), "united states");
        assert_eq!(norm("  Russia  "), "russia");
    }

    #[test]
    fn test_name_overrides_present() {
        assert!(NAME_OVERRIDES.contains_key("usa"));
        assert!(NAME_OVERRIDES.contains_key("united kingdom"));
        assert!(NAME_OVERRIDES.contains_key("uae"));
        assert!(NAME_OVERRIDES.contains_key("czech"));
    }

    #[test]
    fn test_name_overrides_correct() {
        assert_eq!(NAME_OVERRIDES.get("usa"), Some(&"US"));
        assert_eq!(NAME_OVERRIDES.get("united kingdom"), Some(&"GB"));
        assert_eq!(NAME_OVERRIDES.get("uae"), Some(&"AE"));
        assert_eq!(NAME_OVERRIDES.get("ivory coast"), Some(&"CI"));
    }

    #[test]
    fn test_iso_name2alpha2_populated() {
        assert!(!ISO_NAME2ALPHA2.is_empty());
        assert!(ISO_NAME2ALPHA2.contains_key("ukraine"));
        assert!(ISO_NAME2ALPHA2.contains_key("germany"));
        assert!(ISO_NAME2ALPHA2.contains_key("france"));
        assert!(ISO_NAME2ALPHA2.contains_key("japan"));
    }

    #[test]
    fn test_iso_name2alpha2_values() {
        assert_eq!(ISO_NAME2ALPHA2.get("ukraine"), Some(&Alpha2::UA));
        assert_eq!(ISO_NAME2ALPHA2.get("germany"), Some(&Alpha2::DE));
        assert_eq!(ISO_NAME2ALPHA2.get("france"), Some(&Alpha2::FR));
        assert_eq!(ISO_NAME2ALPHA2.get("japan"), Some(&Alpha2::JP));
    }

    #[test]
    fn test_sms_id2country_populated() {
        assert!(!SMS_ID2COUNTRY.is_empty());
        // Should have many countries mapped
        assert!(
            SMS_ID2COUNTRY.len() > 50,
            "Too few countries mapped: {}",
            SMS_ID2COUNTRY.len()
        );
    }

    #[test]
    fn test_country2sms_id_populated() {
        assert!(!COUNTRY2SMS_ID.is_empty());
        assert_eq!(COUNTRY2SMS_ID.len(), SMS_ID2COUNTRY.len());
    }

    #[test]
    fn test_country_to_sms_id() {
        // Test countries from sms_activate_countries.json
        assert_eq!(Alpha2::UA.to_country().sms_id().unwrap(), 1);
        assert_eq!(Alpha2::GB.to_country().sms_id().unwrap(), 16);
        assert_eq!(Alpha2::US.to_country().sms_id().unwrap(), 187);
    }

    #[test]
    fn test_sms_id_to_country() {
        assert_eq!(Country::from_sms_id(1).unwrap().alpha2(), Alpha2::UA);
        assert_eq!(Country::from_sms_id(16).unwrap().alpha2(), Alpha2::GB);
        assert_eq!(Country::from_sms_id(187).unwrap().alpha2(), Alpha2::US);
    }

    #[test]
    fn test_unknown_country() {
        // Antarctica doesn't have SMS service
        assert!(Alpha2::AQ.to_country().sms_id().is_err());
    }

    #[test]
    fn test_unknown_sms_id() {
        assert!(Country::from_sms_id(9999).is_err());
    }

    #[test]
    fn test_round_trip_conversion() {
        for (sms_id, original_country) in SMS_ID2COUNTRY.iter() {
            let converted_country = Country::from_sms_id(*sms_id)
                .unwrap_or_else(|_| panic!("Failed to convert SMS ID {} back to Country", sms_id));
            assert_eq!(
                original_country.alpha2(),
                converted_country.alpha2(),
                "Round-trip failed for {:?} (SMS ID: {})",
                original_country.iso_short_name(),
                sms_id
            );
        }
    }

    #[test]
    fn test_reverse_round_trip_conversion() {
        for (original_id, country) in SMS_ID2COUNTRY.iter() {
            let converted_id = country.sms_id().unwrap_or_else(|_| {
                panic!("Failed to get SMS ID for {:?}", country.iso_short_name())
            });
            assert_eq!(
                *original_id,
                converted_id,
                "Reverse round-trip failed for SMS ID {} ({:?})",
                original_id,
                country.iso_short_name()
            );
        }
    }

    #[test]
    fn test_popular_countries_have_mapping() {
        let popular = [
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
        ];

        for alpha2 in popular {
            let country = alpha2.to_country();
            assert!(
                country.sms_id().is_ok(),
                "Popular country {:?} ({:?}) should have SMS mapping",
                country.iso_short_name(),
                country.alpha2()
            );
        }
    }

    #[test]
    fn test_error_display() {
        let err1 = CountryMapError::UnknownSmsId { id: 12345 };
        assert!(err1.to_string().contains("12345"));
        assert!(err1.to_string().contains("Unknown country"));

        let err2 = CountryMapError::NoSmsMapping {
            country: Box::new(Alpha2::AQ.to_country()),
        };
        assert!(err2.to_string().contains("Antarctica"));
        assert!(err2.to_string().contains("No Hero SMS mapping"));
    }

    #[test]
    fn test_countries_json_valid() {
        let result: Result<HashMap<String, Value>, _> = serde_json::from_str(COUNTRIES_JSON);
        assert!(
            result.is_ok(),
            "hero_sms_countries.json should be valid JSON"
        );

        let data = result.unwrap();
        assert!(
            !data.is_empty(),
            "hero_sms_countries.json should not be empty"
        );
    }
}
