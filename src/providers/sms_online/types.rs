//! Types for SMS.online API responses.

use crate::types::TaskId;
use std::fmt::{Display, Formatter};

/// Response from sms.online `getNumber` API call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetPhoneNumberResponse {
    pub task_id: TaskId,
    pub phone_number: String,
}

impl GetPhoneNumberResponse {
    /// Parse `ACCESS_NUMBER:ID:NUMBER`.
    pub fn from_raw(raw: &str) -> Option<Self> {
        let trimmed = raw.trim();
        let mut parts = trimmed.split(':');

        match (parts.next(), parts.next(), parts.next(), parts.next()) {
            (Some("ACCESS_NUMBER"), Some(task_id), Some(phone_number), None) => Some(Self {
                task_id: TaskId::from(task_id),
                phone_number: phone_number.to_string(),
            }),
            _ => None,
        }
    }
}

/// Parsed result of sms.online `getStatus` call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GetSmsStatusResponse {
    WaitCode,
    WaitResend,
    Cancel,
    Ok { code: String },
}

impl GetSmsStatusResponse {
    pub fn from_raw(raw: &str) -> Option<Self> {
        let trimmed = raw.trim();
        if trimmed == "STATUS_WAIT_CODE" {
            return Some(Self::WaitCode);
        }
        if trimmed == "STATUS_WAIT_RESEND" {
            return Some(Self::WaitResend);
        }
        if trimmed == "STATUS_CANCEL" {
            return Some(Self::Cancel);
        }
        if let Some(code) = trimmed.strip_prefix("STATUS_OK:") {
            return Some(Self::Ok {
                code: code.to_string(),
            });
        }
        None
    }
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

/// Optional parameters for the `getNumber` API request.
///
/// These options allow fine-tuning which phone number is returned by the provider.
///
/// # Example
///
/// ```rust,ignore
/// use sms_solvers::sms_online::GetNumberOptions;
///
/// let options = GetNumberOptions::new()
///     .with_operator("megafon,beeline");
/// ```
#[derive(Debug, Clone, Default)]
pub struct GetNumberOptions {
    /// Comma-separated list of mobile operators to filter by.
    pub operator: Option<String>,
    /// Optional referral id.
    pub ref_id: Option<String>,
    /// Activation type (0 - sms, 1 - by number, 2 - by voice).
    pub activation_type: Option<ActivationType>,
    /// Provider selection (`pr1`, `pr2`, `pr3`, ...).
    pub provider: Option<ProviderSelection>,
}

impl GetNumberOptions {
    /// Create new empty options.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the operator filter (comma-separated list).
    pub fn with_operator(mut self, operator: impl Into<String>) -> Self {
        self.operator = Some(operator.into());
        self
    }

    /// Set referral id.
    pub fn with_ref_id(mut self, ref_id: impl Into<String>) -> Self {
        self.ref_id = Some(ref_id.into());
        self
    }

    /// Set activation type.
    pub fn with_activation_type(mut self, activation_type: ActivationType) -> Self {
        self.activation_type = Some(activation_type);
        self
    }

    /// Set provider selection.
    pub fn with_provider(mut self, provider: ProviderSelection) -> Self {
        self.provider = Some(provider);
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivationType {
    Sms,
    Number,
    Voice,
}

impl ActivationType {
    pub fn code(self) -> u8 {
        match self {
            Self::Sms => 0,
            Self::Number => 1,
            Self::Voice => 2,
        }
    }
}

impl Display for ActivationType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.code())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderSelection {
    Pr1,
    Pr2,
    Pr3,
    Other(String),
}

impl ProviderSelection {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Pr1 => "pr1",
            Self::Pr2 => "pr2",
            Self::Pr3 => "pr3",
            Self::Other(value) => value.as_str(),
        }
    }
}

impl Display for ProviderSelection {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
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
    fn test_get_phone_number_response_from_raw() {
        let response =
            GetPhoneNumberResponse::from_raw("ACCESS_NUMBER:123456789:380501234567").unwrap();
        assert_eq!(response.task_id.as_ref(), "123456789");
        assert_eq!(response.phone_number, "380501234567");
    }

    #[test]
    fn test_get_sms_status_response_from_raw() {
        assert_eq!(
            GetSmsStatusResponse::from_raw("STATUS_WAIT_CODE"),
            Some(GetSmsStatusResponse::WaitCode)
        );
        assert_eq!(
            GetSmsStatusResponse::from_raw("STATUS_WAIT_RESEND"),
            Some(GetSmsStatusResponse::WaitResend)
        );
        assert_eq!(
            GetSmsStatusResponse::from_raw("STATUS_CANCEL"),
            Some(GetSmsStatusResponse::Cancel)
        );
        assert_eq!(
            GetSmsStatusResponse::from_raw("STATUS_OK:123456"),
            Some(GetSmsStatusResponse::Ok {
                code: "123456".to_string()
            })
        );
    }

    #[test]
    fn test_activation_type_code() {
        assert_eq!(ActivationType::Sms.code(), 0);
        assert_eq!(ActivationType::Number.code(), 1);
        assert_eq!(ActivationType::Voice.code(), 2);
    }

    #[test]
    fn test_provider_selection_as_str() {
        assert_eq!(ProviderSelection::Pr1.as_str(), "pr1");
        assert_eq!(ProviderSelection::Pr2.as_str(), "pr2");
        assert_eq!(ProviderSelection::Pr3.as_str(), "pr3");
        assert_eq!(
            ProviderSelection::Other("custom".to_string()).as_str(),
            "custom"
        );
    }
}
