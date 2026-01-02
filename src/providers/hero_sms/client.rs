//! Hero SMS HTTP client.

use super::countries::SmsCountryExt;
use super::errors::{HeroSmsError, Result};
use super::response::{HeroSmsResponse, HeroSmsTextResponse};
use super::services::Service;
use super::types::{ActivationStatus, GetPhoneNumberResponse, GetSmsResponse, SetStatusResponse};
use crate::types::TaskId;
use keshvar::Country;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use secrecy::{ExposeSecret, SecretString};
use std::collections::HashMap;
use url::Url;

#[cfg(feature = "tracing")]
use opentelemetry::trace::Status;
#[cfg(feature = "tracing")]
use tracing::Span;
#[cfg(feature = "tracing")]
use tracing_opentelemetry::OpenTelemetrySpanExt;

/// Default Hero SMS API URL.
pub const DEFAULT_API_URL: &str = "https://hero-sms.com/stubs/handler_api.php";

/// Hero SMS HTTP client.
///
/// This client handles communication with the Hero SMS API for phone number
/// verification services. The client is service-agnostic - you specify the service
/// when requesting a phone number.
///
/// # Example
///
/// ```rust,ignore
/// use sms_solvers::hero_sms::{HeroSms, Service};
/// use sms_solvers::Alpha2;
///
/// let client = HeroSms::with_api_key("your_api_key")?;
///
/// // Get a phone number for WhatsApp verification
/// let response = client.get_phone_number(Alpha2::US.to_country(), Service::Whatsapp).await?;
/// println!("Got number: {}", response.phone_number);
///
/// // Use the same client for Instagram
/// let response = client.get_phone_number(Alpha2::DE.to_country(), Service::InstagramThreads).await?;
/// ```
#[derive(Clone)]
pub struct HeroSms {
    http_client: ClientWithMiddleware,
    api_key: SecretString,
    endpoint: Url,
}

impl std::fmt::Debug for HeroSms {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HeroSmsClient")
            .field("endpoint", &self.endpoint)
            .field("api_key", &"[REDACTED]")
            .finish()
    }
}

/// Builder for configuring a [`HeroSms`].
pub struct HeroSmsClientBuilder {
    api_key: String,
    endpoint: Option<Url>,
    http_client: Option<ClientWithMiddleware>,
}

impl HeroSmsClientBuilder {
    /// Create a new builder with the given API key.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            endpoint: None,
            http_client: None,
        }
    }

    /// Set a custom API endpoint.
    pub fn endpoint(mut self, endpoint: Url) -> Self {
        self.endpoint = Some(endpoint);
        self
    }

    /// Set a custom HTTP client with middleware.
    pub fn http_client(mut self, client: ClientWithMiddleware) -> Self {
        self.http_client = Some(client);
        self
    }

    /// Build the [`HeroSms`].
    pub fn build(self) -> Result<HeroSms> {
        let endpoint = self
            .endpoint
            .unwrap_or_else(|| Url::parse(DEFAULT_API_URL).expect("Invalid default URL"));

        let http_client = match self.http_client {
            Some(client) => client,
            None => {
                let client = reqwest::Client::builder()
                    .build()
                    .map_err(HeroSmsError::BuildHttpClient)?;
                ClientBuilder::new(client).build()
            }
        };

        Ok(HeroSms {
            http_client,
            api_key: SecretString::from(self.api_key),
            endpoint,
        })
    }
}

impl HeroSms {
    /// Create a new Hero SMS client.
    ///
    /// # Arguments
    /// * `endpoint` - Base URL for the Hero SMS API
    /// * `api_key` - API key for authentication
    pub fn new(endpoint: impl AsRef<str>, api_key: impl Into<String>) -> Result<Self> {
        let url = Url::parse(endpoint.as_ref()).map_err(|e| {
            HeroSmsError::BuildRequestUrl(serde_urlencoded::ser::Error::Custom(
                std::borrow::Cow::Owned(e.to_string()),
            ))
        })?;

        Self::builder(api_key).endpoint(url).build()
    }

    /// Create a new client with the default API URL.
    pub fn with_api_key(api_key: impl Into<String>) -> Result<Self> {
        Self::builder(api_key).build()
    }

    /// Create a builder for configuring the client.
    pub fn builder(api_key: impl Into<String>) -> HeroSmsClientBuilder {
        HeroSmsClientBuilder::new(api_key)
    }

    /// Build request URL with action and parameters.
    fn build_request_url(&self, action: &str, additional: Vec<(&str, String)>) -> Result<Url> {
        let mut endpoint = self.endpoint.clone();
        let api_key = self.api_key.expose_secret().to_string();

        let mut params = HashMap::new();
        params.insert("api_key", api_key);
        params.insert("action", action.to_string());

        for (key, value) in additional {
            params.insert(key, value);
        }

        endpoint.set_query(Some(
            &serde_urlencoded::to_string(&params).map_err(HeroSmsError::BuildRequestUrl)?,
        ));

        Ok(endpoint)
    }

    /// Send a GET request and return the response text.
    async fn send_request(&self, url: Url) -> Result<String> {
        let response = self
            .http_client
            .get(url)
            .send()
            .await
            .map_err(HeroSmsError::HttpRequest)?;

        response.text().await.map_err(HeroSmsError::ParseResponse)
    }

