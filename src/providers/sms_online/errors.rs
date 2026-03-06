//! Error types for SMS.online provider.

use crate::errors::RetryableError;
use crate::types::TaskId;
use regex::Regex;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::{self, Display, Formatter};
use std::sync::LazyLock;
use std::time::Duration;
use thiserror::Error;

#[cfg(feature = "tracing")]
use tracing::warn;

/// Error codes returned by SMS.online service API.
#[derive(Debug, Clone, PartialEq)]
pub enum SmsOnlineErrorCode {
    // Retryable
    NoNumbers,
    ErrorSql,
    ChannelsLimit,

    // Non-retryable
    NoActivation,
    BadKey,
    BadAction,
    BadService,
    BadCountry,
    NoBalance,
    AccountInactive,
    Banned { until: String },
    EarlyCancelDenied,
    BadStatus,
    WrongActivationId,

    Unknown { raw: String },
}

impl SmsOnlineErrorCode {
    pub fn code_name(&self) -> &str {
        match self {
            Self::NoNumbers => "NO_NUMBERS",
            Self::ErrorSql => "ERROR_SQL",
            Self::ChannelsLimit => "CHANNELS_LIMIT",
            Self::NoActivation => "NO_ACTIVATION",
            Self::BadKey => "BAD_KEY",
            Self::BadAction => "BAD_ACTION",
            Self::BadService => "BAD_SERVICE",
            Self::BadCountry => "BAD_COUNTRY",
            Self::NoBalance => "NO_BALANCE",
            Self::AccountInactive => "ACCOUNT_INACTIVE",
            Self::Banned { .. } => "BANNED",
            Self::EarlyCancelDenied => "EARLY_CANCEL_DENIED",
            Self::BadStatus => "BAD_STATUS",
            Self::WrongActivationId => "WRONG_ACTIVATION_ID",
            Self::Unknown { raw } => raw.as_str(),
        }
    }

    pub fn description(&self) -> String {
        match self {
            Self::NoNumbers => "No numbers available".to_string(),
            Self::ErrorSql => "Internal SQL error on service side".to_string(),
            Self::ChannelsLimit => "Account blocked by channel limits".to_string(),
            Self::NoActivation => "Activation does not exist".to_string(),
            Self::BadKey => "Invalid API key".to_string(),
            Self::BadAction => "Incorrect action".to_string(),
            Self::BadService => "Incorrect service code".to_string(),
            Self::BadCountry => "Incorrect country code".to_string(),
            Self::NoBalance => "Insufficient balance".to_string(),
            Self::AccountInactive => "Account inactive".to_string(),
            Self::Banned { until } => format!("Account banned until {until}"),
            Self::EarlyCancelDenied => "Cannot cancel number in first 2 minutes".to_string(),
            Self::BadStatus => "Incorrect activation status".to_string(),
            Self::WrongActivationId => "Incorrect activation id".to_string(),
            Self::Unknown { raw } => format!("Unknown error: {raw}"),
        }
    }

    pub fn from_raw(raw: &str) -> Option<Self> {
        let s = raw.trim();

        let code = match s {
            "NO_NUMBERS" => Self::NoNumbers,
            "ERROR_SQL" => Self::ErrorSql,
            "CHANNELS_LIMIT" => Self::ChannelsLimit,
            "NO_ACTIVATION" => Self::NoActivation,
            "BAD_KEY" => Self::BadKey,
            "BAD_ACTION" => Self::BadAction,
            "BAD_SERVICE" => Self::BadService,
            "BAD_COUNTRY" => Self::BadCountry,
            "NO_BALANCE" => Self::NoBalance,
            "ACCOUNT_INACTIVE" => Self::AccountInactive,
            "EARLY_CANCEL_DENIED" => Self::EarlyCancelDenied,
            "BAD_STATUS" => Self::BadStatus,
            "WRONG_ACTIVATION_ID" => Self::WrongActivationId,
            _ => return Self::parse_parametrized_error(s),
        };

        Some(code)
    }

