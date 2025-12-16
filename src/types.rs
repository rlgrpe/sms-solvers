//! Core types for SMS verification operations.

use keshvar::Country;
use rand::Rng;
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use std::fmt::{self, Display, Formatter};
use std::str::FromStr;
use thiserror::Error;
// =============================================================================
// TaskId
// =============================================================================

/// Unique identifier for an SMS activation task.
///
/// This ID is returned by the provider when a phone number is acquired
/// and is used to track the activation status and retrieve SMS codes.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TaskId(String);

impl TaskId {
    /// Create a new TaskId from a string.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl Display for TaskId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for TaskId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<String> for TaskId {
    fn from(id: String) -> Self {
        Self(id)
    }
}

impl From<&str> for TaskId {
    fn from(id: &str) -> Self {
        Self(id.to_string())
    }
}

// =============================================================================
// SmsCode (OTP)
// =============================================================================

/// SMS verification code (OTP).
///
/// Represents the code received via SMS for verification purposes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SmsCode(pub String);

impl SmsCode {
    /// Create a new SmsCode.
    pub fn new(code: impl Into<String>) -> Self {
        Self(code.into())
    }

    /// Get the code as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for SmsCode {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for SmsCode {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<String> for SmsCode {
    fn from(code: String) -> Self {
        Self(code)
    }
}

impl From<&str> for SmsCode {
    fn from(code: &str) -> Self {
        Self(code.to_string())
    }
}

// =============================================================================
// FullNumber
// =============================================================================

/// Full phone number with country code (e.g., "905488242474").
///
/// This represents the complete phone number including the country dial code,
/// as returned by the SMS provider.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FullNumber(String);

impl FullNumber {
    /// Create a new FullNumber.
    pub fn new(number: impl Into<String>) -> Self {
        Self(number.into())
    }

    /// Get the number as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Get the number with a '+' prefix.
    ///
    /// # Example
    ///
    /// ```rust
    /// use sms_solvers::FullNumber;
    ///
    /// let num = FullNumber::new("905488242474");
    /// assert_eq!(num.with_plus_prefix(), "+905488242474");
    /// ```
    pub fn with_plus_prefix(&self) -> String {
        if self.0.starts_with('+') {
            self.0.clone()
        } else {
            format!("+{}", self.0)
        }
    }

    /// Check if the number starts with the given dial code.
    ///
    /// # Example
    ///
    /// ```rust
    /// use sms_solvers::{FullNumber, DialCode};
    ///
    /// let num = FullNumber::new("905488242474");
    /// let dc_tr = DialCode::new("90").unwrap();
    /// let dc_us = DialCode::new("1").unwrap();
    ///
    /// assert!(num.starts_with_dial_code(&dc_tr));
    /// assert!(!num.starts_with_dial_code(&dc_us));
    /// ```
    pub fn starts_with_dial_code(&self, dial_code: &DialCode) -> bool {
        let normalized = self.0.trim_start_matches('+');
        normalized.starts_with(dial_code.as_str())
    }
}

impl Display for FullNumber {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for FullNumber {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<String> for FullNumber {
    fn from(number: String) -> Self {
        Self(number)
    }
}

impl From<&str> for FullNumber {
    fn from(number: &str) -> Self {
        Self(number.to_string())
    }
}

// =============================================================================
// DialCode
// =============================================================================

/// Error when parsing a dial code.
#[derive(Debug, Clone, Error)]
pub enum DialCodeError {
    /// Dial code contains non-digit characters.
    #[error("dial code must contain only digits")]
    NonDigit,
    /// Dial code is empty.
    #[error("dial code cannot be empty")]
    Empty,
}

/// Country dial code (e.g., "1" for USA, "380" for Ukraine).
///
/// Dial codes are stored without the leading '+' sign.
///
/// # Example
///
/// ```rust
/// use sms_solvers::DialCode;
///
/// let dc = DialCode::new("+380").unwrap();
/// assert_eq!(dc.to_string(), "380");
///
/// let dc = DialCode::new("1").unwrap();
/// assert_eq!(dc.to_string(), "1");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DialCode(String);

impl DialCode {
    /// Create a new DialCode from a string.
    ///
    /// The input can include a leading '+' which will be stripped.
    pub fn new(s: impl AsRef<str>) -> Result<Self, DialCodeError> {
        let n = s.as_ref().trim().trim_start_matches('+');
        if n.is_empty() {
            return Err(DialCodeError::Empty);
        }
        if !n.chars().all(|c| c.is_ascii_digit()) {
            return Err(DialCodeError::NonDigit);
        }
        Ok(Self(n.to_string()))
    }

