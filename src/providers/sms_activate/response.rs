//! Response parsing for SMS Activate API.

use super::errors::{SmsActivateServiceError, parse_sms_activate_error};
use serde::de::DeserializeOwned;

/// Unified response type for SMS Activate API calls.
#[derive(Debug)]
pub enum SmsActivateResponse<T> {
    Success(T),
    Error(SmsActivateServiceError),
}

impl<T> SmsActivateResponse<T> {
    /// Convert response into a Result for ergonomic error handling.
    pub fn into_result(self) -> Result<T, SmsActivateServiceError> {
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

impl<T: DeserializeOwned> SmsActivateResponse<T> {
    /// Parse SMS Activate response from raw text.
    ///
    /// This handles the SMS Activate API pattern where errors are returned
    /// as plain text error codes (e.g., "NO_NUMBERS", "BAD_KEY") and
    /// success responses are JSON.
    pub fn from_text(text: &str) -> Result<Self, serde_json::Error> {
        // Check if this is an error response
        if let Some(error) = parse_sms_activate_error(text) {
            return Ok(Self::Error(error));
        }

        // Try to parse as success response
        let data = serde_json::from_str::<T>(text)?;
        Ok(Self::Success(data))
    }
}

/// Response type for setStatus API which returns plain text.
#[derive(Debug)]
pub enum SmsActivateTextResponse {
    Success(String),
    Error(SmsActivateServiceError),
}

impl SmsActivateTextResponse {
    /// Parse response from raw text.
    pub fn from_text(text: &str) -> Self {
        if let Some(error) = parse_sms_activate_error(text) {
            Self::Error(error)
        } else {
            Self::Success(text.to_string())
        }
    }

    /// Convert to Result.
    pub fn into_result(self) -> Result<String, SmsActivateServiceError> {
        match self {
            Self::Success(text) => Ok(text),
            Self::Error(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::sms_activate::errors::SmsActivateErrorCode;
    use crate::providers::sms_activate::types::GetPhoneNumberResponse;

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

        let response = SmsActivateResponse::<GetPhoneNumberResponse>::from_text(json).unwrap();
        assert!(response.is_success());
        let data = response.into_result().unwrap();
        assert_eq!(data.phone_number, "79001234567");
    }

    #[test]
    fn test_json_response_error() {
        let text = "NO_NUMBERS";
        let response = SmsActivateResponse::<GetPhoneNumberResponse>::from_text(text).unwrap();
        assert!(!response.is_success());

        match response.into_result() {
            Err(error) => {
                assert_eq!(error.code, SmsActivateErrorCode::NoNumbers);
            }
            Ok(_) => panic!("Expected error"),
        }
    }

    #[test]
    fn test_text_response_success() {
        let text = "ACCESS_READY";
        let response = SmsActivateTextResponse::from_text(text);

        match response {
            SmsActivateTextResponse::Success(s) => assert_eq!(s, "ACCESS_READY"),
            SmsActivateTextResponse::Error(_) => panic!("Expected success"),
        }
    }

    #[test]
    fn test_text_response_error() {
        let text = "BAD_KEY";
        let response = SmsActivateTextResponse::from_text(text);

        match response {
            SmsActivateTextResponse::Success(_) => panic!("Expected error"),
            SmsActivateTextResponse::Error(e) => {
                assert_eq!(e.code, SmsActivateErrorCode::BadKey);
            }
        }
    }
}
