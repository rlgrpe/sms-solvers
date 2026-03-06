//! Shared country mapping utilities.
//!
//! Provides the name normalization, overrides, and map-building algorithm
//! shared by all provider country mapping modules.

use keshvar::{Alpha2, Country, CountryIterator};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::LazyLock;

/// Name normalization for stable comparison.
/// Converts to lowercase and removes punctuation/extra whitespace.
pub(crate) fn norm(s: &str) -> String {
    const PUNCT: &[char] = &[
        '\'', '"', '`', ',', '.', '-', '_', '(', ')', '\u{2018}',
        '\u{2019}', // curly single quotes
        '\u{00B4}', // acute accent
    ];
    s.to_ascii_lowercase()
        .replace(PUNCT, "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Overrides: normalized name -> ISO alpha-2 code.
/// Used where provider names differ significantly from ISO standard names.
pub(crate) static NAME_OVERRIDES: LazyLock<HashMap<&'static str, &'static str>> =
    LazyLock::new(|| {
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

/// ISO standard names: normalized ISO name -> Alpha2.
/// Built from keshvar at startup.
pub(crate) static ISO_NAME2ALPHA2: LazyLock<HashMap<String, Alpha2>> = LazyLock::new(|| {
    let mut m = HashMap::new();
    for country in CountryIterator::new() {
        m.insert(norm(country.iso_short_name()), country.alpha2());
    }
    m
});

/// Build a provider ID → Country map from a JSON string of `{ "id": "name", ... }`.
///
/// The `asset_name` is used only for the tracing debug message when a name
/// cannot be matched.
pub(crate) fn build_id_to_country_map(json: &str, asset_name: &str) -> HashMap<u16, Country> {
    let raw: HashMap<String, Value> =
        serde_json::from_str(json).unwrap_or_else(|e| panic!("{asset_name} is invalid JSON: {e}"));

    let mut map = HashMap::with_capacity(raw.len());

    for (id_str, name_val) in raw {
        let Ok(id) = id_str.parse::<u16>() else {
            continue;
        };
        let Some(name) = name_val.as_str() else {
            continue;
        };

        let key = norm(name);

        // 1) Check overrides for known name differences
        if let Some(&alpha2_str) = NAME_OVERRIDES.get(key.as_str())
            && let Ok(country) = Country::try_from(alpha2_str)
        {
            map.insert(id, country);
            continue;
        }

        // 2) Try to match against ISO standard name
        if let Some(&alpha2) = ISO_NAME2ALPHA2.get(&key) {
            map.insert(id, alpha2.to_country());
            continue;
        }

        #[cfg(feature = "tracing")]
        tracing::debug!("No ISO match for {asset_name} country name: '{name}' (id={id})");
    }

    map
}

/// Build a reverse map: Alpha2 string → provider ID.
pub(crate) fn build_country_to_id_map(id2country: &HashMap<u16, Country>) -> HashMap<String, u16> {
    let mut m = HashMap::with_capacity(id2country.len());
    for (id, country) in id2country.iter() {
        m.entry(country.alpha2().to_string()).or_insert(*id);
    }
    m
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
}
