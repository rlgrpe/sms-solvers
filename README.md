# SMS Solvers

A Rust library for SMS verification services. Supports [Hero SMS](https://hero-sms.com/) and
[SMS.online](https://sms.online/) with a flexible provider architecture that allows adding new SMS providers.

> **Disclaimer**: This library is provided as-is. I am not obligated to maintain it, fix bugs, or add features. If you
> want to contribute improvements, please submit a pull request.

## Features

- Async/await support with Tokio
- Generic `Provider` trait for implementing SMS services
- `ActivationHandle` for ergonomic lifecycle management
- Built-in retry logic with configurable backoff
- Validated configuration (timeouts, poll intervals)
- Country code mapping (ISO to provider-specific IDs)
- Dial code blacklisting support
- Optional tracing/OpenTelemetry integration
- Optional OpenTelemetry metrics (counters, histograms)

## Architecture

```text
SmsSolverService<P>          High-level: timeout, polling, lifecycle
    │
    ├── ActivationHandle     Lifecycle object (wait/finish/cancel)
    │
    ▼
SmsRetryableProvider<P>      Optional retry decorator
    │
    ▼
Provider trait               Core async interface
    │
    ▼
HeroSms / SmsOnline          HTTP client for provider API
```

See [`docs/architecture.md`](docs/architecture.md) for details.

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
sms-solvers = { git = "https://github.com/rlgrpe/sms-solvers.git", tag = "v0.5.0" }
tokio = { version = "1", features = ["full"] }
```

## Quick Start (Hero SMS)

```rust
use sms_solvers::hero_sms::{HeroSms, HeroSmsProvider, Service};
use sms_solvers::{Alpha2, SmsSolverService, SmsRetryableProvider};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = HeroSms::with_api_key("your_api_key")?;
    let provider = HeroSmsProvider::new(client);
    let retryable = SmsRetryableProvider::new(provider);

    let service = SmsSolverService::builder(retryable)
        .timeout(std::time::Duration::from_secs(120))
        .poll_interval(std::time::Duration::from_secs(3))
        .try_build()?;

    // Get a number and wait for SMS
    let activation = service.activate(Alpha2::UA.to_country(), Service::InstagramThreads).await?;
    println!("Got number: +{}", activation.full_number());

    let code = activation.wait_for_code().await?;
    println!("Received code: {}", code);

    activation.finish().await?;
    Ok(())
}
```

## Quick Start (SMS.online)

```rust
use sms_solvers::sms_online::{SmsOnline, SmsOnlineProvider, Service};
use sms_solvers::{Alpha2, SmsSolverService, SmsRetryableProvider};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = SmsOnline::with_api_key("your_api_key")?;
    let provider = SmsOnlineProvider::new(client);
    let retryable = SmsRetryableProvider::new(provider);

    let service = SmsSolverService::builder(retryable)
        .timeout(std::time::Duration::from_secs(120))
        .poll_interval(std::time::Duration::from_secs(3))
        .try_build()?;

    let activation = service.activate(Alpha2::GB.to_country(), Service::Whatsapp).await?;
    println!("Got number: +{}", activation.full_number());

    let code = activation.wait_for_code().await?;
    println!("Received code: {}", code);

    activation.finish().await?;
    Ok(())
}
```

## Using Retry Logic

Wrap any provider with `SmsRetryableProvider` for automatic retry on transient errors:

```rust
use sms_solvers::hero_sms::{HeroSms, HeroSmsProvider};
use sms_solvers::{RetryConfig, SmsRetryableProvider, SmsSolverService};
use std::time::Duration;

let client = HeroSms::with_api_key("your_api_key")?;
let provider = HeroSmsProvider::new(client);

let retry_config = RetryConfig::default()
    .with_min_delay(Duration::from_millis(500))
    .with_max_delay(Duration::from_secs(5))
    .with_max_retries(3);

