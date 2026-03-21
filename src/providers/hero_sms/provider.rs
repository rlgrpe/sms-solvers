//! Hero SMS provider implementation.

use super::client::HeroSms;
use super::countries::SMS_ID2COUNTRY;
use super::errors::{HeroSmsError, Result};
use super::services::Service;
use super::types::{ActivationStatus, GetNumberOptions};
use crate::providers::traits::Provider;
use crate::types::{DialCode, FullNumber, SmsCode, TaskId};
use keshvar::Country;
use std::collections::HashSet;

#[cfg(feature = "tracing")]
use crate::utils::span_status::{set_span_error, set_span_ok};
#[cfg(feature = "tracing")]
use tracing::warn;

/// Hero SMS provider implementation.
///
/// This wraps the [`HeroSms`] and implements the generic [`Provider`] trait.
/// The service is passed at call time to `get_phone_number`, allowing a single provider
/// to be used for multiple services.
///
/// # Example
///
/// ```rust,ignore
/// use sms_solvers::hero_sms::{HeroSms, HeroSmsProvider, Service};
/// use sms_solvers::{SmsSolverService, SmsRetryableProvider, Alpha2};
///
/// // Create client and provider
/// let client = HeroSms::with_api_key("your_api_key")?;
/// let provider = HeroSmsProvider::new(client);
///
/// // Optionally wrap with retry logic
/// let retryable = SmsRetryableProvider::new(provider);
///
/// // Create service
/// let service = SmsSolverService::with_provider(retryable);
///
/// // Get phone number for WhatsApp
/// let (task_id, number, _dial_code) = provider.get_phone_number(Alpha2::US.to_country(), Service::Whatsapp).await?;
///
/// // Use the same provider for Instagram
/// let (task_id2, number2, _dial_code2) = provider.get_phone_number(Alpha2::DE.to_country(), Service::InstagramThreads).await?;
/// ```
#[derive(Debug, Clone)]
pub struct HeroSmsProvider {
    client: HeroSms,
    blacklisted_dial_codes: HashSet<DialCode>,
    options: Option<GetNumberOptions>,
}

impl HeroSmsProvider {
    /// Create a new Hero SMS provider.
    ///
    /// # Arguments
    /// * `client` - The Hero SMS client to use
    pub fn new(client: HeroSms) -> Self {
        Self {
            client,
            blacklisted_dial_codes: HashSet::new(),
            options: None,
        }
    }

    /// Create a new Hero SMS provider with a blacklist of dial codes.
    ///
    /// Numbers from blacklisted dial codes will not be used.
    pub fn with_blacklist(client: HeroSms, blacklist: HashSet<DialCode>) -> Self {
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
    pub fn client(&self) -> &HeroSms {
        &self.client
    }

    /// Get the blacklisted dial codes.
    pub fn blacklisted_dial_codes(&self) -> &HashSet<DialCode> {
        &self.blacklisted_dial_codes
    }
}

impl Provider for HeroSmsProvider {
    type Error = HeroSmsError;
    type Service = Service;

    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(
            name = "get_phone_number",
            target = "sms.hero",
            skip_all,
            fields(service = %service.code(), country = %country.iso_short_name())
        )
    )]
    async fn get_phone_number(
        &self,
        country: Country,
        service: Self::Service,
    ) -> Result<(TaskId, FullNumber, Option<DialCode>)> {
        let response = self
            .client
            .get_phone_number(country, service, self.options.as_ref())
            .await
            .inspect_err(|e| {
                #[cfg(feature = "tracing")]
                set_span_error(e);
            })?;

        let api_dial_code = if response.country_phone_code > 0 {
            match DialCode::new(response.country_phone_code.to_string()) {
                Ok(dc) => Some(dc),
                Err(_e) => {
                    #[cfg(feature = "tracing")]
                    warn!(
                        country_phone_code = %response.country_phone_code,
                        error = %_e,
                        "Failed to parse API dial code, will derive from country"
                    );
                    None
                }
            }
        } else {
            None
        };

        #[cfg(feature = "tracing")]
        set_span_ok();

        Ok((
            response.task_id,
            FullNumber::from(response.phone_number),
            api_dial_code,
        ))
    }

    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(
            name = "get_sms_code",
            target = "sms.hero",
            skip_all,
            fields(task_id = %task_id)
        )
    )]
    async fn get_sms_code(&self, task_id: &TaskId) -> Result<Option<SmsCode>> {
        let response = self.client.get_sms_code(task_id).await.inspect_err(|e| {
            #[cfg(feature = "tracing")]
            set_span_error(e);
        })?;

        if let Some(sms) = &response.sms
            && !sms.code.is_empty()
        {
            #[cfg(feature = "tracing")]
            set_span_ok();
            return Ok(Some(SmsCode::new(&sms.code)));
        }

        Ok(None)
    }

    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(
            name = "finish_activation",
            target = "sms.hero",
            skip_all,
            fields(task_id = %task_id)
        )
    )]
    async fn finish_activation(&self, task_id: &TaskId) -> Result<()> {
        self.client
            .set_activation_status(task_id, ActivationStatus::FinishActivation)
            .await
            .map(|_| ())
            .inspect(|_| {
                #[cfg(feature = "tracing")]
                set_span_ok();
            })
            .inspect_err(|e| {
                #[cfg(feature = "tracing")]
                set_span_error(e);
            })
    }

    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(
            name = "cancel_activation",
            target = "sms.hero",
            skip_all,
            fields(task_id = %task_id)
        )
    )]
    async fn cancel_activation(&self, task_id: &TaskId) -> Result<()> {
        self.client
            .set_activation_status(task_id, ActivationStatus::CancelUsedNumber)
            .await
            .map(|_| ())
            .inspect(|_| {
                #[cfg(feature = "tracing")]
                set_span_ok();
            })
            .inspect_err(|e| {
                #[cfg(feature = "tracing")]
                set_span_error(e);
            })
    }

    fn is_dial_code_supported(&self, dial_code: &DialCode) -> bool {
        !self.blacklisted_dial_codes.contains(dial_code)
    }
}

