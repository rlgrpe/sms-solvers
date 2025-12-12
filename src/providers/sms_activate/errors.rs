//! Error types for SMS Activate provider.

use crate::errors::RetryableError;
use crate::types::TaskId;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::{self, Display, Formatter};
use std::time::Duration;
use thiserror::Error;

#[cfg(feature = "tracing")]
use tracing::warn;

/// Error codes returned by SMS Activate service API.
#[derive(Debug, Clone, PartialEq)]
pub enum SmsActivateErrorCode {
    // === Transient / Server Errors (Retryable) ===
    /// No numbers available for the requested country/service.
    NoNumbers,
    /// Internal SQL error on service side.
    ErrorSql,
    /// Account blocked by channel limits (temporary).
    ChannelsLimit,

    // === Fatal / Client Errors (Non-retryable) ===
    /// Activation with this id does not exist.
    NoActivation,
    /// Invalid API key.
    BadKey,
    /// Incorrect action.
    BadAction,
    /// Order already exists.
    OrderAlreadyExists,
    /// Incorrect service code.
    BadService,
    /// Incorrect excluding prefixes.
    WrongExceptionPhone,
    /// Account banned until specified datetime.
    Banned { until: String },
    /// Maximum price is less than allowed minimum.
    WrongMaxPrice { min: Option<f64> },
    /// Not allowed to cancel within first 2 minutes.
    EarlyCancelDenied,
    /// Incorrect status.
    BadStatus,
    /// Invalid activation ID or ID is not a number.
    WrongActivationId,

    /// Unknown error code from service.
    Unknown { raw: String },
}

impl SmsActivateErrorCode {
    /// Returns the API error code string representation.
    pub fn code_name(&self) -> &str {
        match self {
            Self::NoNumbers => "NO_NUMBERS",
            Self::ErrorSql => "ERROR_SQL",
            Self::ChannelsLimit => "CHANNELS_LIMIT",
            Self::NoActivation => "NO_ACTIVATION",
            Self::BadKey => "BAD_KEY",
            Self::BadAction => "BAD_ACTION",
            Self::OrderAlreadyExists => "ORDER_ALREADY_EXISTS",
            Self::BadService => "BAD_SERVICE",
            Self::WrongExceptionPhone => "WRONG_EXCEPTION_PHONE",
            Self::Banned { .. } => "BANNED",
            Self::WrongMaxPrice { .. } => "WRONG_MAX_PRICE",
            Self::EarlyCancelDenied => "EARLY_CANCEL_DENIED",
            Self::BadStatus => "BAD_STATUS",
            Self::WrongActivationId => "WRONG_ACTIVATION_ID",
            Self::Unknown { raw } => raw.as_str(),
        }
    }

    /// Returns human-readable description.
    pub fn description(&self) -> String {
        match self {
            Self::NoNumbers => "No numbers available".to_string(),
            Self::ErrorSql => "Internal SQL error on service side".to_string(),
            Self::ChannelsLimit => "Account blocked by channel limits".to_string(),
            Self::NoActivation => "Activation does not exist".to_string(),
            Self::BadKey => "Invalid API key".to_string(),
            Self::BadAction => "Incorrect action".to_string(),
            Self::OrderAlreadyExists => "Order already exists".to_string(),
            Self::BadService => "Incorrect service code".to_string(),
            Self::WrongExceptionPhone => "Incorrect excluding prefixes".to_string(),
            Self::Banned { until } => format!("Account banned until {}", until),
            Self::WrongMaxPrice { min } => match min {
                Some(v) => format!("Maximum price is less than allowed minimum: {}", v),
                None => "Maximum price is less than allowed minimum".to_string(),
            },
            Self::EarlyCancelDenied => "Not allowed to cancel within first 2 minutes".to_string(),
            Self::BadStatus => "Incorrect status".to_string(),
            Self::WrongActivationId => "Invalid activation ID".to_string(),
            Self::Unknown { raw } => format!("Unknown error: {}", raw),
        }
    }

    /// Parse error code from raw API response.
    pub fn from_raw(raw: &str) -> Option<Self> {
        let s = raw.trim();

        let code = match s {
            "NO_NUMBERS" => Self::NoNumbers,
            "ERROR_SQL" => Self::ErrorSql,
            "CHANNELS_LIMIT" => Self::ChannelsLimit,
            "NO_ACTIVATION" => Self::NoActivation,
            "BAD_KEY" => Self::BadKey,
            "BAD_ACTION" => Self::BadAction,
            "ORDER_ALREADY_EXISTS" => Self::OrderAlreadyExists,
            "BAD_SERVICE" => Self::BadService,
            "WRONG_EXCEPTION_PHONE" => Self::WrongExceptionPhone,
            "EARLY_CANCEL_DENIED" => Self::EarlyCancelDenied,
            "BAD_STATUS" => Self::BadStatus,
            "WRONG_ACTIVATION_ID" => Self::WrongActivationId,
            _ => return Self::parse_parametrized_error(s),
        };

        Some(code)
    }

