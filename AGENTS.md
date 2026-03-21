# AGENTS.md

Commands for AI agents and automated tools working with this repository.

## Build Commands

```bash
# Build with all features
cargo build --all-features

# Build release
cargo build --release --all-features
```

## Test Commands

```bash
# Run unit tests
cargo test --all-features

# Run specific test
cargo test test_name --all-features

# Run Hero SMS integration tests (requires API key, consumes credits)
HERO_SMS_API_KEY=your_key cargo test --test hero_sms_api -- --ignored

# Run SMS.online integration tests (requires API key, consumes credits)
SMS_ONLINE_API_KEY=your_key cargo test --test sms_online_api -- --ignored
```

## Lint Commands

```bash
# Format check
cargo fmt --all -- --check

# Format fix
cargo fmt --all

# Clippy
cargo clippy --all-targets --all-features -- -D warnings

# Clippy with cognitive complexity
cargo clippy --all-targets --all-features -- -D warnings -D clippy::cognitive_complexity
```

## Documentation

```bash
# Generate docs
cargo doc --all-features --no-deps

# Open docs
cargo doc --all-features --no-deps --open
```

## Project Structure

```
src/
├── lib.rs              # Library entry point, re-exports
├── errors.rs           # Error types, RetryableError trait
├── types.rs            # Core types: TaskId, FullNumber, SmsCode, etc.
├── utils/
│   ├── mod.rs          # Utility re-exports
│   ├── retry.rs        # RetryConfig for backoff parameters
│   ├── error_chain.rs  # Error chain formatting helper
│   └── span_status.rs  # OpenTelemetry span status helper
├── providers/
│   ├── mod.rs          # Provider module root, re-exports
│   ├── traits.rs       # Provider trait definition
│   ├── capabilities.rs # ProviderCapabilities trait
│   ├── common/
│   │   ├── countries.rs # Shared country mapping utilities
│   │   └── services.rs  # Shared Service enum
│   ├── retryable/
│   │   └── mod.rs      # SmsRetryableProvider decorator
│   ├── hero_sms/
│   │   ├── mod.rs
│   │   ├── client.rs   # HTTP client
│   │   ├── provider.rs # Provider trait implementation
│   │   ├── countries.rs # Country code mapping
│   │   ├── errors.rs   # Error parsing and classification
│   │   ├── services.rs # Re-exports shared Service
│   │   ├── types.rs    # Request/response types
│   │   └── response.rs # Response parsing helpers
│   └── sms_online/
│       ├── mod.rs
│       ├── client.rs   # HTTP client
│       ├── provider.rs # Provider trait implementation
│       ├── countries.rs # Country code mapping
│       ├── errors.rs   # Error parsing and classification
│       ├── services.rs # Re-exports shared Service
│       ├── types.rs    # Request/response types
│       └── response.rs # Response parsing helpers
└── service/
    ├── mod.rs
    ├── core.rs         # SmsSolverService implementation
    ├── activation.rs   # ActivationHandle lifecycle object
    ├── builder.rs      # SmsSolverServiceBuilder
    ├── config.rs       # Configuration and presets
    ├── error.rs        # SmsSolverServiceError
    ├── traits.rs       # SmsSolverServiceTrait
    └── telemetry.rs    # OpenTelemetry metrics (behind `metrics` feature)
```

## Feature Flags

- `hero-sms` (default): Hero SMS provider
- `sms-online` (default): SMS.online provider
- `tracing` (default): OpenTelemetry tracing
- `metrics`: OpenTelemetry metrics
- `random` (default): Random dial code selection
- `native-tls` (default): Native TLS backend
- `rustls-tls`: Rustls TLS backend
