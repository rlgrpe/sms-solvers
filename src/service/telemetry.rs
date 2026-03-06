//! Service metrics for OpenTelemetry.

#[cfg(feature = "metrics")]
use opentelemetry::{
    global,
    metrics::{Counter, Histogram},
};
#[cfg(feature = "metrics")]
use std::sync::OnceLock;

/// Metrics for the SMS Solver service.
#[cfg(feature = "metrics")]
pub(crate) struct ServiceMetrics {
    /// Counter for number requests.
    pub numbers_requested: Counter<u64>,
    /// Counter for successful SMS codes received.
    pub sms_codes_received: Counter<u64>,
    /// Counter for timeouts.
    pub timeouts: Counter<u64>,
    /// Counter for cancellations.
    pub cancellations: Counter<u64>,
    /// Counter for errors.
    pub errors: Counter<u64>,
    /// Histogram for SMS wait times in seconds.
    pub sms_wait_time: Histogram<f64>,
    /// Histogram for poll counts.
    pub poll_counts: Histogram<u64>,
}

#[cfg(feature = "metrics")]
impl ServiceMetrics {
    pub fn global() -> &'static Self {
        static METRICS: OnceLock<ServiceMetrics> = OnceLock::new();
        METRICS.get_or_init(|| {
            let meter = global::meter("sms_solvers");
            Self {
                numbers_requested: meter
                    .u64_counter("sms_solvers.numbers_requested")
                    .with_description("Number of phone number requests")
                    .build(),
                sms_codes_received: meter
                    .u64_counter("sms_solvers.sms_codes_received")
                    .with_description("Number of SMS codes successfully received")
                    .build(),
                timeouts: meter
                    .u64_counter("sms_solvers.timeouts")
                    .with_description("Number of SMS wait timeouts")
                    .build(),
                cancellations: meter
                    .u64_counter("sms_solvers.cancellations")
                    .with_description("Number of cancelled operations")
                    .build(),
                errors: meter
                    .u64_counter("sms_solvers.errors")
                    .with_description("Number of errors")
                    .build(),
                sms_wait_time: meter
                    .f64_histogram("sms_solvers.sms_wait_time_seconds")
                    .with_description("Time spent waiting for SMS codes")
                    .build(),
                poll_counts: meter
                    .u64_histogram("sms_solvers.poll_counts")
                    .with_description("Number of polls before receiving SMS")
                    .build(),
            }
        })
    }
}