impl crate::providers::capabilities::ProviderCapabilities for HeroSmsProvider {
    type Service = Service;

    fn supports_service(&self, _service: &Self::Service) -> Option<bool> {
        // Hero SMS doesn't provide a reliable way to check per-service support.
        None
    }

    fn available_countries(&self, _service: &Self::Service) -> Option<Vec<Country>> {
        // Static mapping of countries with Hero SMS IDs — not per-service.
        Some(SMS_ID2COUNTRY.values().cloned().collect())
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

    fn create_test_provider(mock_server: &MockServer) -> HeroSmsProvider {
        let client = HeroSms::new(mock_server.uri(), "test_key").unwrap();
        HeroSmsProvider::new(client)
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
                "countryCode": 380,
                "canGetAnotherSms": true,
                "activationTime": "2025-01-01 12:00:00",
                "activationEndTime": "2025-01-01 12:20:00",
                "activationOperator": "kyivstar",
                "countryPhoneCode": 380
            })))
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
        let client = HeroSms::with_api_key("test_key").unwrap();
        let mut provider = HeroSmsProvider::new(client);

        let dial_code = DialCode::new("33").unwrap();
        assert!(provider.is_dial_code_supported(&dial_code));

        provider.blacklist_dial_code(dial_code.clone());
        assert!(!provider.is_dial_code_supported(&dial_code));

        provider.remove_from_blacklist(&dial_code);
        assert!(provider.is_dial_code_supported(&dial_code));
    }

    #[test]
    fn test_supports_service_returns_none() {
        let client = HeroSms::with_api_key("test_key").unwrap();
        let provider = HeroSmsProvider::new(client);

        // Hero SMS cannot reliably check per-service support
        assert_eq!(provider.supports_service(&Service::Whatsapp), None);
        assert_eq!(provider.supports_service(&Service::InstagramThreads), None);
    }

    #[test]
    fn test_available_countries() {
        let client = HeroSms::with_api_key("test_key").unwrap();
        let provider = HeroSmsProvider::new(client);

        let countries = provider.available_countries(&Service::Whatsapp).unwrap();
        assert!(!countries.is_empty());
        assert!(countries.iter().any(|c| c.alpha2() == Alpha2::US));
        assert!(countries.iter().any(|c| c.alpha2() == Alpha2::UA));
    }

    #[test]
    fn test_supported_services() {
        let client = HeroSms::with_api_key("test_key").unwrap();
        let provider = HeroSmsProvider::new(client);

        let services = provider.supported_services().unwrap();
        assert!(!services.is_empty());
        assert!(services.contains(&Service::Whatsapp));
        assert!(services.contains(&Service::InstagramThreads));
        assert!(services.contains(&Service::Facebook));
    }
}
