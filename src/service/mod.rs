//! SMS verification service with polling and timeout handling.

pub(crate) mod config;
pub(crate) mod error;
pub(crate) mod structure;
pub(crate) mod traits;

pub use config::{ConfigError, SmsSolverServiceConfig, SmsSolverServiceConfigBuilder};
pub use error::SmsSolverServiceError;
pub use structure::{SmsSolverService, SmsSolverServiceBuilder};
pub use traits::SmsSolverServiceTrait;