    /// Get a phone number for verification.
    ///
    /// # Arguments
    /// * `country` - The country to get a phone number for
    /// * `service` - The service to use for verification (e.g., WhatsApp, Instagram)
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(
            name = "HeroSms::get_phone_number",
            skip_all,
            fields(service = %service.code(), country = %country.iso_short_name())
        )
    )]
    pub async fn get_phone_number(
        &self,
        country: Country,
        service: Service,
    ) -> Result<GetPhoneNumberResponse> {
        let country_id = country.sms_id().map_err(|_| HeroSmsError::CountryMapping {
            country: Box::new(country),
        })?;

        let url = self.build_request_url(
            "getNumberV2",
            vec![
                ("service", service.code().to_string()),
                ("country", country_id.to_string()),
            ],
        )?;

        let text = self.send_request(url).await?;

        let response = HeroSmsResponse::<GetPhoneNumberResponse>::from_text(&text)
            .map_err(HeroSmsError::DeserializeJson)?;

        let data = response.into_result().map_err(HeroSmsError::Service)?;

        #[cfg(feature = "tracing")]
        {
            Span::current()
                .record("task_id", data.task_id.as_ref())
                .record("phone_number", &data.phone_number)
                .set_status(Status::Ok);
        }

        Ok(data)
    }

    /// Get SMS code for an activation.
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(
            name = "HeroSms::get_sms_code",
            skip_all,
            fields(task_id = %task_id)
        )
    )]
    pub async fn get_sms_code(&self, task_id: &TaskId) -> Result<GetSmsResponse> {
        let url = self.build_request_url("getStatusV2", vec![("id", task_id.to_string())])?;

        let text = self.send_request(url).await?;

        let response = HeroSmsResponse::<GetSmsResponse>::from_text(&text)
            .map_err(HeroSmsError::DeserializeJson)?;

        let data = response.into_result().map_err(HeroSmsError::Service)?;

        #[cfg(feature = "tracing")]
        if let Some(sms) = &data.sms
            && !sms.code.is_empty()
        {
            Span::current()
                .record("sms_code", sms.code.as_str())
                .set_status(Status::Ok);
        }

        Ok(data)
    }

    /// Set activation status.
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(
            name = "HeroSms::set_activation_status",
            skip_all,
            fields(task_id = %task_id, status = %status)
        )
    )]
    pub async fn set_activation_status(
        &self,
        task_id: &TaskId,
        status: ActivationStatus,
    ) -> Result<SetStatusResponse> {
        let url = self.build_request_url(
            "setStatus",
            vec![
                ("id", task_id.to_string()),
                ("status", status.code().to_string()),
            ],
        )?;

        let text = self.send_request(url).await?;

        let response = HeroSmsTextResponse::from_text(&text);
        let raw = response.into_result().map_err(HeroSmsError::Service)?;

        let result = SetStatusResponse::from_raw(&raw)
            .ok_or_else(|| HeroSmsError::FailedToParseSetStatusResponse { raw: raw.clone() })?;

        #[cfg(feature = "tracing")]
        {
            Span::current()
                .record("response", result.to_string())
                .set_status(Status::Ok);
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::hero_sms::errors::HeroSmsErrorCode;
    use keshvar::Alpha2;
    use wiremock::matchers::{method, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn test_get_phone_number_success() {
        let mock_server = MockServer::start().await;

        let response_body = serde_json::json!({
            "activationId": "123456789",
            "phoneNumber": "380501234567",
            "activationCost": 10.5,
            "currency": 643,
            "countryCode": "380",
            "canGetAnotherSms": true,
            "activationTime": "2025-01-01 12:00:00",
            "activationEndTime": "2025-01-01 12:20:00",
            "activationOperator": "kyivstar"
        });

        Mock::given(method("GET"))
            .and(query_param("action", "getNumberV2"))
            .and(query_param("service", "ig"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
            .mount(&mock_server)
            .await;

        let client = HeroSms::new(mock_server.uri(), "test_key").unwrap();
        let result = client
            .get_phone_number(Alpha2::UA.to_country(), Service::InstagramThreads)
            .await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.task_id.as_ref(), "123456789");
        assert_eq!(response.phone_number, "380501234567");
    }

    #[tokio::test]
    async fn test_get_phone_number_no_numbers_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(query_param("action", "getNumberV2"))
            .and(query_param("service", "wa"))
            .respond_with(ResponseTemplate::new(200).set_body_string("NO_NUMBERS"))
            .mount(&mock_server)
            .await;

        let client = HeroSms::new(mock_server.uri(), "test_key").unwrap();
        let result = client
            .get_phone_number(Alpha2::UA.to_country(), Service::Whatsapp)
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            HeroSmsError::Service(error) => {
                assert_eq!(error.code, HeroSmsErrorCode::NoNumbers);
            }
            _ => panic!("Expected Service error"),
        }
    }

    #[tokio::test]
    async fn test_get_sms_code_success() {
        let mock_server = MockServer::start().await;

        let response_body = serde_json::json!({
            "sms": {
                "dateTime": "2025-01-01 12:05:00",
                "code": "123456",
                "text": "Your code is: 123456"
            }
        });

        Mock::given(method("GET"))
            .and(query_param("action", "getStatusV2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
            .mount(&mock_server)
            .await;

        let client = HeroSms::new(mock_server.uri(), "test_key").unwrap();
        let result = client.get_sms_code(&TaskId::from("123456789")).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.sms.is_some());
        assert_eq!(response.sms.unwrap().code, "123456");
    }

    #[tokio::test]
    async fn test_set_activation_status_cancel() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(query_param("action", "setStatus"))
            .and(query_param("status", "8"))
            .respond_with(ResponseTemplate::new(200).set_body_string("ACCESS_CANCEL"))
            .mount(&mock_server)
            .await;

        let client = HeroSms::new(mock_server.uri(), "test_key").unwrap();
        let result = client
            .set_activation_status(
                &TaskId::from("123456789"),
                ActivationStatus::CancelUsedNumber,
            )
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), SetStatusResponse::Cancel);
    }
}
