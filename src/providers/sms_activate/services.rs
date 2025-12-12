//! Service definitions for SMS Activate API.

use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// SMS Activate service identifiers.
///
/// Each service represents a different verification target (app/website).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Service {
    /// Full rent (code: "full").
    FullRent,
    /// Instagram/Threads (code: "ig").
    InstagramThreads,
    /// WhatsApp (code: "wa").
    Whatsapp,
    /// Facebook (code: "fb").
    Facebook,
    /// VFS Global (code: "afp").
    Vfs,
    /// Other/custom service.
    Other { code: String },
}

impl Service {
    /// Get the service code for the API.
    pub fn code(&self) -> &str {
        match self {
            Service::FullRent => "full",
            Service::InstagramThreads => "ig",
            Service::Whatsapp => "wa",
            Service::Facebook => "fb",
            Service::Vfs => "afp",
            Service::Other { code } => code.as_str(),
        }
    }

    /// Create a Service from a code string.
    pub fn from_code<S: AsRef<str>>(code: S) -> Self {
        match code.as_ref() {
            "full" => Service::FullRent,
            "ig" => Service::InstagramThreads,
            "wa" => Service::Whatsapp,
            "fb" => Service::Facebook,
            "afp" => Service::Vfs,
            other => Service::Other {
                code: other.to_string(),
            },
        }
    }
}

impl FromStr for Service {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Service::from_code(s))
    }
}

impl Serialize for Service {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.code())
    }
}

impl<'de> Deserialize<'de> for Service {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(Service::from_code(s))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_code() {
        assert_eq!(Service::Whatsapp.code(), "wa");
        assert_eq!(Service::Facebook.code(), "fb");
    }

    #[test]
    fn test_service_from_code() {
        assert_eq!(Service::from_code("wa"), Service::Whatsapp);
        assert_eq!(
            Service::from_code("custom"),
            Service::Other {
                code: "custom".to_string()
            }
        );
    }

    #[test]
    fn test_service_serde() {
        let service = Service::InstagramThreads;
        let json = serde_json::to_string(&service).unwrap();
        assert_eq!(json, "\"ig\"");

        let parsed: Service = serde_json::from_str("\"ig\"").unwrap();
        assert_eq!(parsed, Service::InstagramThreads);
    }
}
