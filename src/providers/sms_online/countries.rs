//! Country code mapping for SMS.online API.

use crate::providers::common::countries::{build_country_to_id_map, build_id_to_country_map};
use keshvar::Country;
use std::collections::HashMap;
use std::sync::LazyLock;
use thiserror::Error;

/// Error when mapping country codes.
#[derive(Debug, Clone, Error)]
pub enum CountryMapError {
    /// Unknown SMS.online ID.
    #[error("Unknown country for SMS.online id {id}")]
    UnknownSmsOnlineId { id: u16 },
    /// No SMS.online mapping for country.
    #[error("No SMS.online mapping for country {}", country.iso_short_name())]
    NoSmsOnlineMapping { country: Box<Country> },
}

/// SMS.online countries JSON embedded at compile time.
static COUNTRIES_JSON: &str = include_str!("../../../assets/sms_online_countries.json");

/// Mapping from SMS.online country IDs to Country.
pub static SMS_ONLINE_ID2COUNTRY: LazyLock<HashMap<u16, Country>> =
    LazyLock::new(|| build_id_to_country_map(COUNTRIES_JSON, "sms_online_countries.json"));

/// Reverse mapping: Alpha2 string -> SMS.online ID.
pub static COUNTRY2SMS_ONLINE_ID: LazyLock<HashMap<String, u16>> =
    LazyLock::new(|| build_country_to_id_map(&SMS_ONLINE_ID2COUNTRY));

/// Extension trait for country code mapping.
pub trait SmsOnlineCountryExt {
    /// Get the SMS.online country ID for this country.
    fn sms_online_id(&self) -> Result<u16, CountryMapError>;

    /// Get the Country for a SMS.online ID.
    fn from_sms_online_id(id: u16) -> Result<Country, CountryMapError>;
}

impl SmsOnlineCountryExt for Country {
    fn sms_online_id(&self) -> Result<u16, CountryMapError> {
        COUNTRY2SMS_ONLINE_ID
            .get(&self.alpha2().to_string())
            .copied()
            .ok_or_else(|| CountryMapError::NoSmsOnlineMapping {
                country: Box::new(self.clone()),
            })
    }

    fn from_sms_online_id(id: u16) -> Result<Country, CountryMapError> {
        SMS_ONLINE_ID2COUNTRY
            .get(&id)
            .cloned()
            .ok_or(CountryMapError::UnknownSmsOnlineId { id })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use keshvar::Alpha2;
    use serde_json::Value;

    #[test]
    fn test_sms_online_id2country_populated() {
        assert!(!SMS_ONLINE_ID2COUNTRY.is_empty());
        assert!(
            SMS_ONLINE_ID2COUNTRY.len() > 50,
            "Too few countries mapped: {}",
            SMS_ONLINE_ID2COUNTRY.len()
        );
    }

    #[test]
    fn test_country2sms_online_id_populated() {
        assert!(!COUNTRY2SMS_ONLINE_ID.is_empty());
        assert_eq!(COUNTRY2SMS_ONLINE_ID.len(), SMS_ONLINE_ID2COUNTRY.len());
    }

    #[test]
    fn test_country_to_sms_online_id() {
        assert_eq!(Alpha2::UA.to_country().sms_online_id().unwrap(), 1);
        assert_eq!(Alpha2::GB.to_country().sms_online_id().unwrap(), 16);
        assert_eq!(Alpha2::US.to_country().sms_online_id().unwrap(), 187);
    }

    #[test]
    fn test_sms_online_id_to_country() {
        assert_eq!(Country::from_sms_online_id(1).unwrap().alpha2(), Alpha2::UA);
        assert_eq!(
            Country::from_sms_online_id(16).unwrap().alpha2(),
            Alpha2::GB
        );
        assert_eq!(
            Country::from_sms_online_id(187).unwrap().alpha2(),
            Alpha2::US
        );
    }

    #[test]
    fn test_unknown_country() {
        assert!(Alpha2::AQ.to_country().sms_online_id().is_err());
    }

    #[test]
    fn test_unknown_sms_online_id() {
        assert!(Country::from_sms_online_id(9999).is_err());
    }

    #[test]
    fn test_round_trip_conversion() {
        for (sms_id, original_country) in SMS_ONLINE_ID2COUNTRY.iter() {
            let converted_country = Country::from_sms_online_id(*sms_id).unwrap_or_else(|_| {
                panic!("Failed to convert SMS.online ID {} back to Country", sms_id)
            });
            assert_eq!(
                original_country.alpha2(),
                converted_country.alpha2(),
                "Round-trip failed for {:?} (SMS.online ID: {})",
                original_country.iso_short_name(),
                sms_id
            );
        }
    }

    #[test]
    fn test_reverse_round_trip_conversion() {
        for (original_id, country) in SMS_ONLINE_ID2COUNTRY.iter() {
            let converted_id = country.sms_online_id().unwrap_or_else(|_| {
                panic!(
                    "Failed to get SMS.online ID for {:?}",
                    country.iso_short_name()
                )
            });
            assert_eq!(
                *original_id,
                converted_id,
                "Reverse round-trip failed for SMS.online ID {} ({:?})",
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
                country.sms_online_id().is_ok(),
                "Popular country {:?} ({:?}) should have SMS mapping",
                country.iso_short_name(),
                country.alpha2()
            );
        }
    }

    #[test]
    fn test_error_display() {
        let err1 = CountryMapError::UnknownSmsOnlineId { id: 12345 };
        assert!(err1.to_string().contains("12345"));
        assert!(err1.to_string().contains("Unknown country"));

        let err2 = CountryMapError::NoSmsOnlineMapping {
            country: Box::new(Alpha2::AQ.to_country()),
        };
        assert!(err2.to_string().contains("Antarctica"));
        assert!(err2.to_string().contains("No SMS.online mapping"));
    }

    #[test]
    fn test_countries_json_valid() {
        let result: Result<HashMap<String, Value>, _> = serde_json::from_str(COUNTRIES_JSON);
        assert!(
            result.is_ok(),
            "sms_online_countries.json should be valid JSON"
        );

        let data = result.unwrap();
        assert!(
            !data.is_empty(),
            "sms_online_countries.json should not be empty"
        );
    }
}
