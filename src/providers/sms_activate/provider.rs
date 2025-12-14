//! SMS Activate provider implementation.

use super::client::SmsActivateClient;
use super::countries::CC2SMS_ID;
use super::errors::{Result, SmsActivateError};
use super::services::Service;
use super::types::ActivationStatus;
use crate::providers::traits::Provider;
use crate::types::{DialCode, FullNumber, SmsCode, TaskId};
use isocountry::CountryCode;
use std::collections::HashSet;

#[cfg(feature = "tracing")]
use tracing::debug;

/// SMS Activate provider implementation.
///
/// This wraps the [`SmsActivateClient`] and implements the generic [`Provider`] trait.
/// The service is passed at call time to `get_phone_number`, allowing a single provider
/// to be used for multiple services.
///
/// # Example
///
/// ```rust,ignore
/// use sms_solvers::providers::sms_activate::{SmsActivateClient, SmsActivateProvider, Service};
/// use sms_solvers::{SmsService, SmsServiceConfig, RetryableProvider};
///
/// // Create client and provider
/// let client = SmsActivateClient::with_api_key("your_api_key")?;
/// let provider = SmsActivateProvider::new(client);
///
/// // Optionally wrap with retry logic
/// let retryable = RetryableProvider::new(provider);
///
/// // Create service
/// let service = SmsService::with_provider(retryable);
///
/// // Get phone number for WhatsApp
/// let (task_id, number) = provider.get_phone_number(CountryCode::USA, Service::Whatsapp).await?;
///
/// // Use the same provider for Instagram
/// let (task_id2, number2) = provider.get_phone_number(CountryCode::DEU, Service::InstagramThreads).await?;
/// ```
#[derive(Debug, Clone)]
pub struct SmsActivateProvider {
    client: SmsActivateClient,
    blacklisted_dial_codes: HashSet<String>,
}

impl SmsActivateProvider {
    /// Create a new SMS Activate provider.
    ///
    /// # Arguments
    /// * `client` - The SMS Activate client to use
    pub fn new(client: SmsActivateClient) -> Self {
        Self {
            client,
            blacklisted_dial_codes: HashSet::new(),
        }
    }

    /// Create a new SMS Activate provider with a blacklist of dial codes.
    ///
    /// Numbers from blacklisted dial codes will not be used.
    pub fn with_blacklist(client: SmsActivateClient, blacklist: HashSet<String>) -> Self {
        Self {
            client,
            blacklisted_dial_codes: blacklist,
        }
    }

    /// Add a dial code to the blacklist.
    pub fn blacklist_dial_code(&mut self, dial_code: impl Into<String>) {
        self.blacklisted_dial_codes.insert(dial_code.into());
    }

    /// Remove a dial code from the blacklist.
    pub fn remove_from_blacklist(&mut self, dial_code: &str) -> bool {
        self.blacklisted_dial_codes.remove(dial_code)
    }

    /// Get reference to the inner client.
    pub fn client(&self) -> &SmsActivateClient {
        &self.client
    }

    /// Get the blacklisted dial codes.
    pub fn blacklisted_dial_codes(&self) -> &HashSet<String> {
        &self.blacklisted_dial_codes
    }
}

