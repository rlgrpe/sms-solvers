//! Country code mapping for SMS Activate API.

use isocountry::CountryCode;
use once_cell::sync::Lazy;
use serde_json::Value;
use std::collections::HashMap;
use thiserror::Error;

/// Error when mapping country codes.
#[derive(Debug, Clone, Error)]
pub enum CountryMapError {
    /// Unknown SMS-Activate ID.
    #[error("Unknown ISO country for SMS-Activate id {id}")]
    UnknownSmsId { id: u16 },
    /// No SMS-Activate mapping for country.
    #[error("No SMS-Activate mapping for country {}", code.alpha2())]
    NoSmsMapping { code: CountryCode },
}

/// SMS Activate countries JSON embedded at compile time.
static COUNTRIES_JSON: &str = include_str!("../../../assets/sms_activate_countries.json");

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

/// Overrides: normalized SMS name -> ISO CountryCode
/// Used where SMS-Activate names differ significantly from ISO standard names.
static NAME_OVERRIDES: Lazy<HashMap<&'static str, CountryCode>> = Lazy::new(|| {
    use CountryCode::*;
    HashMap::from([
        // Primary mappings
        ("usa", USA),
        ("united states", USA),
        ("united kingdom", GBR),
        ("uae", ARE),
        // Name differences
        ("vietnam", VNM),
        ("south korea", KOR),
        ("north korea", PRK),
        ("dr congo", COD),
        ("ivory coast", CIV),
        ("czech", CZE),
        ("moldova", MDA),
        ("laos", LAO),
        ("syria", SYR),
        ("iran", IRN),
        ("venezuela", VEN),
        ("tanzania", TZA),
        ("bolivia", BOL),
        ("bosnia", BIH),
        ("brunei", BRN),
        ("palestine", PSE),
        ("taiwan", TWN),
        // Alternative/old names
        ("swaziland", SWZ),
        ("cape verde", CPV),
        ("north macedonia", MKD),
        ("timor leste", TLS),
        ("timorleste", TLS),
        // Abbreviations
        ("salvador", SLV),
        ("papua", PNG),
        // Diacritics removed
        ("reunion", REU),
        // Region codes
        ("hong kong", HKG),
        ("macao", MAC),
        ("puerto rico", PRI),
    ])
});

/// ISO standard names: normalized ISO name() -> CountryCode
/// Built from isocountry at startup.
static ISO_NAME2CC: Lazy<HashMap<String, CountryCode>> = Lazy::new(|| {
    let mut m = HashMap::new();
    for cc in CountryCode::iter() {
        m.insert(norm(cc.name()), *cc);
    }
    m
});

/// Mapping from SMS Activate country IDs to ISO CountryCode.
/// Built from sms_activate_countries.json at startup.
pub static SMS_ID2CC: Lazy<HashMap<u16, CountryCode>> = Lazy::new(|| {
    let raw: HashMap<String, Value> =
        serde_json::from_str(COUNTRIES_JSON).expect("sms_activate_countries.json is invalid");

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
        if let Some(&cc) = NAME_OVERRIDES.get(key.as_str()) {
            map.insert(id, cc);
            continue;
        }

        // 2) Try to match against ISO standard name()
        if let Some(&cc) = ISO_NAME2CC.get(&key) {
            map.insert(id, cc);
            continue;
        }

        // If no match found, skip but could log for debugging
        #[cfg(feature = "tracing")]
        tracing::debug!("No ISO match for SMS country name: '{name}' (id={id})");
    }

    map
});

/// Reverse mapping: CountryCode -> SMS Activate ID.
pub static CC2SMS_ID: Lazy<HashMap<CountryCode, u16>> = Lazy::new(|| {
    let mut m = HashMap::with_capacity(SMS_ID2CC.len());
    for (id, cc) in SMS_ID2CC.iter() {
        m.entry(*cc).or_insert(*id);
    }
    m
});

/// Extension trait for country code mapping.
pub trait SmsCountryExt {
    /// Get the SMS Activate country ID for this country.
    fn sms_id(&self) -> Result<u16, CountryMapError>;

    /// Get the ISO country code for an SMS Activate ID.
    #[allow(dead_code)]
    fn from_sms_id(id: u16) -> Result<CountryCode, CountryMapError>;
}

impl SmsCountryExt for CountryCode {
    fn sms_id(&self) -> Result<u16, CountryMapError> {
        CC2SMS_ID
            .get(self)
            .copied()
            .ok_or(CountryMapError::NoSmsMapping { code: *self })
    }

