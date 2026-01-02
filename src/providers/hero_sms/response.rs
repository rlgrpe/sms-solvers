//! Response parsing for Hero SMS API.

use super::errors::{HeroSmsServiceError, parse_hero_sms_error};
use serde::de::DeserializeOwned;

/// Unified response type for Hero SMS API calls.
#[derive(Debug)]
pub enum HeroSmsResponse<T> {
    Success(T),
    Error(HeroSmsServiceError),
}

impl<T> HeroSmsResponse<T> {
    /// Convert response into a Result for ergonomic error handling.
    pub fn into_result(self) -> Result<T, HeroSmsServiceError> {
        match self {
            Self::Success(data) => Ok(data),
            Self::Error(e) => Err(e),
        }
    }

    /// Check if response is successful without consuming.
    #[allow(dead_code)]
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success(_))
    }

    /// Get reference to success data if available.
    #[allow(dead_code)]
    pub fn as_success(&self) -> Option<&T> {
        match self {
            Self::Success(data) => Some(data),
            Self::Error(_) => None,
        }
    }
}

impl<T: DeserializeOwned> HeroSmsResponse<T> {
    /// Parse Hero SMS response from raw text.
    ///
    /// This handles the Hero SMS API pattern where errors are returned
    /// as plain text error codes (e.g., "NO_NUMBERS", "BAD_KEY") and
    /// success responses are JSON.
    pub fn from_text(text: &str) -> Result<Self, serde_json::Error> {
        // Check if this is an error response
        if let Some(error) = parse_hero_sms_error(text) {
            return Ok(Self::Error(error));
        }

        // Try to parse as success response
        let data = serde_json::from_str::<T>(text)?;
        Ok(Self::Success(data))
    }
}

/// Response type for setStatus API which returns plain text.
#[derive(Debug)]
pub enum HeroSmsTextResponse {
    Success(String),
    Error(HeroSmsServiceError),
}

impl HeroSmsTextResponse {
    /// Parse response from raw text.
    pub fn from_text(text: &str) -> Self {
        if let Some(error) = parse_hero_sms_error(text) {
            Self::Error(error)
        } else {
            Self::Success(text.to_string())
        }
    }

    /// Convert to Result.
    pub fn into_result(self) -> Result<String, HeroSmsServiceError> {
        match self {
            Self::Success(text) => Ok(text),
            Self::Error(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::hero_sms::errors::HeroSmsErrorCode;
    use crate::providers::hero_sms::types::GetPhoneNumberResponse;

    #[test]
    fn test_json_response_success() {
        let json = r#"{
            "activationId": "123456",
            "phoneNumber": "79001234567",
            "activationCost": 10.5,
            "currency": 643,
            "countryCode": "7",
            "canGetAnotherSms": true,
            "activationTime": "2025-01-01 12:00:00",
            "activationEndTime": "2025-01-01 12:20:00",
            "activationOperator": "mts"
        }"#;

        let response = HeroSmsResponse::<GetPhoneNumberResponse>::from_text(json).unwrap();
        assert!(response.is_success());
        let data = response.into_result().unwrap();
        assert_eq!(data.phone_number, "79001234567");
    }

    #[test]
    fn test_json_response_error() {
        let text = "NO_NUMBERS";
        let response = HeroSmsResponse::<GetPhoneNumberResponse>::from_text(text).unwrap();
        assert!(!response.is_success());

        match response.into_result() {
            Err(error) => {
                assert_eq!(error.code, HeroSmsErrorCode::NoNumbers);
            }
            Ok(_) => panic!("Expected error"),
        }
    }

    #[test]
    fn test_text_response_success() {
        let text = "ACCESS_READY";
        let response = HeroSmsTextResponse::from_text(text);

        match response {
            HeroSmsTextResponse::Success(s) => assert_eq!(s, "ACCESS_READY"),
            HeroSmsTextResponse::Error(_) => panic!("Expected success"),
        }
    }

    #[test]
    fn test_text_response_error() {
        let text = "BAD_KEY";
        let response = HeroSmsTextResponse::from_text(text);

        match response {
            HeroSmsTextResponse::Success(_) => panic!("Expected error"),
            HeroSmsTextResponse::Error(e) => {
                assert_eq!(e.code, HeroSmsErrorCode::BadKey);
            }
        }
    }
}
