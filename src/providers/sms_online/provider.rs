//! SMS.online provider implementation.

use super::client::SmsOnline;
use super::countries::SMS_ONLINE_ID2COUNTRY;
use super::errors::{Result, SmsOnlineError};
use super::services::Service;
use super::types::{ActivationStatus, GetNumberOptions, GetSmsStatusResponse};
use crate::providers::traits::Provider;
use crate::types::{DialCode, FullNumber, SmsCode, TaskId};
use keshvar::Country;
use std::collections::HashSet;

#[cfg(feature = "tracing")]
use tracing::{debug, warn};

/// SMS.online provider implementation.
///
/// This wraps the [`SmsOnline`] and implements the generic [`Provider`] trait.
/// The service is passed at call time to `get_phone_number`, allowing a single provider
/// to be used for multiple services.
#[derive(Debug, Clone)]
pub struct SmsOnlineProvider {
    client: SmsOnline,
    blacklisted_dial_codes: HashSet<DialCode>,
    options: Option<GetNumberOptions>,
}

impl SmsOnlineProvider {
    /// Create a new SMS.online provider.
    ///
    /// # Arguments
    /// * `client` - The SMS.online client to use
    pub fn new(client: SmsOnline) -> Self {
        Self {
            client,
            blacklisted_dial_codes: HashSet::new(),
            options: None,
        }
    }

    /// Create a new SMS.online provider with a blacklist of dial codes.
    ///
    /// Numbers from blacklisted dial codes will not be used.
    pub fn with_blacklist(client: SmsOnline, blacklist: HashSet<DialCode>) -> Self {
        Self {
            client,
            blacklisted_dial_codes: blacklist,
            options: None,
        }
    }

    /// Set optional request parameters for number acquisition.
    pub fn with_options(mut self, options: GetNumberOptions) -> Self {
        self.options = Some(options);
        self
    }

    /// Add a dial code to the blacklist.
    pub fn blacklist_dial_code(&mut self, dial_code: DialCode) {
        self.blacklisted_dial_codes.insert(dial_code);
    }

    /// Remove a dial code from the blacklist.
    pub fn remove_from_blacklist(&mut self, dial_code: &DialCode) -> bool {
        self.blacklisted_dial_codes.remove(dial_code)
    }

    /// Get reference to the inner client.
    pub fn client(&self) -> &SmsOnline {
        &self.client
    }

    /// Get the blacklisted dial codes.
    pub fn blacklisted_dial_codes(&self) -> &HashSet<DialCode> {
        &self.blacklisted_dial_codes
    }
}

impl Provider for SmsOnlineProvider {
    type Error = SmsOnlineError;
    type Service = Service;

    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(
            name = "SmsOnlineProvider::get_phone_number",
            skip_all,
            fields(service = %service.code(), country = %country.iso_short_name())
        )
    )]
    async fn get_phone_number(
        &self,
        country: Country,
        service: Self::Service,
    ) -> Result<(TaskId, FullNumber, Option<DialCode>)> {
        // SMS.online API doesn't return dial code separately, derive from requested country
        let dial_code = DialCode::from(&country);

        let response = self
            .client
            .get_phone_number(country, service, self.options.as_ref())
            .await?;

        #[cfg(feature = "tracing")]
        if !response.phone_number.starts_with(dial_code.as_str()) {
            warn!(
                expected_prefix = dial_code.as_str(),
                phone_number = "[REDACTED]",
                "Phone number doesn't start with expected dial code"
            );
        }

        Ok((
            response.task_id,
            FullNumber::from(response.phone_number),
            Some(dial_code),
        ))
    }

    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(
            name = "SmsOnlineProvider::get_sms_code",
            skip_all,
            fields(task_id = %task_id)
        )
    )]
    async fn get_sms_code(&self, task_id: &TaskId) -> Result<Option<SmsCode>> {
        let response = self.client.get_sms_code(task_id).await?;

        match response {
            GetSmsStatusResponse::WaitCode | GetSmsStatusResponse::WaitResend => Ok(None),
            GetSmsStatusResponse::Ok { code } => Ok(Some(SmsCode::new(&code))),
            GetSmsStatusResponse::Cancel => Err(SmsOnlineError::ActivationCanceled {
                task_id: task_id.clone(),
            }),
        }
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
        !self.blacklisted_dial_codes.contains(dial_code)
    }
}

impl crate::providers::capabilities::ProviderCapabilities for SmsOnlineProvider {
    type Service = Service;

