//! Types for SMS Activate API responses.

use crate::types::TaskId;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

/// Response from SMS Activate getNumberV2 API call.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetPhoneNumberResponse {
    /// Activation ID (task ID) for this phone number.
    #[serde(rename = "activationId")]
    pub task_id: TaskId,
    /// Full phone number with country code.
    pub phone_number: String,
    /// Cost of this activation in the specified currency.
    pub activation_cost: f64,
    /// Currency code (e.g., 643 for RUB).
    pub currency: i64,
    /// Country calling code.
    pub country_code: String,
    /// Whether another SMS can be requested for this activation.
    pub can_get_another_sms: bool,
    /// When the activation started.
    pub activation_time: String,
    /// When the activation expires.
    pub activation_end_time: String,
    /// Mobile operator name.
    pub activation_operator: String,
}

/// Response from SMS Activate getStatusV2 API call.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetSmsResponse {
    /// SMS data if an SMS was received.
    pub sms: Option<SmsData>,
    /// Call data if a call was received (for voice verification).
    pub call: Option<CallData>,
}

/// SMS data from verification message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SmsData {
    /// When the SMS was received.
    pub date_time: String,
    /// Verification code extracted from SMS.
    pub code: String,
    /// Full SMS message text.
    pub text: String,
}

/// Call data from voice verification.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallData {
    /// Caller number.
    pub from: String,
    /// Call message text.
    pub text: String,
    /// Verification code extracted from call.
    pub code: String,
    /// When the call was received.
    pub date_time: String,
    /// Optional URL to call recording.
    pub url: Option<String>,
    /// Number of times the code was parsed.
    pub parsing_count: u32,
}

/// Activation status codes for setStatus API call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivationStatus {
    /// Request one more code (for free).
    RequestAnotherCode,
    /// Finish the activation.
    FinishActivation,
    /// Report number has been already used and cancel the activation.
    CancelUsedNumber,
}

impl ActivationStatus {
    /// Get the numeric status code for the API.
    pub fn code(&self) -> u8 {
        match self {
            Self::RequestAnotherCode => 3,
            Self::FinishActivation => 6,
            Self::CancelUsedNumber => 8,
        }
    }
}

impl Display for ActivationStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RequestAnotherCode => write!(f, "RequestAnotherCode(3)"),
            Self::FinishActivation => write!(f, "FinishActivation(6)"),
            Self::CancelUsedNumber => write!(f, "CancelUsedNumber(8)"),
        }
    }
}

/// Response from setStatus API call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetStatusResponse {
    /// Numbers readiness confirmed.
    Ready,
    /// Waiting for new SMS.
    RetryGet,
    /// Service successfully activated.
    Activation,
    /// Activation canceled.
    Cancel,
}

impl SetStatusResponse {
    /// Parse response from raw API response text.
    pub fn from_raw(raw: &str) -> Option<Self> {
        match raw.trim() {
            "ACCESS_READY" => Some(Self::Ready),
            "ACCESS_RETRY_GET" => Some(Self::RetryGet),
            "ACCESS_ACTIVATION" => Some(Self::Activation),
            "ACCESS_CANCEL" => Some(Self::Cancel),
            _ => None,
        }
    }
}

impl Display for SetStatusResponse {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ready => write!(f, "ACCESS_READY"),
            Self::RetryGet => write!(f, "ACCESS_RETRY_GET"),
            Self::Activation => write!(f, "ACCESS_ACTIVATION"),
            Self::Cancel => write!(f, "ACCESS_CANCEL"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_activation_status_code() {
        assert_eq!(ActivationStatus::RequestAnotherCode.code(), 3);
        assert_eq!(ActivationStatus::FinishActivation.code(), 6);
        assert_eq!(ActivationStatus::CancelUsedNumber.code(), 8);
    }

    #[test]
    fn test_set_status_response_from_raw() {
        assert_eq!(
            SetStatusResponse::from_raw("ACCESS_READY"),
            Some(SetStatusResponse::Ready)
        );
        assert_eq!(
            SetStatusResponse::from_raw("ACCESS_CANCEL"),
            Some(SetStatusResponse::Cancel)
        );
        assert_eq!(SetStatusResponse::from_raw("UNKNOWN"), None);
    }

    #[test]
    fn test_get_phone_number_response_deserialization() {
        let json = r#"{
            "activationId": "123456789",
            "phoneNumber": "380501234567",
            "activationCost": 10.5,
            "currency": 643,
            "countryCode": "380",
            "canGetAnotherSms": true,
            "activationTime": "2025-01-01 12:00:00",
            "activationEndTime": "2025-01-01 12:20:00",
            "activationOperator": "kyivstar"
        }"#;

        let response: GetPhoneNumberResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.task_id.as_ref(), "123456789");
        assert_eq!(response.phone_number, "380501234567");
        assert_eq!(response.activation_cost, 10.5);
    }

    #[test]
    fn test_get_sms_response_with_code() {
        let json = r#"{
            "sms": {
                "dateTime": "2025-01-01 12:05:00",
                "code": "123456",
                "text": "Your code is: 123456"
            }
        }"#;

        let response: GetSmsResponse = serde_json::from_str(json).unwrap();
        assert!(response.sms.is_some());
        assert_eq!(response.sms.unwrap().code, "123456");
    }

    #[test]
    fn test_get_sms_response_empty() {
        let json = r#"{}"#;

        let response: GetSmsResponse = serde_json::from_str(json).unwrap();
        assert!(response.sms.is_none());
        assert!(response.call.is_none());
    }
}