    fn parse_parametrized_error(s: &str) -> Option<Self> {
        static RE_BANNED: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r#"^BANNED\s*:\s*['"]([^'"]+)['"]$"#).unwrap());
        if let Some(cap) = RE_BANNED.captures(s) {
            let until = cap.get(1).map(|m| m.as_str().to_string())?;
            return Some(Self::Banned { until });
        }

        if Self::looks_like_error_code(s) {
            return Some(Self::Unknown { raw: s.to_string() });
        }

        None
    }

    fn looks_like_error_code(s: &str) -> bool {
        if s.is_empty() {
            return false;
        }

        // Success/status responses that are not errors.
        if s.starts_with("ACCESS_") || s.starts_with("STATUS_") {
            return false;
        }

        let known_error_prefixes = [
            "NO_",
            "ERROR_",
            "BAD_",
            "WRONG_",
            "EARLY_",
            "BANNED",
            "CHANNELS_",
            "ACCOUNT_",
        ];

        known_error_prefixes
            .iter()
            .any(|prefix| s.starts_with(prefix))
    }

    pub fn is_retryable(&self) -> bool {
        matches!(self, Self::NoNumbers | Self::ErrorSql | Self::ChannelsLimit)
    }

    pub fn should_retry_operation(&self) -> bool {
        match self {
            Self::NoNumbers | Self::ErrorSql | Self::ChannelsLimit => true,
            Self::NoActivation | Self::WrongActivationId => true,
            Self::BadKey
            | Self::BadAction
            | Self::BadService
            | Self::BadCountry
            | Self::NoBalance
            | Self::AccountInactive
            | Self::Banned { .. }
            | Self::EarlyCancelDenied
            | Self::BadStatus
            | Self::Unknown { .. } => false,
        }
    }
}

impl Display for SmsOnlineErrorCode {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.code_name())
    }
}

impl Serialize for SmsOnlineErrorCode {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.code_name())
    }
}

impl<'de> Deserialize<'de> for SmsOnlineErrorCode {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(Self::from_raw(&s).unwrap_or(Self::Unknown { raw: s }))
    }
}

/// Error returned by SMS.online service.
#[derive(Debug, Clone, Error)]
#[error("SMS.online service error: code={code}, description={description}")]
pub struct SmsOnlineServiceError {
    pub code: SmsOnlineErrorCode,
    pub description: String,
    pub raw: String,
}

impl SmsOnlineServiceError {
    pub fn new(code: SmsOnlineErrorCode, raw: String) -> Self {
        let description = code.description();
        Self {
            code,
            description,
            raw,
        }
    }
}

pub(crate) fn parse_sms_online_error(raw: &str) -> Option<SmsOnlineServiceError> {
    let code = SmsOnlineErrorCode::from_raw(raw)?;
    let error = SmsOnlineServiceError::new(code, raw.to_string());

    #[cfg(feature = "tracing")]
    warn!(
        code = %error.code,
        description = %error.description,
        raw = %raw,
        "SMS.online service returned error"
    );

    Some(error)
}