    /// Parse error codes with parameters (BANNED, WRONG_MAX_PRICE).
    fn parse_parametrized_error(s: &str) -> Option<Self> {
        // BANNED:'YYYY-m-d H-i-s'
        static RE_BANNED: Lazy<Regex> =
            Lazy::new(|| Regex::new(r#"^BANNED\s*:\s*['"]([^'"]+)['"]$"#).unwrap());
        if let Some(cap) = RE_BANNED.captures(s) {
            let until = cap.get(1).unwrap().as_str().to_string();
            return Some(Self::Banned { until });
        }

        // WRONG_MAX_PRICE:<num>
        static RE_WRONG_MAX_PRICE: Lazy<Regex> =
            Lazy::new(|| Regex::new(r#"^WRONG_MAX_PRICE\s*:\s*([0-9]+(?:\.[0-9]+)?)$"#).unwrap());
        if let Some(cap) = RE_WRONG_MAX_PRICE.captures(s) {
            let min = cap.get(1).and_then(|m| m.as_str().parse::<f64>().ok());
            return Some(Self::WrongMaxPrice { min });
        }

        // Check if this looks like an error code
        if Self::looks_like_error_code(s) {
            return Some(Self::Unknown { raw: s.to_string() });
        }

        None
    }

    /// Check if string looks like an error code format.
    fn looks_like_error_code(s: &str) -> bool {
        if s.is_empty() {
            return false;
        }

        // Filter out known success responses
        if s.starts_with("ACCESS_") {
            return false;
        }

        // Known error code prefixes
        let known_error_prefixes = [
            "NO_",
            "ERROR_",
            "BAD_",
            "WRONG_",
            "EARLY_",
            "BANNED",
            "CHANNELS_",
            "ORDER_",
        ];

        for prefix in &known_error_prefixes {
            if s.starts_with(prefix) {
                return true;
            }
        }

        false
    }

    /// Returns true if this error is transient and the operation should be retried.
    pub fn is_retryable(&self) -> bool {
        matches!(self, Self::NoNumbers | Self::ErrorSql | Self::ChannelsLimit)
    }

    /// Returns true if a fresh operation might succeed.
    pub fn should_retry_operation(&self) -> bool {
        match self {
            // Transient errors - retry
            Self::NoNumbers | Self::ErrorSql | Self::ChannelsLimit => true,
            // Activation-specific errors - fresh attempt might work
            Self::NoActivation | Self::WrongActivationId => true,
            // Account/configuration issues - won't work until fixed
            Self::BadKey
            | Self::BadAction
            | Self::OrderAlreadyExists
            | Self::BadService
            | Self::WrongExceptionPhone
            | Self::Banned { .. }
            | Self::WrongMaxPrice { .. }
            | Self::EarlyCancelDenied
            | Self::BadStatus => false,
            // Unknown errors - conservative: don't retry
            Self::Unknown { .. } => false,
        }
    }
}

impl Display for SmsActivateErrorCode {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.code_name())
    }
}

impl Serialize for SmsActivateErrorCode {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.code_name())
    }
}

impl<'de> Deserialize<'de> for SmsActivateErrorCode {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(Self::from_raw(&s).unwrap_or(Self::Unknown { raw: s }))
    }
}

/// Error returned by SMS Activate service.
#[derive(Debug, Clone, Error)]
#[error("SMS Activate service error: code={code}, description={description}")]
pub struct SmsActivateServiceError {
    /// Error code from the service.
    pub code: SmsActivateErrorCode,
    /// Human-readable description.
    pub description: String,
    /// Original raw response text.
    pub raw: String,
}

impl SmsActivateServiceError {
    /// Create new service error from code and raw response.
    pub fn new(code: SmsActivateErrorCode, raw: String) -> Self {
        let description = code.description();
        Self {
            code,
            description,
            raw,
        }
    }
}

/// Parse SMS Activate error from API response text.
pub(crate) fn parse_sms_activate_error(raw: &str) -> Option<SmsActivateServiceError> {
    let code = SmsActivateErrorCode::from_raw(raw)?;
    let error = SmsActivateServiceError::new(code, raw.to_string());

    #[cfg(feature = "tracing")]
    warn!(
        code = %error.code,
        description = %error.description,
        raw = %raw,
        "SMS Activate service returned error"
    );

    Some(error)
}

