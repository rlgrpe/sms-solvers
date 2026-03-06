//! Optional provider capability/discovery traits.

use keshvar::Country;

/// Optional trait for providers that can report their capabilities.
///
/// Not all providers have reliable capability data. Implementations
/// should only provide this when the data is accurate.
///
/// Providers that cannot guarantee accurate capability data should
/// simply not implement this trait.
pub trait ProviderCapabilities: Send + Sync {
    /// Service type for this provider.
    type Service: Clone + Send + Sync;

    /// Check if the provider supports the given service.
    ///
    /// Returns `None` if the provider cannot determine support,
    /// `Some(true)` if supported, `Some(false)` if not.
    fn supports_service(&self, service: &Self::Service) -> Option<bool> {
        let _ = service;
        None
    }

    /// Get the list of countries where the given service is available.
    ///
    /// Returns `None` if the provider cannot determine available countries.
    fn available_countries(&self, service: &Self::Service) -> Option<Vec<Country>> {
        let _ = service;
        None
    }

    /// Get the list of all services supported by this provider.
    ///
    /// Returns `None` if the provider cannot enumerate its services.
    fn supported_services(&self) -> Option<Vec<Self::Service>> {
        None
    }
}
