//! Provider contract tests.
//!
//! Verifies that all Provider implementations satisfy the core contract:
//! - get_phone_number returns valid TaskId and FullNumber
//! - get_sms_code returns Ok(None) when waiting, Ok(Some) when code arrives
//! - cancel_activation succeeds
//! - error classification (retryable vs permanent) is consistent
//!
//! These tests use wiremock to avoid real API calls.

use sms_solvers::{Provider, RetryableError, TaskId};
use wiremock::matchers::{method, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ============================================================================
// Hero SMS provider contract
// ============================================================================

#[cfg(feature = "hero-sms")]
mod hero_sms_contract {
    use super::*;
    use keshvar::Alpha2;
    use sms_solvers::hero_sms::{HeroSms, HeroSmsProvider, Service};

    fn setup_provider(mock_server: &MockServer) -> HeroSmsProvider {
        let client = HeroSms::new(mock_server.uri(), "test_key").unwrap();
        HeroSmsProvider::new(client)
    }

    #[tokio::test]
    async fn contract_get_number_returns_valid_task_id_and_number() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(query_param("action", "getNumberV2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "activationId": "42",
                "phoneNumber": "380501234567",
                "activationCost": 5.0,
                "currency": 643,
                "countryCode": "380",
                "canGetAnotherSms": true,
                "activationTime": "2025-01-01 12:00:00",
                "activationEndTime": "2025-01-01 12:20:00",
                "activationOperator": "kyivstar",
                "countryPhoneCode": 380
            })))
            .mount(&mock_server)
            .await;

        let provider = setup_provider(&mock_server);
        let (task_id, full_number, _dial_code) = provider
            .get_phone_number(Alpha2::UA.to_country(), Service::Whatsapp)
            .await
            .expect("get_phone_number should succeed");

        assert!(!task_id.as_ref().is_empty(), "TaskId must be non-empty");
        assert!(
            !full_number.as_ref().is_empty(),
            "FullNumber must be non-empty"
        );
    }

    #[tokio::test]
    async fn contract_get_sms_code_none_means_continue_polling() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(query_param("action", "getStatusV2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
            .mount(&mock_server)
            .await;

        let provider = setup_provider(&mock_server);
        let result = provider
            .get_sms_code(&TaskId::from("42"))
            .await
            .expect("get_sms_code should succeed");

        assert!(result.is_none(), "Ok(None) means SMS not yet received");
    }

    #[tokio::test]
    async fn contract_get_sms_code_some_returns_code() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(query_param("action", "getStatusV2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "sms": { "dateTime": "2025-01-01", "code": "999888", "text": "code: 999888" }
            })))
            .mount(&mock_server)
            .await;

        let provider = setup_provider(&mock_server);
        let result = provider
            .get_sms_code(&TaskId::from("42"))
            .await
            .expect("get_sms_code should succeed");

        let code = result.expect("Should have received SMS code");
        assert!(!code.as_str().is_empty(), "SmsCode must be non-empty");
    }

    #[tokio::test]
    async fn contract_cancel_activation_succeeds() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(query_param("action", "setStatus"))
            .and(query_param("status", "8"))
            .respond_with(ResponseTemplate::new(200).set_body_string("ACCESS_CANCEL"))
            .mount(&mock_server)
            .await;

        let provider = setup_provider(&mock_server);
        provider
            .cancel_activation(&TaskId::from("42"))
            .await
            .expect("cancel_activation should succeed");
    }

    #[tokio::test]
    async fn contract_no_numbers_error_is_retryable() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(query_param("action", "getNumberV2"))
            .respond_with(ResponseTemplate::new(200).set_body_string("NO_NUMBERS"))
            .mount(&mock_server)
            .await;

        let provider = setup_provider(&mock_server);
        let err = provider
            .get_phone_number(Alpha2::UA.to_country(), Service::Whatsapp)
            .await
            .unwrap_err();

        assert!(
            err.is_retryable(),
            "NO_NUMBERS should be retryable, got: {err}"
        );
    }

    #[tokio::test]
    async fn contract_bad_key_error_is_not_retryable() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(query_param("action", "getNumberV2"))
            .respond_with(ResponseTemplate::new(200).set_body_string("BAD_KEY"))
            .mount(&mock_server)
            .await;

        let provider = setup_provider(&mock_server);
        let err = provider
            .get_phone_number(Alpha2::UA.to_country(), Service::Whatsapp)
            .await
            .unwrap_err();

        assert!(
            !err.is_retryable(),
            "BAD_KEY should not be retryable, got: {err}"
        );
        assert!(
            !err.should_retry_operation(),
            "BAD_KEY should not retry operation"
        );
    }
}

