# Architecture

## Layering

```mermaid
classDiagram
    direction TB

    class Provider {
        <<trait>>
        +type Error
        +type Service
        +get_phone_number(country, service) Future~Result~
        +get_sms_code(task_id) Future~Result~Option~~
        +finish_activation(task_id) Future~Result~
        +cancel_activation(task_id) Future~Result~
        +is_dial_code_supported(dial_code) bool
    }

    class ProviderCapabilities {
        <<trait>>
        +type Service
        +supports_service(service) Option~bool~
        +available_countries(service) Option~Vec~Country~~
        +supported_services() Option~Vec~Service~~
    }

    class SmsSolverService~P~ {
        -provider: P
        -config: SmsSolverServiceConfig
        +try_new(provider, config) Result
        +builder(provider) SmsSolverServiceBuilder
        +activate(country, service) Result~ActivationHandle~
        +get_number(country, service) Result~SmsTaskResult~
        +wait_for_sms_code(task_id) Result~SmsCode~
    }

    class ActivationHandle~P~ {
        -service: SmsSolverService~P~
        -result: SmsTaskResult
        +task_id() TaskId
        +full_number() FullNumber
        +wait_for_code() Result~SmsCode~
        +finish() Result
        +cancel() Result
    }

    class SmsRetryableProvider~P~ {
        -inner: Arc~P~
        -retry_config: RetryConfig
        -on_retry: Option~Callback~
    }

    class HeroSmsProvider {
        -client: HeroSms
        -blacklisted_dial_codes: HashSet
        -options: Option~GetNumberOptions~
    }

    class SmsOnlineProvider {
        -client: SmsOnline
        -blacklisted_dial_codes: HashSet
        -options: Option~GetNumberOptions~
    }

    class SmsSolverServiceConfig {
        +timeout: Duration
        +poll_interval: Duration
        +fast() Self
        +balanced() Self
        +patient() Self
        +validate() Result
    }

    class RetryableError {
        <<trait>>
        +is_retryable() bool
        +should_retry_operation() bool
    }

    SmsSolverService --> Provider : uses P
    SmsSolverService --> SmsSolverServiceConfig
    SmsSolverService --> ActivationHandle : creates
    SmsRetryableProvider ..|> Provider : implements
    SmsRetryableProvider --> Provider : wraps P
    SmsRetryableProvider ..|> ProviderCapabilities : conditionally
    HeroSmsProvider ..|> Provider : implements
    HeroSmsProvider ..|> ProviderCapabilities : implements
    SmsOnlineProvider ..|> Provider : implements
    SmsOnlineProvider ..|> ProviderCapabilities : implements
    Provider --> RetryableError : Error must impl
```

## Activation Lifecycle

```mermaid
sequenceDiagram
    participant User
    participant Service as SmsSolverService
    participant Handle as ActivationHandle
    participant Retry as SmsRetryableProvider
    participant Provider as HeroSms/SmsOnline

    User->>Service: activate(country, service)
    Service->>Retry: get_phone_number(country, service)
    loop Retry on transient errors
        Retry->>Provider: HTTP GET /getNumber
        Provider-->>Retry: Response
    end
    Retry-->>Service: (TaskId, FullNumber, DialCode)
    Service-->>User: ActivationHandle

    User->>Handle: wait_for_code()
    loop Poll until timeout
        Handle->>Service: get_sms_code(task_id)
        Service->>Retry: get_sms_code(task_id)
        Retry->>Provider: HTTP GET /getStatus
        Provider-->>Retry: Response
        alt SMS received
            Retry-->>Service: Ok(Some(code))
            Service-->>Handle: SmsCode
            Handle-->>User: Ok(SmsCode)
        else Still waiting
            Retry-->>Service: Ok(None)
            Note over Service: sleep(poll_interval)
        else Timeout reached
            Service->>Provider: cancel_activation(task_id)
            Service-->>Handle: Err(SmsTimeout)
            Handle-->>User: Err(SmsTimeout)
        end
    end

    User->>Handle: finish()
    Handle->>Provider: finish_activation(task_id)
    Provider-->>Handle: Ok
    Handle-->>User: Ok
```

