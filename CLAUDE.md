# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build Commands

```bash
# Build
cargo build --all-features

# Test (unit tests)
cargo test --all-features

# Test (integration tests - requires API key, consumes credits)
SMS_ACTIVATE_API_KEY=your_key cargo test --test sms_activate_api -- --ignored

# Run a single test
cargo test test_name --all-features

# Lint
cargo clippy --all-targets --all-features -- -D warnings

# Format
cargo fmt --all

# Format check
cargo fmt --all -- --check

# Build examples
cargo build --examples --all-features

# Run example
SMS_ACTIVATE_API_KEY=your_key cargo run --example basic_usage
```

## Architecture

```
SmsSolverService<P>          High-level service with timeout/polling
        │
        ▼
SmsRetryableProvider<P>      Optional retry wrapper (exponential backoff)
        │
        ▼
    Provider                 Trait implemented by SmsActivateProvider, etc.
        │
        ▼
  SmsActivateClient          HTTP client for SMS Activate API
```

### Key Abstractions

- **`Provider` trait** (`src/providers/traits.rs`): Core interface for SMS providers. Async methods return
  `impl Future + Send`. Implementations must be `Clone + Send + Sync`.

- **`SmsSolverService<P>`** (`src/service/structure.rs`): Wraps any `Provider` with polling logic, timeouts, and
  cancellation support. Uses config presets (`fast()`, `balanced()`, `patient()`).

- **`SmsRetryableProvider<P>`** (`src/providers/retryable/mod.rs`): Decorator that adds retry logic using `backon`
  crate. Uses `Arc<P>` internally to avoid cloning providers.

- **`RetryableError` trait** (`src/errors.rs`): Two-level retry classification - `is_retryable()` for same-task retries,
  `should_retry_operation()` for fresh attempts.

### SMS Activate Provider

- **Country mapping** (`src/providers/sms_activate/countries.rs`): Static JSON maps ISO country codes to SMS Activate
  numeric IDs. Uses `SmsCountryExt` trait extension.

- **Error handling** (`src/providers/sms_activate/errors.rs`): Parses API error strings with regex, classifies into
  retryable vs permanent.

- **Services** (`src/providers/sms_activate/services.rs`): Enum of supported services (WhatsApp, Instagram, etc.) with
  `Other { code }` for custom services.

## Feature Flags

- `sms-activate` (default): SMS Activate provider support
- `tracing` (default): OpenTelemetry tracing instrumentation
- `metrics`: OpenTelemetry metrics (counters, histograms)

## Type Patterns

- Newtype wrappers: `TaskId`, `FullNumber`, `Number`, `DialCode`, `SmsCode` for type safety
- Builder pattern: `SmsSolverService::builder(provider).timeout(...).build()`
- Config presets: `SmsSolverServiceConfig::fast()`, `balanced()`, `patient()`
- Validation: `config.validate()` or `builder.try_build()`