// ============================================================================
// SMS.online provider contract
// ============================================================================

#[cfg(feature = "sms-online")]
mod sms_online_contract {
    use super::*;
    use keshvar::Alpha2;
    use sms_solvers::sms_online::{Service, SmsOnline, SmsOnlineProvider};

    fn setup_provider(mock_server: &MockServer) -> SmsOnlineProvider {
        let client = SmsOnline::new(mock_server.uri(), "test_key").unwrap();
        SmsOnlineProvider::new(client)
    }

    #[tokio::test]
    async fn contract_get_number_returns_valid_task_id_and_number() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(query_param("action", "getNumber"))
            .respond_with(
                ResponseTemplate::new(200).set_body_string("ACCESS_NUMBER:42:380501234567"),
            )
            .mount(&mock_server)
            .await;

        let provider = setup_provider(&mock_server);
        let (task_id, full_number, _dial_code) = provider
            .get_phone_number(Alpha2::UA.to_country(), Service::Whatsapp)
            .await
            .expect("get_phone_number should succeed");

        assert!(!task_id.as_ref().is_empty(), "TaskId must be non-empty");
        assert!(
            !full_number.as_ref().is_empty(),
            "FullNumber must be non-empty"
        );
    }

    #[tokio::test]
    async fn contract_get_sms_code_none_means_continue_polling() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(query_param("action", "getStatus"))
            .respond_with(ResponseTemplate::new(200).set_body_string("STATUS_WAIT_CODE"))
            .mount(&mock_server)
            .await;

        let provider = setup_provider(&mock_server);
        let result = provider
            .get_sms_code(&TaskId::from("42"))
            .await
            .expect("get_sms_code should succeed");

        assert!(result.is_none(), "Ok(None) means SMS not yet received");
    }

    #[tokio::test]
    async fn contract_get_sms_code_some_returns_code() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(query_param("action", "getStatus"))
            .respond_with(ResponseTemplate::new(200).set_body_string("STATUS_OK:999888"))
            .mount(&mock_server)
            .await;

        let provider = setup_provider(&mock_server);
        let result = provider
            .get_sms_code(&TaskId::from("42"))
            .await
            .expect("get_sms_code should succeed");

        let code = result.expect("Should have received SMS code");
        assert!(!code.as_str().is_empty(), "SmsCode must be non-empty");
    }

    #[tokio::test]
    async fn contract_cancel_activation_succeeds() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(query_param("action", "setStatus"))
            .and(query_param("status", "8"))
            .respond_with(ResponseTemplate::new(200).set_body_string("ACCESS_CANCEL"))
            .mount(&mock_server)
            .await;

        let provider = setup_provider(&mock_server);
        provider
            .cancel_activation(&TaskId::from("42"))
            .await
            .expect("cancel_activation should succeed");
    }

    #[tokio::test]
    async fn contract_no_numbers_error_is_retryable() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(query_param("action", "getNumber"))
            .respond_with(ResponseTemplate::new(200).set_body_string("NO_NUMBERS"))
            .mount(&mock_server)
            .await;

        let provider = setup_provider(&mock_server);
        let err = provider
            .get_phone_number(Alpha2::UA.to_country(), Service::Whatsapp)
            .await
            .unwrap_err();

        assert!(
            err.is_retryable(),
            "NO_NUMBERS should be retryable, got: {err}"
        );
    }

    #[tokio::test]
    async fn contract_bad_key_error_is_not_retryable() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(query_param("action", "getNumber"))
            .respond_with(ResponseTemplate::new(200).set_body_string("BAD_KEY"))
            .mount(&mock_server)
            .await;

        let provider = setup_provider(&mock_server);
        let err = provider
            .get_phone_number(Alpha2::UA.to_country(), Service::Whatsapp)
            .await
            .unwrap_err();

        assert!(
            !err.is_retryable(),
            "BAD_KEY should not be retryable, got: {err}"
        );
        assert!(
            !err.should_retry_operation(),
            "BAD_KEY should not retry operation"
        );
    }

    #[tokio::test]
    async fn contract_activation_cancel_is_terminal_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(query_param("action", "getStatus"))
            .respond_with(ResponseTemplate::new(200).set_body_string("STATUS_CANCEL"))
            .mount(&mock_server)
            .await;

        let provider = setup_provider(&mock_server);
        let err = provider
            .get_sms_code(&TaskId::from("42"))
            .await
            .unwrap_err();

        assert!(!err.is_retryable(), "STATUS_CANCEL should not be retryable");
    }
}