impl Provider for SmsActivateProvider {
    type Error = SmsActivateError;
    type Service = Service;

    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(
            name = "SmsActivateProvider::get_phone_number",
            skip_all,
            fields(service = %service.code(), country = %country.alpha2())
        )
    )]
    async fn get_phone_number(
        &self,
        country: CountryCode,
        service: Self::Service,
    ) -> Result<(TaskId, FullNumber)> {
        let response = self.client.get_phone_number(country, service).await?;

        Ok((response.task_id, FullNumber::from(response.phone_number)))
    }

    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(
            name = "SmsActivateProvider::get_sms_code",
            skip_all,
            fields(task_id = %task_id)
        )
    )]
    async fn get_sms_code(&self, task_id: &TaskId) -> Result<Option<SmsCode>> {
        let response = self.client.get_sms_code(task_id).await?;

        if let Some(sms) = &response.sms
            && !sms.code.is_empty()
        {
            return Ok(Some(SmsCode::new(&sms.code)));
        }

        Ok(None)
    }

    async fn finish_activation(&self, task_id: &TaskId) -> Result<()> {
        self.client
            .set_activation_status(task_id, ActivationStatus::FinishActivation)
            .await?;

        #[cfg(feature = "tracing")]
        debug!(task_id = %task_id, "Activation finished successfully");

        Ok(())
    }

    async fn cancel_activation(&self, task_id: &TaskId) -> Result<()> {
        self.client
            .set_activation_status(task_id, ActivationStatus::CancelUsedNumber)
            .await?;

        #[cfg(feature = "tracing")]
        debug!(task_id = %task_id, "Activation cancelled");

        Ok(())
    }

    fn is_dial_code_supported(&self, dial_code: &DialCode) -> bool {
        !self.blacklisted_dial_codes.contains(dial_code.as_str())
    }

    fn supports_service(&self, _service: &Self::Service) -> bool {
        // SMS Activate supports all services including custom ones
        true
    }

    fn available_countries(&self, _service: &Self::Service) -> Vec<CountryCode> {
        // Return all countries that have SMS Activate mapping
        CC2SMS_ID.keys().copied().collect()
    }

    fn supported_services(&self) -> Vec<Self::Service> {
        Service::all()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn create_test_provider(mock_server: &MockServer) -> SmsActivateProvider {
        let client = SmsActivateClient::new(mock_server.uri(), "test_key").unwrap();
        SmsActivateProvider::new(client)
    }

    #[tokio::test]
    async fn test_get_phone_number() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(query_param("action", "getNumberV2"))
            .and(query_param("service", "ig"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "activationId": "123456",
                "phoneNumber": "380501234567",
                "activationCost": 10.5,
                "currency": 643,
                "countryCode": "380",
                "canGetAnotherSms": true,
                "activationTime": "2025-01-01 12:00:00",
                "activationEndTime": "2025-01-01 12:20:00",
                "activationOperator": "kyivstar"
            })))
            .mount(&mock_server)
            .await;

        let provider = create_test_provider(&mock_server);
        let result = provider
            .get_phone_number(CountryCode::UKR, Service::InstagramThreads)
            .await;

        assert!(result.is_ok());
        let (task_id, full_number) = result.unwrap();
        assert_eq!(task_id.as_ref(), "123456");
        assert_eq!(full_number.as_ref(), "380501234567");
    }

    #[tokio::test]
    async fn test_get_sms_code_received() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(query_param("action", "getStatusV2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "sms": {
                    "dateTime": "2025-01-01 12:05:00",
                    "code": "123456",
                    "text": "Your code is: 123456"
                }
            })))
            .mount(&mock_server)
            .await;

        let provider = create_test_provider(&mock_server);
        let result = provider.get_sms_code(&TaskId::from("123")).await;

        assert!(result.is_ok());
        let code = result.unwrap();
        assert!(code.is_some());
        assert_eq!(code.unwrap().as_str(), "123456");
    }

    #[tokio::test]
    async fn test_get_sms_code_not_yet_received() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(query_param("action", "getStatusV2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
            .mount(&mock_server)
            .await;

        let provider = create_test_provider(&mock_server);
        let result = provider.get_sms_code(&TaskId::from("123")).await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_cancel_activation() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(query_param("action", "setStatus"))
            .and(query_param("status", "8"))
            .respond_with(ResponseTemplate::new(200).set_body_string("ACCESS_CANCEL"))
            .mount(&mock_server)
            .await;

        let provider = create_test_provider(&mock_server);
        let result = provider.cancel_activation(&TaskId::from("123")).await;

        assert!(result.is_ok());
    }

    #[test]
    fn test_dial_code_blacklist() {
        let client = SmsActivateClient::with_api_key("test_key").unwrap();
        let mut provider = SmsActivateProvider::new(client);

        let dial_code = DialCode::new("33").unwrap();
        assert!(provider.is_dial_code_supported(&dial_code));

        provider.blacklist_dial_code("33");
        assert!(!provider.is_dial_code_supported(&dial_code));

        provider.remove_from_blacklist("33");
        assert!(provider.is_dial_code_supported(&dial_code));
    }

    #[test]
    fn test_supports_service() {
        let client = SmsActivateClient::with_api_key("test_key").unwrap();
        let provider = SmsActivateProvider::new(client);

        assert!(provider.supports_service(&Service::Whatsapp));
        assert!(provider.supports_service(&Service::InstagramThreads));
        assert!(provider.supports_service(&Service::Other {
            code: "custom".to_string()
        }));
    }

    #[test]
    fn test_available_countries() {
        let client = SmsActivateClient::with_api_key("test_key").unwrap();
        let provider = SmsActivateProvider::new(client);

        let countries = provider.available_countries(&Service::Whatsapp);
        assert!(!countries.is_empty());
        assert!(countries.contains(&CountryCode::USA));
        assert!(countries.contains(&CountryCode::UKR));
    }

    #[test]
    fn test_supported_services() {
        let client = SmsActivateClient::with_api_key("test_key").unwrap();
        let provider = SmsActivateProvider::new(client);

        let services = provider.supported_services();
        assert!(!services.is_empty());
        assert!(services.contains(&Service::Whatsapp));
        assert!(services.contains(&Service::InstagramThreads));
        assert!(services.contains(&Service::Facebook));
    }
}
