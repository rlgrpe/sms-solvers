//! SMS.online HTTP client.

use super::countries::SmsOnlineCountryExt;
use super::errors::{Result, SmsOnlineError};
use super::response::SmsOnlineTextResponse;
use super::services::Service;
use super::types::{
    ActivationStatus, GetNumberOptions, GetPhoneNumberResponse, GetSmsStatusResponse,
    SetStatusResponse,
};
use crate::types::TaskId;
use keshvar::Country;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use secrecy::{ExposeSecret, SecretString};
use url::Url;

#[cfg(feature = "tracing")]
use crate::utils::span_status::{set_span_error, set_span_ok};
#[cfg(feature = "tracing")]
use tracing::Span;
#[cfg(feature = "tracing")]
use tracing_opentelemetry::OpenTelemetrySpanExt;

/// Default SMS.online API URL.
pub const DEFAULT_API_URL: &str = "https://api.sms.online/stubs/handler_api.php";

#[derive(Clone)]
pub struct SmsOnline {
    http_client: ClientWithMiddleware,
    api_key: SecretString,
    endpoint: Url,
    ref_id: Option<String>,
}

impl std::fmt::Debug for SmsOnline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SmsOnlineClient")
            .field("endpoint", &self.endpoint)
            .field("api_key", &crate::utils::REDACTED)
            .finish()
    }
}

pub struct SmsOnlineClientBuilder {
    api_key: String,
    endpoint: Option<Url>,
    http_client: Option<ClientWithMiddleware>,
    ref_id: Option<String>,
}

impl SmsOnlineClientBuilder {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            endpoint: None,
            http_client: None,
            ref_id: None,
        }
    }

    pub fn endpoint(mut self, endpoint: Url) -> Self {
        self.endpoint = Some(endpoint);
        self
    }

    pub fn http_client(mut self, client: ClientWithMiddleware) -> Self {
        self.http_client = Some(client);
        self
    }

    pub fn ref_id(mut self, ref_id: impl Into<String>) -> Self {
        self.ref_id = Some(ref_id.into());
        self
    }

    pub fn build(self) -> Result<SmsOnline> {
        let endpoint = self
            .endpoint
            .unwrap_or_else(|| Url::parse(DEFAULT_API_URL).expect("Invalid default URL"));

        let http_client = match self.http_client {
            Some(client) => client,
            None => {
                let client = reqwest::Client::builder()
                    .build()
                    .map_err(SmsOnlineError::BuildHttpClient)?;
                ClientBuilder::new(client).build()
            }
        };

        Ok(SmsOnline {
            http_client,
            api_key: SecretString::from(self.api_key),
            endpoint,
            ref_id: self.ref_id,
        })
    }
}

impl SmsOnline {
    pub fn new(endpoint: impl AsRef<str>, api_key: impl Into<String>) -> Result<Self> {
        let url = Url::parse(endpoint.as_ref()).map_err(SmsOnlineError::InvalidUrl)?;
        Self::builder(api_key).endpoint(url).build()
    }

    pub fn with_api_key(api_key: impl Into<String>) -> Result<Self> {
        Self::builder(api_key).build()
    }

    pub fn builder(api_key: impl Into<String>) -> SmsOnlineClientBuilder {
        SmsOnlineClientBuilder::new(api_key)
    }

    fn build_request_url(&self, action: &str, mut params: Vec<(&str, String)>) -> Result<Url> {
        let mut endpoint = self.endpoint.clone();

        params.push(("api_key", self.api_key.expose_secret().to_string()));
        params.push(("action", action.to_string()));

        endpoint.set_query(Some(
            &serde_urlencoded::to_string(&params).map_err(SmsOnlineError::BuildRequestUrl)?,
        ));

        Ok(endpoint)
    }