    /// Generate a random popular DialCode.
    #[cfg(feature = "random")]
    pub fn generate() -> Result<Self, DialCodeError> {
        const POPULAR_CODES: &[&str] = &["1", "44", "49", "7", "380", "91", "81", "61", "33", "39"];
        let mut rng = rand::thread_rng();
        let idx = rng.gen_range(0..POPULAR_CODES.len());
        Self::new(POPULAR_CODES[idx])
    }

    /// Get the dial code as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for DialCode {
    type Err = DialCodeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl Display for DialCode {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<'de> Deserialize<'de> for DialCode {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(d)?;
        DialCode::new(raw).map_err(de::Error::custom)
    }
}

impl Serialize for DialCode {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.0)
    }
}

/// Error when converting a dial code to a country.
#[derive(Debug, Clone, Error)]
pub enum DialCodeToCountryError {
    /// No country found for the given dial code.
    #[error("no country found for dial code '{dial_code}'")]
    NotFound { dial_code: String },
    /// Invalid dial code format (not a valid number).
    #[error("invalid dial code format: '{dial_code}'")]
    InvalidFormat { dial_code: String },
}

impl From<&Country> for DialCode {
    /// Convert a Country to its dial code.
    ///
    /// # Panics
    ///
    /// This should never panic as keshvar provides valid country codes.
    fn from(country: &Country) -> Self {
        DialCode::new(country.country_code().to_string())
            .expect("keshvar country codes are always valid")
    }
}

impl From<Country> for DialCode {
    fn from(country: Country) -> Self {
        DialCode::from(&country)
    }
}

impl TryFrom<&DialCode> for Country {
    type Error = DialCodeToCountryError;

    /// Convert a dial code to a Country.
    ///
    /// Uses keshvar's `find_by_code` function for lookup.
    ///
    /// # Errors
    ///
    /// Returns an error if no country is found for the given dial code
    /// or if the dial code is not a valid number.
    fn try_from(dial_code: &DialCode) -> Result<Self, Self::Error> {
        let code: usize =
            dial_code
                .as_str()
                .parse()
                .map_err(|_| DialCodeToCountryError::InvalidFormat {
                    dial_code: dial_code.to_string(),
                })?;

        keshvar::find_by_code(code).map_err(|_| DialCodeToCountryError::NotFound {
            dial_code: dial_code.to_string(),
        })
    }
}

impl TryFrom<DialCode> for Country {
    type Error = DialCodeToCountryError;

    fn try_from(dial_code: DialCode) -> Result<Self, Self::Error> {
        Country::try_from(&dial_code)
    }
}

impl DialCode {
    /// Try to convert this dial code to a Country.
    ///
    /// Uses keshvar's built-in country code lookup.
    ///
    /// # Errors
    ///
    /// Returns an error if no country is found for the given dial code.
    pub fn to_country(&self) -> Result<Country, DialCodeToCountryError> {
        Country::try_from(self)
    }
}

// =============================================================================
// Number
// =============================================================================

/// Error when parsing a phone number.
#[derive(Debug, Clone, Error)]
pub enum NumberError {
    /// Number contains non-digit characters.
    #[error("number must contain only digits")]
    NonDigit,
    /// Number has invalid length.
    #[error("number must be between 4 and 14 digits")]
    InvalidLength,
    /// Number starts with zero.
    #[error("number cannot start with 0")]
    LeadingZero,
    /// Dial code not found at the beginning.
    #[error("dial code not found at the beginning of the number")]
    MissingDialCode,
}

/// Phone number without country code (e.g., "5488242474").
///
/// This represents just the national part of a phone number,
/// without the country dial code.
///
/// # Validation Rules
///
/// - Must contain only digits
/// - Must be between 4 and 14 digits
/// - Cannot start with 0
///
/// # Example
///
/// ```rust
/// use sms_solvers::{Number, DialCode, FullNumber};
///
/// // Create from string
/// let num = Number::new("5488242474").unwrap();
///
/// // Extract from full number
/// let dial_code = DialCode::new("90").unwrap();
/// let full = FullNumber::new("905488242474");
/// let num = Number::from_full_number(&full, &dial_code).unwrap();
/// assert_eq!(num.to_string(), "5488242474");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Number(String);

impl Number {
    /// Create a new Number from a string.
    pub fn new(s: impl AsRef<str>) -> Result<Self, NumberError> {
        let s = s.as_ref().trim();
        if !s.chars().all(|c| c.is_ascii_digit()) {
            return Err(NumberError::NonDigit);
        }
        let len = s.len();
        if !(4..=14).contains(&len) {
            return Err(NumberError::InvalidLength);
        }
        if s.starts_with('0') {
            return Err(NumberError::LeadingZero);
        }
        Ok(Self(s.to_string()))
    }