## Error Classification

```mermaid
flowchart TD
    Error([Provider Error]) --> Retryable{is_retryable?}

    Retryable -->|Yes| RetryExamples["NO_NUMBERS
    ERROR_SQL
    CHANNELS_LIMIT
    HTTP timeout"]
    RetryExamples --> SameTask[Retry same task<br/>with backoff]

    Retryable -->|No| RetryOp{should_retry_operation?}

    RetryOp -->|Yes| RetryOpExamples["NO_ACTIVATION
    WRONG_ACTIVATION_ID
    SmsTimeout"]
    RetryOpExamples --> FreshTask[Cancel current,<br/>start fresh activation]

    RetryOp -->|No| Permanent["BAD_KEY
    BAD_SERVICE
    NO_BALANCE
    BANNED
    Parse errors"]
    Permanent --> Fail[Return error to caller]
```

## Error Ownership

| Layer | Error Type | Responsibility |
|-------|-----------|----------------|
| HTTP client | `HeroSmsError` / `SmsOnlineError` | Transport, parsing, service-level errors |
| Provider trait | `Provider::Error` | Classified via `RetryableError` (retryable vs permanent) |
| Retry wrapper | `P::Error` | Passes through; retries based on `is_retryable()` |
| Service | `SmsSolverServiceError` | Wraps provider errors, adds timeout/cancel/blacklist errors |

### RetryableError Two-Level Classification

```rust
pub trait RetryableError {
    fn is_retryable(&self) -> bool;           // Retry the SAME task
    fn should_retry_operation(&self) -> bool;  // Retry with a FRESH task
}
```

- `is_retryable()` = true: transient error, same activation can be retried (e.g., `NO_NUMBERS`, HTTP timeout)
- `should_retry_operation()` = true: permanent for this activation, but a new attempt might succeed (e.g., `NO_ACTIVATION`, `WRONG_ACTIVATION_ID`)

## Retry Policy

`SmsRetryableProvider<P>` uses the `backon` crate for exponential backoff. Retry decisions are driven by `is_retryable()` on the provider's error type. The retry wrapper:

1. Only retries `get_phone_number` and `get_sms_code` (not finish/cancel)
2. Calls `is_retryable()` on each error to decide whether to retry
3. Supports an optional `on_retry` callback for observability

## Lifecycle Model

1. **Acquire**: `service.activate(country, service)` → `ActivationHandle`
2. **Poll**: `handle.wait_for_code()` — polls with configured interval until timeout
3. **Complete**: `handle.finish()` — marks activation as used
4. **Cancel**: `handle.cancel()` — releases the number (also called automatically on timeout/error)

The `ActivationHandle` consumes itself on `finish()` and `cancel()` to prevent use-after-complete.

Legacy flow (`get_number` + `wait_for_sms_code`) remains available for backward compatibility.

## Capability Discovery

Discovery methods (`supports_service`, `available_countries`, `supported_services`) are separated from the core `Provider` trait into `ProviderCapabilities`:

```rust
pub trait ProviderCapabilities: Send + Sync {
    type Service: Clone + Send + Sync;
    fn supports_service(&self, service: &Self::Service) -> Option<bool>;
    fn available_countries(&self, service: &Self::Service) -> Option<Vec<Country>>;
    fn supported_services(&self) -> Option<Vec<Self::Service>>;
}
```

Returns `Option<T>` instead of definitive answers. `None` means the provider cannot determine the answer reliably (honest unknown).

## Extension Points

### Adding a new provider

See [`docs/adding-provider.md`](adding-provider.md).

### Configuration

`SmsSolverServiceConfig` supports presets (`fast()`, `balanced()`, `patient()`) and custom values via builder. All configs are validated on construction via `try_new` / `try_build`.

### Feature flags

| Flag | Default | Description |
|------|---------|-------------|
| `hero-sms` | yes | Hero SMS provider |
| `sms-online` | yes | SMS.online provider |
| `tracing` | yes | OpenTelemetry tracing spans |
| `metrics` | no | OpenTelemetry counters/histograms |
| `random` | yes | Random dial code selection |