/// Main error type for SMS.online client operations.
#[derive(Debug, Error)]
pub enum SmsOnlineError {
    #[error("Failed to build HTTP client: {0}")]
    BuildHttpClient(#[source] reqwest::Error),

    #[error("Error building SMS.online request URL: {0}")]
    BuildRequestUrl(#[source] serde_urlencoded::ser::Error),

    #[error("Failed to send HTTP request: {0}")]
    HttpRequest(#[from] reqwest_middleware::Error),

    #[error("Failed to parse response: {0}")]
    ParseResponse(#[source] reqwest::Error),

    #[error("SMS.online service error: {0}")]
    Service(#[source] SmsOnlineServiceError),

    #[error("Activation was canceled (STATUS_CANCEL); task id: {task_id}")]
    ActivationCanceled { task_id: TaskId },

    #[error(
        "Timeout waiting for SMS after {:.1}s; Task id: {task_id}",
        timeout.as_secs_f64()
    )]
    SolutionTimeout { timeout: Duration, task_id: TaskId },

    #[error("No SMS.online mapping for country {}", country.iso_short_name())]
    CountryMapping { country: Box<keshvar::Country> },

    #[error("Failed to parse getNumber response: {raw}")]
    FailedToParseGetNumberResponse { raw: String },

    #[error("Failed to parse getStatus response: {raw}")]
    FailedToParseGetStatusResponse { raw: String },

    #[error("Failed to parse setStatus response: {raw}")]
    FailedToParseSetStatusResponse { raw: String },

    #[error("Invalid API URL: {0}")]
    InvalidUrl(#[from] url::ParseError),
}

pub type Result<T> = std::result::Result<T, SmsOnlineError>;

impl RetryableError for SmsOnlineError {
    fn is_retryable(&self) -> bool {
        match self {
            SmsOnlineError::Service(error) => error.code.is_retryable(),
            SmsOnlineError::HttpRequest(_) => true,
            SmsOnlineError::BuildHttpClient(_)
            | SmsOnlineError::BuildRequestUrl(_)
            | SmsOnlineError::ParseResponse(_)
            | SmsOnlineError::ActivationCanceled { .. }
            | SmsOnlineError::SolutionTimeout { .. }
            | SmsOnlineError::CountryMapping { .. }
            | SmsOnlineError::FailedToParseGetNumberResponse { .. }
            | SmsOnlineError::FailedToParseGetStatusResponse { .. }
            | SmsOnlineError::FailedToParseSetStatusResponse { .. }
            | SmsOnlineError::InvalidUrl(_) => false,
        }
    }

    fn should_retry_operation(&self) -> bool {
        match self {
            SmsOnlineError::Service(error) => error.code.should_retry_operation(),
            SmsOnlineError::HttpRequest(_) => true,
            SmsOnlineError::SolutionTimeout { .. } => true,
            SmsOnlineError::BuildHttpClient(_)
            | SmsOnlineError::BuildRequestUrl(_)
            | SmsOnlineError::ParseResponse(_)
            | SmsOnlineError::ActivationCanceled { .. }
            | SmsOnlineError::CountryMapping { .. }
            | SmsOnlineError::FailedToParseGetNumberResponse { .. }
            | SmsOnlineError::FailedToParseGetStatusResponse { .. }
            | SmsOnlineError::FailedToParseSetStatusResponse { .. }
            | SmsOnlineError::InvalidUrl(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_errors() {
        let test_cases = vec![
            ("NO_ACTIVATION", SmsOnlineErrorCode::NoActivation),
            ("ERROR_SQL", SmsOnlineErrorCode::ErrorSql),
            ("BAD_KEY", SmsOnlineErrorCode::BadKey),
            ("NO_NUMBERS", SmsOnlineErrorCode::NoNumbers),
            ("CHANNELS_LIMIT", SmsOnlineErrorCode::ChannelsLimit),
            ("ACCOUNT_INACTIVE", SmsOnlineErrorCode::AccountInactive),
            ("NO_BALANCE", SmsOnlineErrorCode::NoBalance),
        ];

        for (input, expected) in test_cases {
            let error = parse_sms_online_error(input).unwrap();
            assert_eq!(error.code, expected);
            assert_eq!(error.raw, input);
        }
    }

    #[test]
    fn test_parse_banned_error() {
        let input = "BANNED:'2025-12-31 23:59:59'";
        let error = parse_sms_online_error(input).unwrap();
        assert_eq!(
            error.code,
            SmsOnlineErrorCode::Banned {
                until: "2025-12-31 23:59:59".to_string()
            }
        );
    }

    #[test]
    fn test_success_and_status_not_treated_as_errors() {
        let responses = [
            "ACCESS_NUMBER:123:79001234567",
            "ACCESS_RETRY_GET",
            "ACCESS_ACTIVATION",
            "ACCESS_CANCEL",
            "STATUS_WAIT_CODE",
            "STATUS_WAIT_RESEND",
            "STATUS_OK:1234",
            "STATUS_CANCEL",
        ];

        for response in responses {
            assert!(
                parse_sms_online_error(response).is_none(),
                "'{response}' should not be parsed as service error"
            );
        }
    }
}