    /// Extract the national number from a full number by removing the dial code.
    pub fn from_full_number(full: &FullNumber, dial_code: &DialCode) -> Result<Self, NumberError> {
        let full_str = full.as_ref().trim().trim_start_matches('+');
        let code = dial_code.as_str();

        let number_part = full_str
            .strip_prefix(code)
            .ok_or(NumberError::MissingDialCode)?;

        Self::new(number_part)
    }

    /// Generate a random valid Number.
    #[cfg(feature = "random")]
    pub fn generate() -> Result<Self, NumberError> {
        let mut rng = rand::thread_rng();
        let first: u64 = rng.gen_range(1..10);
        let rest: u64 = rng.gen_range(0..1_000_000_000);
        Number::new(format!("{first}{rest:09}"))
    }

    /// Get the number as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for Number {
    type Err = NumberError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl Display for Number {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// =============================================================================
// SmsTaskResult
// =============================================================================

/// Result of acquiring a phone number for SMS verification.
///
/// Contains all information about the acquired phone number,
/// including the task ID for tracking and the parsed number components.
#[derive(Debug, Clone)]
pub struct SmsTaskResult {
    /// Unique identifier for this SMS task.
    pub task_id: TaskId,
    /// Country dial code.
    pub dial_code: DialCode,
    /// National phone number (without dial code).
    pub number: Number,
    /// Full phone number with dial code.
    pub full_number: FullNumber,
    /// Country.
    pub country: Country,
}

#[cfg(test)]
mod tests {
    use super::*;

    // TaskId tests
    #[test]
    fn test_task_id_from_string() {
        let id = TaskId::from("12345");
        assert_eq!(id.to_string(), "12345");
        assert_eq!(id.as_ref(), "12345");
    }

    // SmsCode tests
    #[test]
    fn test_sms_code() {
        let code = SmsCode::new("123456");
        assert_eq!(code.as_str(), "123456");
        assert_eq!(code.to_string(), "123456");
    }

    // FullNumber tests
    #[test]
    fn test_full_number() {
        let num = FullNumber::new("905488242474");
        assert_eq!(num.as_str(), "905488242474");
        assert_eq!(num.to_string(), "905488242474");
    }

    #[test]
    fn test_full_number_with_plus_prefix() {
        let num = FullNumber::new("905488242474");
        assert_eq!(num.with_plus_prefix(), "+905488242474");

        // Already has plus prefix
        let num_with_plus = FullNumber::new("+905488242474");
        assert_eq!(num_with_plus.with_plus_prefix(), "+905488242474");
    }

    #[test]
    fn test_full_number_starts_with_dial_code() {
        let num = FullNumber::new("905488242474");
        let dc_tr = DialCode::new("90").unwrap();
        let dc_us = DialCode::new("1").unwrap();

        assert!(num.starts_with_dial_code(&dc_tr));
        assert!(!num.starts_with_dial_code(&dc_us));

        // With plus prefix
        let num_with_plus = FullNumber::new("+905488242474");
        assert!(num_with_plus.starts_with_dial_code(&dc_tr));
    }

    // DialCode tests
    #[test]
    fn test_dial_code_valid() {
        assert!(DialCode::new("1").is_ok());
        assert!(DialCode::new("380").is_ok());
        assert!(DialCode::new("44").is_ok());
    }

    #[test]
    fn test_dial_code_with_plus() {
        let dc = DialCode::new("+380").unwrap();
        assert_eq!(dc.as_str(), "380");
    }

    #[test]
    fn test_dial_code_trim() {
        let dc = DialCode::new("  +7  ").unwrap();
        assert_eq!(dc.as_str(), "7");
    }

    #[test]
    fn test_dial_code_empty() {
        assert!(matches!(DialCode::new(""), Err(DialCodeError::Empty)));
        assert!(matches!(DialCode::new("+"), Err(DialCodeError::Empty)));
    }

    #[test]
    fn test_dial_code_non_digit() {
        assert!(matches!(DialCode::new("12a"), Err(DialCodeError::NonDigit)));
    }

    #[test]
    fn test_dial_code_serde() {
        let dc = DialCode::new("+380").unwrap();
        let json = serde_json::to_string(&dc).unwrap();
        assert_eq!(json, r#""380""#);

        let dc: DialCode = serde_json::from_str(r#""+380""#).unwrap();
        assert_eq!(dc.as_str(), "380");
    }

    // Number tests
    #[test]
    fn test_number_valid() {
        assert!(Number::new("1234").is_ok());
        assert!(Number::new("12345678").is_ok());
        assert!(Number::new("12345678901234").is_ok());
    }

    #[test]
    fn test_number_invalid_length() {
        assert!(matches!(
            Number::new("123"),
            Err(NumberError::InvalidLength)
        ));
        assert!(matches!(
            Number::new("123456789012345"),
            Err(NumberError::InvalidLength)
        ));
    }

    #[test]
    fn test_number_non_digit() {
        assert!(matches!(Number::new("123a456"), Err(NumberError::NonDigit)));
    }

    #[test]
    fn test_number_leading_zero() {
        assert!(matches!(
            Number::new("01234567"),
            Err(NumberError::LeadingZero)
        ));
    }

    #[test]
    fn test_number_from_full_number() {
        let full = FullNumber::new("905488242474");
        let dial_code = DialCode::new("90").unwrap();
        let num = Number::from_full_number(&full, &dial_code).unwrap();
        assert_eq!(num.as_str(), "5488242474");
    }

    #[test]
    fn test_number_from_full_number_missing_dial_code() {
        let full = FullNumber::new("905488242474");
        let dial_code = DialCode::new("380").unwrap();
        assert!(matches!(
            Number::from_full_number(&full, &dial_code),
            Err(NumberError::MissingDialCode)
        ));
    }

    use keshvar::Alpha2;

    #[test]
    fn test_country_to_dial_code() {
        assert_eq!(DialCode::from(Alpha2::US.to_country()).to_string(), "1");
        assert_eq!(DialCode::from(Alpha2::UA.to_country()).to_string(), "380");
        assert_eq!(DialCode::from(Alpha2::GB.to_country()).to_string(), "44");
        assert_eq!(DialCode::from(Alpha2::TR.to_country()).to_string(), "90");
    }

    #[test]
    fn test_country_ref_to_dial_code() {
        let country = Alpha2::DE.to_country();
        let dial_code = DialCode::from(&country);
        assert_eq!(dial_code.to_string(), "49");
    }

    #[test]
    fn test_dial_code_to_country() {
        // Test unique dial codes
        let dc = DialCode::new("380").unwrap();
        let country = Country::try_from(&dc).unwrap();
        assert_eq!(country.alpha2(), Alpha2::UA);

        let dc = DialCode::new("49").unwrap();
        let country = Country::try_from(&dc).unwrap();
        assert_eq!(country.alpha2(), Alpha2::DE);

        // +44 is shared by multiple countries (GB, JE, GG, IM)
        // Just verify we get a country with dial code 44
        let dc = DialCode::new("44").unwrap();
        let country = Country::try_from(&dc).unwrap();
        assert_eq!(country.country_code(), 44);
    }

    #[test]
    fn test_dial_code_to_country_us() {
        let dc = DialCode::new("1").unwrap();
        let country = Country::try_from(&dc).unwrap();
        // keshvar's find_by_code returns a country for dial code 1
        assert_eq!(country.country_code(), 1);
    }

    #[test]
    fn test_dial_code_to_country_not_found() {
        let dc = DialCode::new("99999").unwrap();
        let result = Country::try_from(&dc);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("99999"));
    }

    #[test]
    fn test_dial_code_to_country_method() {
        let dc = DialCode::new("33").unwrap();
        let country = dc.to_country().unwrap();
        assert_eq!(country.alpha2(), Alpha2::FR);
    }

    #[test]
    fn test_round_trip_conversion() {
        let countries = [
            Alpha2::UA,
            Alpha2::GB,
            Alpha2::DE,
            Alpha2::FR,
            Alpha2::JP,
            Alpha2::AU,
            Alpha2::BR,
            Alpha2::IN,
        ];

        for alpha2 in countries {
            let original = alpha2.to_country();
            let dial_code = DialCode::from(&original);
            let converted = Country::try_from(&dial_code).unwrap();
            // Compare dial codes since some dial codes are shared
            assert_eq!(
                original.country_code(),
                converted.country_code(),
                "Round-trip failed for {:?}",
                alpha2
            );
        }
    }

    // =========================================================================
    // Comprehensive Country <-> DialCode conversion tests
    // =========================================================================

    #[test]
    fn test_all_countries_to_dial_code() {
        use keshvar::SUPPORTED_ALPHA2_LIST;

        // Test that every supported country can be converted to a dial code
        for alpha2 in SUPPORTED_ALPHA2_LIST {
            let country = alpha2.to_country();
            let dial_code = DialCode::from(&country);

            // Verify dial code is valid (non-empty, digits only)
            assert!(
                !dial_code.as_str().is_empty(),
                "Empty dial code for {:?}",
                alpha2
            );
            assert!(
                dial_code.as_str().chars().all(|c| c.is_ascii_digit()),
                "Non-digit in dial code for {:?}: {}",
                alpha2,
                dial_code
            );

            // Verify it matches the country's country_code
            assert_eq!(
                dial_code.as_str(),
                country.country_code().to_string(),
                "Dial code mismatch for {:?}",
                alpha2
            );
        }
    }

    #[test]
    fn test_country_to_dial_code_owned_vs_ref() {
        use keshvar::SUPPORTED_ALPHA2_LIST;

        // Verify that From<Country> and From<&Country> produce the same result
        for alpha2 in SUPPORTED_ALPHA2_LIST {
            let country = alpha2.to_country();
            let dial_code_from_ref = DialCode::from(&country);
            let dial_code_from_owned = DialCode::from(alpha2.to_country());

            assert_eq!(
                dial_code_from_ref, dial_code_from_owned,
                "Owned vs ref conversion mismatch for {:?}",
                alpha2
            );
        }
    }

    #[test]
    fn test_dial_code_to_country_common_codes() {
        // Test common/unique dial codes that should reliably map to a country
        let test_cases = [
            ("1", 1),     // North America (US/CA)
            ("7", 7),     // Russia/Kazakhstan
            ("20", 20),   // Egypt
            ("27", 27),   // South Africa
            ("30", 30),   // Greece
            ("31", 31),   // Netherlands
            ("32", 32),   // Belgium
            ("33", 33),   // France
            ("34", 34),   // Spain
            ("36", 36),   // Hungary
            ("39", 39),   // Italy
            ("40", 40),   // Romania
            ("41", 41),   // Switzerland
            ("43", 43),   // Austria
            ("44", 44),   // UK
            ("45", 45),   // Denmark
            ("46", 46),   // Sweden
            ("47", 47),   // Norway
            ("48", 48),   // Poland
            ("49", 49),   // Germany
            ("51", 51),   // Peru
            ("52", 52),   // Mexico
            ("53", 53),   // Cuba
            ("54", 54),   // Argentina
            ("55", 55),   // Brazil
            ("56", 56),   // Chile
            ("57", 57),   // Colombia
            ("58", 58),   // Venezuela
            ("60", 60),   // Malaysia
            ("61", 61),   // Australia
            ("62", 62),   // Indonesia
            ("63", 63),   // Philippines
            ("64", 64),   // New Zealand
            ("65", 65),   // Singapore
            ("66", 66),   // Thailand
            ("81", 81),   // Japan
            ("82", 82),   // South Korea
            ("84", 84),   // Vietnam
            ("86", 86),   // China
            ("90", 90),   // Turkey
            ("91", 91),   // India
            ("92", 92),   // Pakistan
            ("93", 93),   // Afghanistan
            ("94", 94),   // Sri Lanka
            ("95", 95),   // Myanmar
            ("98", 98),   // Iran
            ("212", 212), // Morocco
            ("213", 213), // Algeria
            ("216", 216), // Tunisia
            ("218", 218), // Libya
            ("220", 220), // Gambia
            ("221", 221), // Senegal
            ("234", 234), // Nigeria
            ("254", 254), // Kenya
            ("255", 255), // Tanzania
            ("256", 256), // Uganda
            ("260", 260), // Zambia
            ("263", 263), // Zimbabwe
            ("351", 351), // Portugal
            ("352", 352), // Luxembourg
            ("353", 353), // Ireland
            ("354", 354), // Iceland
            ("358", 358), // Finland
            ("370", 370), // Lithuania
            ("371", 371), // Latvia
            ("372", 372), // Estonia
            ("373", 373), // Moldova
            ("374", 374), // Armenia
            ("375", 375), // Belarus
            ("376", 376), // Andorra
            ("380", 380), // Ukraine
            ("381", 381), // Serbia
            ("385", 385), // Croatia
            ("386", 386), // Slovenia
            ("420", 420), // Czech Republic
            ("421", 421), // Slovakia
            ("852", 852), // Hong Kong
            ("853", 853), // Macau
            ("886", 886), // Taiwan
            ("961", 961), // Lebanon
            ("962", 962), // Jordan
            ("963", 963), // Syria
            ("964", 964), // Iraq
            ("965", 965), // Kuwait
            ("966", 966), // Saudi Arabia
            ("971", 971), // UAE
            ("972", 972), // Israel
            ("974", 974), // Qatar
        ];

        for (dial_code_str, expected_country_code) in test_cases {
            let dial_code = DialCode::new(dial_code_str).unwrap();
            let country_result = Country::try_from(&dial_code);

            assert!(
                country_result.is_ok(),
                "Failed to convert dial code {} to country",
                dial_code_str
            );

            let country = country_result.unwrap();
            assert_eq!(
                country.country_code(),
                expected_country_code,
                "Country code mismatch for dial code {}",
                dial_code_str
            );
        }
    }

    #[test]
    fn test_dial_code_to_country_owned_vs_ref() {
        // Verify that TryFrom<DialCode> and TryFrom<&DialCode> produce the same result
        let test_codes = ["1", "44", "49", "81", "86", "91", "380", "420"];

        for code_str in test_codes {
            let dial_code = DialCode::new(code_str).unwrap();
            let country_from_ref = Country::try_from(&dial_code).unwrap();
            let country_from_owned = Country::try_from(dial_code).unwrap();

            assert_eq!(
                country_from_ref.alpha2(),
                country_from_owned.alpha2(),
                "Owned vs ref conversion mismatch for dial code {}",
                code_str
            );
        }
    }

    #[test]
    fn test_dial_code_to_country_invalid_codes() {
        // Test dial codes that don't correspond to any country
        let invalid_codes = ["0", "99999", "12345", "999"];

        for code_str in invalid_codes {
            let dial_code = DialCode::new(code_str).unwrap();
            let result = Country::try_from(&dial_code);

            assert!(
                result.is_err(),
                "Expected error for invalid dial code {}, got {:?}",
                code_str,
                result
            );
        }
    }

    #[test]
    fn test_all_countries_round_trip() {
        use keshvar::SUPPORTED_ALPHA2_LIST;

        // Test round-trip conversion for all supported countries
        // Country -> DialCode -> Country should preserve the dial code
        let mut successful = 0;
        let mut failed_lookups = Vec::new();

        for alpha2 in SUPPORTED_ALPHA2_LIST {
            let original = alpha2.to_country();
            let dial_code = DialCode::from(&original);

            match Country::try_from(&dial_code) {
                Ok(converted) => {
                    // The converted country should have the same dial code
                    assert_eq!(
                        original.country_code(),
                        converted.country_code(),
                        "Round-trip dial code mismatch for {:?}",
                        alpha2
                    );
                    successful += 1;
                }
                Err(_) => {
                    // Some dial codes might not be found (edge cases)
                    failed_lookups.push((alpha2, dial_code.to_string()));
                }
            }
        }

        // Most countries should round-trip successfully
        assert!(
            successful > 200,
            "Too few successful round-trips: {} (failed: {:?})",
            successful,
            failed_lookups
        );
    }

    #[test]
    fn test_dial_code_to_country_method_consistency() {
        // Verify that dial_code.to_country() and Country::try_from(&dial_code) are consistent
        let test_codes = ["33", "49", "81", "380", "1", "44"];

        for code_str in test_codes {
            let dial_code = DialCode::new(code_str).unwrap();
            let via_method = dial_code.to_country();
            let via_try_from = Country::try_from(&dial_code);

            match (via_method, via_try_from) {
                (Ok(c1), Ok(c2)) => {
                    assert_eq!(
                        c1.alpha2(),
                        c2.alpha2(),
                        "Method vs TryFrom mismatch for {}",
                        code_str
                    );
                }
                (Err(_), Err(_)) => {
                    // Both failed, which is consistent
                }
                (Ok(_), Err(_)) | (Err(_), Ok(_)) => {
                    panic!(
                        "Inconsistent results for dial code {}: method and TryFrom differ",
                        code_str
                    );
                }
            }
        }
    }
}
