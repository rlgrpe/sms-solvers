//! Response parsing for SMS.online API.

use super::errors::{SmsOnlineServiceError, parse_sms_online_error};

/// Response type for sms.online methods returning plain text.
#[derive(Debug)]
pub enum SmsOnlineTextResponse {
    Success(String),
    Error(SmsOnlineServiceError),
}

impl SmsOnlineTextResponse {
    pub fn from_text(text: &str) -> Self {
        if let Some(error) = parse_sms_online_error(text) {
            Self::Error(error)
        } else {
            Self::Success(text.to_string())
        }
    }

    pub fn into_result(self) -> Result<String, SmsOnlineServiceError> {
        match self {
            Self::Success(text) => Ok(text),
            Self::Error(error) => Err(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::sms_online::errors::SmsOnlineErrorCode;

    #[test]
    fn test_text_response_success() {
        let response = SmsOnlineTextResponse::from_text("ACCESS_CANCEL");
        match response {
            SmsOnlineTextResponse::Success(s) => assert_eq!(s, "ACCESS_CANCEL"),
            SmsOnlineTextResponse::Error(_) => panic!("Expected success"),
        }
    }

    #[test]
    fn test_text_response_error() {
        let response = SmsOnlineTextResponse::from_text("BAD_KEY");
        match response {
            SmsOnlineTextResponse::Success(_) => panic!("Expected error"),
            SmsOnlineTextResponse::Error(e) => assert_eq!(e.code, SmsOnlineErrorCode::BadKey),
        }
    }
}
