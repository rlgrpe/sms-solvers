# SMS Solvers

A Rust library for SMS verification services. Currently supports [Hero SMS](https://hero-sms.com/) with a
flexible provider architecture that allows adding new SMS providers.

> **Disclaimer**: This library is provided as-is. I am not obligated to maintain it, fix bugs, or add features. If you
> want to contribute improvements, please submit a pull request.

## Features

- Async/await support with Tokio
- Generic `Provider` trait for implementing SMS services
- Built-in retry logic with configurable backoff
- Country code mapping (ISO to provider-specific IDs)
- Dial code blacklisting support
- Optional tracing/OpenTelemetry integration

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
sms-solvers = { git = "https://github.com/rlgrpe/sms-solvers.git", tag = "v0.3.0" }
tokio = { version = "1", features = ["full"] }
```

## Quick Start

```rust
use sms_solvers::hero_sms::{HeroSms, HeroSmsProvider, Service};
use sms_solvers::{
    Alpha2, SmsSolverService, SmsSolverServiceConfig, SmsSolverServiceTrait,
};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create client and provider
    let client = HeroSms::with_api_key("your_api_key")?;
    let provider = HeroSmsProvider::new(client);

    // Configure the service
    let config = SmsSolverServiceConfig::default()
        .with_timeout(Duration::from_secs(120))
        .with_poll_interval(Duration::from_secs(3));
    let service = SmsSolverService::new(provider, config);

    // Get a phone number for Instagram verification in Ukraine
    let result = service
        .get_number(Alpha2::UA.to_country(), Service::InstagramThreads)
        .await?;
    println!("Got number: +{}", result.full_number);

    // Wait for SMS code
    let code = service.wait_for_sms_code(&result.task_id).await?;
    println!("Received code: {}", code);

    Ok(())
}
```

## Using Retry Logic

Wrap the provider with `SmsRetryableProvider` for automatic retry on transient errors:

```rust
use sms_solvers::hero_sms::{HeroSms, HeroSmsProvider};
use sms_solvers::{RetryConfig, SmsRetryableProvider, SmsSolverService, SmsSolverServiceConfig};
use std::time::Duration;

let client = HeroSms::with_api_key("your_api_key") ?;
let provider = HeroSmsProvider::new(client);

let retry_config = RetryConfig::default ()
.with_min_delay(Duration::from_millis(500))
.with_max_delay(Duration::from_secs(5))
.with_max_retries(3);

let retryable_provider = SmsRetryableProvider::with_config(provider, retry_config);

let config = SmsSolverServiceConfig::default ();
let service = SmsSolverService::new(retryable_provider, config);
```

## Using the Builder Pattern

```rust
use sms_solvers::{SmsSolverService, SmsRetryableProvider};
use std::time::Duration;

let service = SmsSolverService::builder(SmsRetryableProvider::new(provider))
.timeout(Duration::from_secs(180))
.poll_interval(Duration::from_secs(5))
.build();
```

## Using the Provider Directly

You can use the provider without the service layer:

```rust
use sms_solvers::{Alpha2, Provider};
use sms_solvers::hero_sms::{HeroSmsProvider, Service};

let provider = HeroSmsProvider::new(client);

// Get a phone number
let (task_id, full_number) = provider
.get_phone_number(Alpha2::US.to_country(), Service::Whatsapp)
.await?;

// Poll for SMS code
let sms_code = provider.get_sms_code( & task_id).await?;

// Finish or cancel activation
provider.finish_activation( & task_id).await?;
// or
provider.cancel_activation( & task_id).await?;
```

## Dial Code Blacklisting

Block specific dial codes from being used:

```rust
use sms_solvers::hero_sms::{HeroSms, HeroSmsProvider};
use sms_solvers::DialCode;
use std::collections::HashSet;

let client = HeroSms::with_api_key("your_api_key") ?;
let blacklist: HashSet<DialCode> = ["33", "49"]
.into_iter()
.map( | s| DialCode::new(s).unwrap())
.collect();
let provider = HeroSmsProvider::with_blacklist(client, blacklist);

// Or add after creation
let mut provider = HeroSmsProvider::new(client);
provider.blacklist_dial_code(DialCode::new("33").unwrap());
```

## Supported Services

The library supports various Hero SMS services including:

- `Service::Whatsapp`
- `Service::InstagramThreads`
- `Service::Telegram`
- And many more (see `Service` enum)

## Country Code Mapping

The library automatically maps ISO country codes to Hero SMS IDs. `Alpha2` and `Country` are
re-exported from `keshvar` along with the `SmsCountryExt` helper trait:

```rust
use sms_solvers::hero_sms::SmsCountryExt;
use sms_solvers::{Alpha2, Country};

// Get Hero SMS ID for a country
let sms_id = Alpha2::UA.to_country().sms_id() ?; // Returns 1

// Get country from Hero SMS ID
let country = Country::from_sms_id(1) ?; // Returns Country::Ukraine
```

## Running Examples

```bash
# Set your API key
export HERO_SMS_API_KEY=your_api_key

# Run basic usage example
cargo run --example basic_usage

# Run retry example
cargo run --example with_retry

# Run country mapping demo (no API key needed)
cargo run --example country_mapping
```

## Running Tests

```bash
# Run unit tests
cargo test

# Run integration tests (requires API key, consumes credits)
HERO_SMS_API_KEY=your_key cargo test --test hero_sms_api -- --ignored
```

## Features

Enable optional features in `Cargo.toml`:

```toml
[dependencies]
sms-solvers = { git = "https://github.com/rlgrpe/sms-solvers.git", tag = "v0.3.0", features = ["tracing"] }
```

- `hero-sms` - Hero SMS provider support (enabled by default)
- `tracing` - Enables tracing instrumentation and OpenTelemetry integration (enabled by default)

## Public API

All main types are exported from the crate root:

```rust
use sms_solvers::{
    // Core types
    Alpha2, Country, DialCode, FullNumber, Number, SmsCode, SmsTaskResult, TaskId,
    // Traits
    Provider, RetryableError, SmsSolverServiceTrait,
    // Service
    SmsSolverService, SmsSolverServiceBuilder,
    SmsSolverServiceConfig, SmsSolverServiceConfigBuilder, SmsSolverServiceError,
    // Retry
    RetryConfig, SmsRetryableProvider,
};

// Provider-specific types under hero_sms module
use sms_solvers::hero_sms::{
    HeroSms, HeroSmsProvider, HeroSmsError, Service, SmsCountryExt,
};
```

## License

MIT