//! SMS verification service with polling and timeout handling.

pub(crate) mod activation;
pub(crate) mod builder;
pub(crate) mod config;
pub(crate) mod core;
pub(crate) mod error;
#[cfg(feature = "metrics")]
pub(crate) mod telemetry;
pub(crate) mod traits;

pub use activation::ActivationHandle;
pub use builder::SmsSolverServiceBuilder;
pub use config::{ConfigError, SmsSolverServiceConfig, SmsSolverServiceConfigBuilder};
pub use core::SmsSolverService;
pub use error::SmsSolverServiceError;
pub use traits::SmsSolverServiceTrait;