/// Main error type for SMS Activate client operations.
#[derive(Debug, Error)]
pub enum SmsActivateError {
    /// Failed to build HTTP client.
    #[error("Failed to build HTTP client: {0}")]
    BuildHttpClient(#[source] reqwest::Error),

    /// Error building SMS Activate request URL.
    #[error("Error building SMS Activate request URL: {0}")]
    BuildRequestUrl(#[source] serde_urlencoded::ser::Error),

    /// Failed to send HTTP request.
    #[error("Failed to send HTTP request: {0}")]
    HttpRequest(#[from] reqwest_middleware::Error),

    /// Failed to parse response.
    #[error("Failed to parse response: {0}")]
    ParseResponse(#[source] reqwest::Error),

    /// SMS Activate service error.
    #[error("SMS Activate service error: {0}")]
    Service(#[source] SmsActivateServiceError),

    /// Timeout waiting for SMS.
    #[error(
        "Timeout waiting for SMS after {:.1}s; Task id: {task_id}",
        timeout.as_secs_f64()
    )]
    SolutionTimeout { timeout: Duration, task_id: TaskId },

    /// Failed to map country code.
    #[error("No SMS-Activate mapping for country {country}")]
    CountryMapping { country: isocountry::CountryCode },

    /// Failed to parse SetStatus response.
    #[error("Failed to parse SetStatus response: {raw}")]
    FailedToParseSetStatusResponse { raw: String },

    /// Failed to deserialize JSON response.
    #[error("Failed to deserialize JSON response: {0}")]
    DeserializeJson(#[source] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, SmsActivateError>;

impl RetryableError for SmsActivateError {
    fn is_retryable(&self) -> bool {
        match self {
            // Retryable service errors - temporary unavailability
            SmsActivateError::Service(error) => error.code.is_retryable(),
            // Retryable HTTP/network errors
            SmsActivateError::HttpRequest(_) => true,
            // Non-retryable errors - permanent configuration or logic errors
            SmsActivateError::BuildHttpClient(_)
            | SmsActivateError::BuildRequestUrl(_)
            | SmsActivateError::ParseResponse(_)
            | SmsActivateError::SolutionTimeout { .. }
            | SmsActivateError::CountryMapping { .. }
            | SmsActivateError::FailedToParseSetStatusResponse { .. }
            | SmsActivateError::DeserializeJson(_) => false,
        }
    }

    fn should_retry_operation(&self) -> bool {
        match self {
            // Service errors have their own logic
            SmsActivateError::Service(error) => error.code.should_retry_operation(),
            // HTTP errors - retry the operation
            SmsActivateError::HttpRequest(_) => true,
            // Timeouts - fresh attempt might work
            SmsActivateError::SolutionTimeout { .. } => true,
            // Configuration errors - won't work until fixed
            SmsActivateError::BuildHttpClient(_)
            | SmsActivateError::BuildRequestUrl(_)
            | SmsActivateError::ParseResponse(_)
            | SmsActivateError::CountryMapping { .. }
            | SmsActivateError::FailedToParseSetStatusResponse { .. }
            | SmsActivateError::DeserializeJson(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_errors() {
        let test_cases = vec![
            ("NO_ACTIVATION", SmsActivateErrorCode::NoActivation),
            ("ERROR_SQL", SmsActivateErrorCode::ErrorSql),
            ("BAD_KEY", SmsActivateErrorCode::BadKey),
            ("NO_NUMBERS", SmsActivateErrorCode::NoNumbers),
            ("CHANNELS_LIMIT", SmsActivateErrorCode::ChannelsLimit),
        ];

        for (input, expected) in test_cases {
            let error = parse_sms_activate_error(input).unwrap();
            assert_eq!(error.code, expected);
            assert_eq!(error.raw, input);
        }
    }

    #[test]
    fn test_parse_banned_error() {
        let input = "BANNED:'2025-12-31 23:59:59'";
        let error = parse_sms_activate_error(input).unwrap();
        assert_eq!(
            error.code,
            SmsActivateErrorCode::Banned {
                until: "2025-12-31 23:59:59".to_string()
            }
        );
    }

    #[test]
    fn test_parse_wrong_max_price() {
        let input = "WRONG_MAX_PRICE:10.5";
        let error = parse_sms_activate_error(input).unwrap();
        assert_eq!(
            error.code,
            SmsActivateErrorCode::WrongMaxPrice { min: Some(10.5) }
        );
    }

    #[test]
    fn test_success_responses_not_treated_as_errors() {
        let success_responses = vec![
            "ACCESS_READY",
            "ACCESS_RETRY_GET",
            "ACCESS_ACTIVATION",
            "ACCESS_CANCEL",
        ];

        for response in success_responses {
            assert!(
                parse_sms_activate_error(response).is_none(),
                "Success response '{}' should not be treated as an error",
                response
            );
        }
    }

    #[test]
    fn test_retryable_errors() {
        assert!(SmsActivateErrorCode::NoNumbers.is_retryable());
        assert!(SmsActivateErrorCode::ErrorSql.is_retryable());
        assert!(SmsActivateErrorCode::ChannelsLimit.is_retryable());

        assert!(!SmsActivateErrorCode::BadKey.is_retryable());
        assert!(!SmsActivateErrorCode::NoActivation.is_retryable());
    }
}