    fn from_sms_id(id: u16) -> Result<CountryCode, CountryMapError> {
        SMS_ID2CC
            .get(&id)
            .copied()
            .ok_or(CountryMapError::UnknownSmsId { id })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(NAME_OVERRIDES.get("usa"), Some(&CountryCode::USA));
        assert_eq!(
            NAME_OVERRIDES.get("united kingdom"),
            Some(&CountryCode::GBR)
        );
        assert_eq!(NAME_OVERRIDES.get("uae"), Some(&CountryCode::ARE));
        assert_eq!(NAME_OVERRIDES.get("ivory coast"), Some(&CountryCode::CIV));
    }

    #[test]
    fn test_iso_name2cc_populated() {
        assert!(!ISO_NAME2CC.is_empty());
        assert!(ISO_NAME2CC.contains_key("ukraine"));
        assert!(ISO_NAME2CC.contains_key("germany"));
        assert!(ISO_NAME2CC.contains_key("france"));
        assert!(ISO_NAME2CC.contains_key("japan"));
    }

    #[test]
    fn test_iso_name2cc_values() {
        assert_eq!(ISO_NAME2CC.get("ukraine"), Some(&CountryCode::UKR));
        assert_eq!(ISO_NAME2CC.get("germany"), Some(&CountryCode::DEU));
        assert_eq!(ISO_NAME2CC.get("france"), Some(&CountryCode::FRA));
        assert_eq!(ISO_NAME2CC.get("japan"), Some(&CountryCode::JPN));
    }

    #[test]
    fn test_sms_id2cc_populated() {
        assert!(!SMS_ID2CC.is_empty());
        // Should have many countries mapped
        assert!(
            SMS_ID2CC.len() > 50,
            "Too few countries mapped: {}",
            SMS_ID2CC.len()
        );
    }

    #[test]
    fn test_cc2sms_id_populated() {
        assert!(!CC2SMS_ID.is_empty());
        assert_eq!(CC2SMS_ID.len(), SMS_ID2CC.len());
    }

    #[test]
    fn test_country_to_sms_id() {
        // Test countries from sms_activate_countries.json
        assert_eq!(CountryCode::UKR.sms_id().unwrap(), 1);
        assert_eq!(CountryCode::GBR.sms_id().unwrap(), 16);
        assert_eq!(CountryCode::USA.sms_id().unwrap(), 187);
    }

    #[test]
    fn test_sms_id_to_country() {
        assert_eq!(CountryCode::from_sms_id(1).unwrap(), CountryCode::UKR);
        assert_eq!(CountryCode::from_sms_id(16).unwrap(), CountryCode::GBR);
        assert_eq!(CountryCode::from_sms_id(187).unwrap(), CountryCode::USA);
    }

    #[test]
    fn test_unknown_country() {
        // Antarctica doesn't have SMS service
        assert!(CountryCode::ATA.sms_id().is_err());
    }

    #[test]
    fn test_unknown_sms_id() {
        assert!(CountryCode::from_sms_id(9999).is_err());
    }

    #[test]
    fn test_round_trip_conversion() {
        for (original_cc, sms_id) in CC2SMS_ID.iter() {
            let converted_cc = CountryCode::from_sms_id(*sms_id).expect(&format!(
                "Failed to convert SMS ID {} back to CountryCode",
                sms_id
            ));
            assert_eq!(
                *original_cc, converted_cc,
                "Round-trip failed for {:?} (SMS ID: {})",
                original_cc, sms_id
            );
        }
    }

    #[test]
    fn test_reverse_round_trip_conversion() {
        for (original_id, cc) in SMS_ID2CC.iter() {
            let converted_id = cc
                .sms_id()
                .expect(&format!("Failed to get SMS ID for {:?}", cc));
            assert_eq!(
                *original_id, converted_id,
                "Reverse round-trip failed for SMS ID {} ({:?})",
                original_id, cc
            );
        }
    }

    #[test]
    fn test_popular_countries_have_mapping() {
        let popular = [
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
        ];

        for cc in popular {
            assert!(
                cc.sms_id().is_ok(),
                "Popular country {:?} ({}) should have SMS mapping",
                cc,
                cc.name()
            );
        }
    }

    #[test]
    fn test_error_display() {
        let err1 = CountryMapError::UnknownSmsId { id: 12345 };
        assert!(err1.to_string().contains("12345"));
        assert!(err1.to_string().contains("Unknown ISO country"));

        let err2 = CountryMapError::NoSmsMapping {
            code: CountryCode::ATA,
        };
        assert!(err2.to_string().contains("AQ"));
        assert!(err2.to_string().contains("No SMS-Activate mapping"));
    }

    #[test]
    fn test_countries_json_valid() {
        let result: Result<HashMap<String, Value>, _> = serde_json::from_str(COUNTRIES_JSON);
        assert!(
            result.is_ok(),
            "sms_activate_countries.json should be valid JSON"
        );

        let data = result.unwrap();
        assert!(
            !data.is_empty(),
            "sms_activate_countries.json should not be empty"
        );
    }
}