let retryable_provider = SmsRetryableProvider::with_config(provider, retry_config);
let service = SmsSolverService::with_provider(retryable_provider);
```

## Using the Builder Pattern

```rust
use sms_solvers::{SmsSolverService, SmsRetryableProvider};
use std::time::Duration;

let service = SmsSolverService::builder(SmsRetryableProvider::new(provider))
    .timeout(Duration::from_secs(180))
    .poll_interval(Duration::from_secs(5))
    .try_build()?;
```

## Using the Provider Directly

You can use any provider without the service layer:

```rust
use sms_solvers::{Alpha2, Provider};
use sms_solvers::hero_sms::{HeroSmsProvider, Service};

let provider = HeroSmsProvider::new(client);

// Get a phone number
let (task_id, full_number, _dial_code) = provider
    .get_phone_number(Alpha2::US.to_country(), Service::Whatsapp)
    .await?;

// Poll for SMS code
let sms_code = provider.get_sms_code(&task_id).await?;

// Finish or cancel activation
provider.finish_activation(&task_id).await?;
// or
provider.cancel_activation(&task_id).await?;
```

## Dial Code Blacklisting

Block specific dial codes from being used:

```rust
use sms_solvers::hero_sms::{HeroSms, HeroSmsProvider};
use sms_solvers::DialCode;
use std::collections::HashSet;

let client = HeroSms::with_api_key("your_api_key")?;
let blacklist: HashSet<DialCode> = ["33", "49"]
    .into_iter()
    .map(|s| DialCode::new(s).unwrap())
    .collect();
let provider = HeroSmsProvider::with_blacklist(client, blacklist);

// Or add after creation
let mut provider = HeroSmsProvider::new(client);
provider.blacklist_dial_code(DialCode::new("33").unwrap());
```

## Country Code Mapping

The library maps ISO country codes to provider-specific IDs. `Alpha2` and `Country` are
re-exported from `keshvar`:

```rust
use sms_solvers::hero_sms::SmsCountryExt;
use sms_solvers::sms_online::SmsOnlineCountryExt;
use sms_solvers::{Alpha2, Country};

// Hero SMS
let sms_id = Alpha2::UA.to_country().sms_id()?;           // Returns 1
let country = Country::from_sms_id(1)?;                    // Returns Ukraine

// SMS.online
let sms_id = Alpha2::UA.to_country().sms_online_id()?;    // Returns 1
let country = Country::from_sms_online_id(1)?;             // Returns Ukraine
```

## Running Examples

```bash
# Hero SMS examples
export HERO_SMS_API_KEY=your_api_key
cargo run --example basic_usage
cargo run --example with_retry
cargo run --example with_cancellation
cargo run --example retry_callbacks

# SMS.online examples
export SMS_ONLINE_API_KEY=your_api_key
cargo run --example sms_online_basic
cargo run --example sms_online_with_retry

# No API key needed
cargo run --example config_presets
cargo run --example country_mapping
cargo run --example sms_online_country_mapping
```

## Running Tests

```bash
# Run unit tests
cargo test --all-features

# Run Hero SMS integration tests (requires API key, consumes credits)
HERO_SMS_API_KEY=your_key cargo test --test hero_sms_api -- --ignored

# Run SMS.online integration tests (requires API key, consumes credits)
SMS_ONLINE_API_KEY=your_key cargo test --test sms_online_api -- --ignored
```

## Feature Flags

| Flag | Default | Description |
|------|---------|-------------|
| `hero-sms` | yes | Hero SMS provider support |
| `sms-online` | yes | SMS.online provider support |
| `tracing` | yes | OpenTelemetry tracing instrumentation |
| `metrics` | no | OpenTelemetry metrics (counters, histograms) |
| `random` | yes | Random dial code selection |
| `native-tls` | yes | Native TLS backend |
| `rustls-tls` | no | Rustls TLS backend (alternative) |

## Adding a New Provider

See [`docs/adding-provider.md`](docs/adding-provider.md) for a guide on implementing new SMS providers.

## License

MIT