    async fn send_request(&self, url: Url) -> Result<String> {
        let response = self
            .http_client
            .get(url)
            .send()
            .await
            .map_err(SmsOnlineError::HttpRequest)?;

        response.text().await.map_err(SmsOnlineError::ParseResponse)
    }

    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(
            name = "get_phone_number",
            target = "sms.online.client",
            skip_all,
            fields(
                service = %service.code(),
                country = %country.iso_short_name(),
                task_id = tracing::field::Empty,
                phone_number = tracing::field::Empty,
            )
        )
    )]
    pub async fn get_phone_number(
        &self,
        country: Country,
        service: Service,
        options: Option<&GetNumberOptions>,
    ) -> Result<GetPhoneNumberResponse> {
        #[cfg(feature = "tracing")]
        Span::current().set_attribute("otel.kind", "client");

        let result = self.get_phone_number_inner(country, service, options).await;

        #[cfg(feature = "tracing")]
        match &result {
            Ok(data) => {
                Span::current()
                    .record("task_id", data.task_id.as_ref())
                    .record("phone_number", crate::utils::REDACTED);
                set_span_ok();
            }
            Err(e) => set_span_error(e),
        }

        result
    }

    async fn get_phone_number_inner(
        &self,
        country: Country,
        service: Service,
        options: Option<&GetNumberOptions>,
    ) -> Result<GetPhoneNumberResponse> {
        let country_id = country
            .sms_online_id()
            .map_err(|_| SmsOnlineError::CountryMapping {
                country: Box::new(country),
            })?;

        let mut params = vec![
            ("service", service.code().to_string()),
            ("country", country_id.to_string()),
        ];

        let ref_id = options
            .and_then(|o| o.ref_id.clone())
            .or_else(|| self.ref_id.clone());
        if let Some(ref_id) = ref_id {
            params.push(("ref", ref_id));
        }

        if let Some(opts) = options {
            if let Some(operator) = &opts.operator {
                params.push(("operator", operator.clone()));
            }
            if let Some(activation_type) = opts.activation_type {
                params.push(("activationType", activation_type.code().to_string()));
            }
            if let Some(provider) = &opts.provider {
                params.push(("provider", provider.as_str().to_string()));
            }
        }

        let url = self.build_request_url("getNumber", params)?;
        let text = self.send_request(url).await?;
        let raw = SmsOnlineTextResponse::from_text(&text)
            .into_result()
            .map_err(SmsOnlineError::Service)?;

        GetPhoneNumberResponse::from_raw(&raw)
            .ok_or_else(|| SmsOnlineError::FailedToParseGetNumberResponse { raw: raw.clone() })
    }

    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(
            name = "get_sms_code",
            target = "sms.online.client",
            skip_all,
            fields(
                task_id = %task_id,
                sms_code = tracing::field::Empty,
            )
        )
    )]
    pub async fn get_sms_code(&self, task_id: &TaskId) -> Result<GetSmsStatusResponse> {
        #[cfg(feature = "tracing")]
        Span::current().set_attribute("otel.kind", "client");

        let result = self.get_sms_code_inner(task_id).await;

        #[cfg(feature = "tracing")]
        match &result {
            Ok(data) => {
                if matches!(data, GetSmsStatusResponse::Ok { .. }) {
                    Span::current().record("sms_code", crate::utils::REDACTED);
                }
                set_span_ok();
            }
            Err(e) => set_span_error(e),
        }

        result
    }

    async fn get_sms_code_inner(&self, task_id: &TaskId) -> Result<GetSmsStatusResponse> {
        let url = self.build_request_url("getStatus", vec![("id", task_id.to_string())])?;
        let text = self.send_request(url).await?;

        let raw = SmsOnlineTextResponse::from_text(&text)
            .into_result()
            .map_err(SmsOnlineError::Service)?;

        GetSmsStatusResponse::from_raw(&raw)
            .ok_or_else(|| SmsOnlineError::FailedToParseGetStatusResponse { raw: raw.clone() })
    }

    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(
            name = "set_activation_status",
            target = "sms.online.client",
            skip_all,
            fields(
                task_id = %task_id,
                status = %status,
                response = tracing::field::Empty,
            )
        )
    )]
    pub async fn set_activation_status(
        &self,
        task_id: &TaskId,
        status: ActivationStatus,
    ) -> Result<SetStatusResponse> {
        #[cfg(feature = "tracing")]
        Span::current().set_attribute("otel.kind", "client");

        let result = self.set_activation_status_inner(task_id, status).await;

        #[cfg(feature = "tracing")]
        match &result {
            Ok(data) => {
                Span::current().record("response", data.to_string());
                set_span_ok();
            }
            Err(e) => set_span_error(e),
        }

        result
    }

    async fn set_activation_status_inner(
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
        let raw = SmsOnlineTextResponse::from_text(&text)
            .into_result()
            .map_err(SmsOnlineError::Service)?;

        SetStatusResponse::from_raw(&raw)
            .ok_or_else(|| SmsOnlineError::FailedToParseSetStatusResponse { raw: raw.clone() })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::sms_online::errors::SmsOnlineErrorCode;
    use keshvar::Alpha2;
    use wiremock::matchers::{method, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn test_get_phone_number_success() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(query_param("action", "getNumber"))
            .and(query_param("service", "ig"))
            .respond_with(
                ResponseTemplate::new(200).set_body_string("ACCESS_NUMBER:123456789:380501234567"),
            )
            .mount(&mock_server)
            .await;

        let client = SmsOnline::new(mock_server.uri(), "test_key").unwrap();
        let result = client
            .get_phone_number(Alpha2::UA.to_country(), Service::InstagramThreads, None)
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
            .and(query_param("action", "getNumber"))
            .and(query_param("service", "wa"))
            .respond_with(ResponseTemplate::new(200).set_body_string("NO_NUMBERS"))
            .mount(&mock_server)
            .await;

        let client = SmsOnline::new(mock_server.uri(), "test_key").unwrap();
        let result = client
            .get_phone_number(Alpha2::UA.to_country(), Service::Whatsapp, None)
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            SmsOnlineError::Service(error) => {
                assert_eq!(error.code, SmsOnlineErrorCode::NoNumbers);
            }
            _ => panic!("Expected Service error"),
        }
    }

    #[tokio::test]
    async fn test_get_sms_code_success() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(query_param("action", "getStatus"))
            .respond_with(ResponseTemplate::new(200).set_body_string("STATUS_OK:123456"))
            .mount(&mock_server)
            .await;

        let client = SmsOnline::new(mock_server.uri(), "test_key").unwrap();
        let result = client.get_sms_code(&TaskId::from("123456789")).await;

        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            GetSmsStatusResponse::Ok {
                code: "123456".to_string()
            }
        );
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

        let client = SmsOnline::new(mock_server.uri(), "test_key").unwrap();
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
