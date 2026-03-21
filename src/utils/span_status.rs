use opentelemetry::trace::Status;
use tracing::Span;
use tracing_opentelemetry::OpenTelemetrySpanExt;

#[inline]
pub(crate) fn set_span_ok() {
    Span::current().set_status(Status::Ok);
}

#[inline]
pub(crate) fn set_span_error(err: &dyn std::fmt::Display) {
    Span::current().set_status(Status::error(err.to_string()));
}