    fn supports_service(&self, _service: &Self::Service) -> Option<bool> {
        // SMS.online doesn't provide a reliable way to check per-service support.
        None
    }

    fn available_countries(&self, _service: &Self::Service) -> Option<Vec<Country>> {
        // Static mapping of countries with SMS.online IDs — not per-service.
        Some(SMS_ONLINE_ID2COUNTRY.values().cloned().collect())
    }

    fn supported_services(&self) -> Option<Vec<Self::Service>> {
        Some(Service::all())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::capabilities::ProviderCapabilities;
    use keshvar::Alpha2;
    use wiremock::matchers::{method, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn create_test_provider(mock_server: &MockServer) -> SmsOnlineProvider {
        let client = SmsOnline::new(mock_server.uri(), "test_key").unwrap();
        SmsOnlineProvider::new(client)
    }

    #[tokio::test]
    async fn test_get_phone_number() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(query_param("action", "getNumber"))
            .and(query_param("service", "ig"))
            .respond_with(
                ResponseTemplate::new(200).set_body_string("ACCESS_NUMBER:123456:380501234567"),
            )
            .mount(&mock_server)
            .await;

        let provider = create_test_provider(&mock_server);
        let result = provider
            .get_phone_number(Alpha2::UA.to_country(), Service::InstagramThreads)
            .await;

        assert!(result.is_ok());
        let (task_id, full_number, dial_code) = result.unwrap();
        assert_eq!(task_id.as_ref(), "123456");
        assert_eq!(full_number.as_ref(), "380501234567");
        assert_eq!(dial_code.unwrap().as_str(), "380");
    }

    #[tokio::test]
    async fn test_get_sms_code_received() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(query_param("action", "getStatus"))
            .respond_with(ResponseTemplate::new(200).set_body_string("STATUS_OK:123456"))
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
            .and(query_param("action", "getStatus"))
            .respond_with(ResponseTemplate::new(200).set_body_string("STATUS_WAIT_CODE"))
            .mount(&mock_server)
            .await;

        let provider = create_test_provider(&mock_server);
        let result = provider.get_sms_code(&TaskId::from("123")).await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_get_sms_code_cancelled() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(query_param("action", "getStatus"))
            .respond_with(ResponseTemplate::new(200).set_body_string("STATUS_CANCEL"))
            .mount(&mock_server)
            .await;

        let provider = create_test_provider(&mock_server);
        let task_id = TaskId::from("123");
        let result = provider.get_sms_code(&task_id).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            SmsOnlineError::ActivationCanceled {
                task_id: err_task_id,
            } => {
                assert_eq!(err_task_id, task_id);
            }
            other => panic!("Expected ActivationCanceled, got {other:?}"),
        }
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
        let client = SmsOnline::with_api_key("test_key").unwrap();
        let mut provider = SmsOnlineProvider::new(client);

        let dial_code = DialCode::new("33").unwrap();
        assert!(provider.is_dial_code_supported(&dial_code));

        provider.blacklist_dial_code(dial_code.clone());
        assert!(!provider.is_dial_code_supported(&dial_code));

        provider.remove_from_blacklist(&dial_code);
        assert!(provider.is_dial_code_supported(&dial_code));
    }

    #[test]
    fn test_supports_service_returns_none() {
        let client = SmsOnline::with_api_key("test_key").unwrap();
        let provider = SmsOnlineProvider::new(client);

        // SMS.online cannot reliably check per-service support
        assert_eq!(provider.supports_service(&Service::Whatsapp), None);
        assert_eq!(provider.supports_service(&Service::InstagramThreads), None);
    }

    #[test]
    fn test_available_countries() {
        let client = SmsOnline::with_api_key("test_key").unwrap();
        let provider = SmsOnlineProvider::new(client);

        let countries = provider.available_countries(&Service::Whatsapp).unwrap();
        assert!(!countries.is_empty());
        assert!(countries.iter().any(|c| c.alpha2() == Alpha2::US));
        assert!(countries.iter().any(|c| c.alpha2() == Alpha2::UA));
    }

    #[test]
    fn test_supported_services() {
        let client = SmsOnline::with_api_key("test_key").unwrap();
        let provider = SmsOnlineProvider::new(client);

        let services = provider.supported_services().unwrap();
        assert!(!services.is_empty());
        assert!(services.contains(&Service::Whatsapp));
        assert!(services.contains(&Service::InstagramThreads));
        assert!(services.contains(&Service::Facebook));
    }
}
