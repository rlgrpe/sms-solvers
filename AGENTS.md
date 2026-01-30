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

# Run integration tests (requires API key)
HERO_SMS_API_KEY=your_key cargo test --test hero_sms_api -- --ignored
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
├── providers/
│   ├── mod.rs          # Provider trait definition
│   ├── traits.rs       # Provider trait and related types
│   ├── retryable/      # Retry wrapper for providers
│   └── hero_sms/       # Hero SMS provider implementation
│       ├── mod.rs
│       ├── client.rs   # HTTP client
│       ├── countries.rs # Country code mapping
│       ├── errors.rs   # Error parsing
│       └── services.rs # Service enum
└── service/
    ├── mod.rs
    ├── structure.rs    # SmsSolverService implementation
    └── config.rs       # Configuration and presets
```

## Feature Flags

- `hero-sms` (default): Hero SMS provider
- `tracing` (default): OpenTelemetry tracing
- `metrics`: OpenTelemetry metrics
- `random`: Random number generation
- `native-tls` (default): Native TLS backend
- `rustls-tls`: Rustls TLS backend
