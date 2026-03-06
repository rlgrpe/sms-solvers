# Adding a New Provider

## Minimum Files

```
src/providers/<name>/
├── mod.rs           Module root with re-exports
├── client.rs        HTTP client (request building, response parsing)
├── provider.rs      Provider trait implementation
├── errors.rs        Error types with RetryableError impl
├── countries.rs     Country ID mapping (uses shared utilities)
├── services.rs      Re-exports shared Service type
├── types.rs         Request/response types
└── response.rs      Response parsing helpers (optional)
```

## Required Invariants

### Provider Trait

Your provider must implement all four core methods:

```rust
impl Provider for YourProvider {
    type Error = YourError;
    type Service = Service; // from providers::common::services

    async fn get_phone_number(&self, country, service)
        -> Result<(TaskId, FullNumber, Option<DialCode>), Self::Error>;
    async fn get_sms_code(&self, task_id: &TaskId)
        -> Result<Option<SmsCode>, Self::Error>;
    async fn finish_activation(&self, task_id: &TaskId) -> Result<(), Self::Error>;
    async fn cancel_activation(&self, task_id: &TaskId) -> Result<(), Self::Error>;
}
```

Key semantics:
- `get_sms_code` returns `Ok(None)` when waiting — the service layer uses this to continue polling
- `get_sms_code` returns `Ok(Some(code))` when the SMS is received
- `get_sms_code` may return `Err(...)` for terminal conditions (e.g., activation cancelled by the API)

### Error Classification

Your error type must implement `RetryableError`:

```rust
impl RetryableError for YourError {
    fn is_retryable(&self) -> bool { /* transient errors only */ }
    fn should_retry_operation(&self) -> bool { /* broader: fresh attempt might succeed */ }
}
```

Rules:
- `is_retryable()` = true: NO_NUMBERS, SQL errors, HTTP timeouts, rate limits
- `is_retryable()` = false: BAD_KEY, BAD_SERVICE, NO_BALANCE, parsing errors
- `should_retry_operation()` = true: includes `is_retryable` cases PLUS activation-not-found errors
- `should_retry_operation()` = false: auth errors, config errors, banned

### Country Mapping

Use the shared utilities from `providers::common::countries`:

```rust
use crate::providers::common::countries::{build_id_to_country_map, build_country_to_id_map};

static COUNTRIES_JSON: &str = include_str!("../../../assets/your_countries.json");

pub static YOUR_ID2COUNTRY: LazyLock<HashMap<u16, Country>> =
    LazyLock::new(|| build_id_to_country_map(COUNTRIES_JSON, "your_countries.json"));
```

The JSON format is `{ "numeric_id": "country_name", ... }`.

### ProviderCapabilities (Optional)

If your provider has discovery data, implement `ProviderCapabilities`:

```rust
impl ProviderCapabilities for YourProvider {
    type Service = Service;
    fn supports_service(&self, _service: &Self::Service) -> Option<bool> { None }
    fn available_countries(&self, _service: &Self::Service) -> Option<Vec<Country>> {
        Some(YOUR_ID2COUNTRY.values().cloned().collect())
    }
    fn supported_services(&self) -> Option<Vec<Self::Service>> { Some(Service::all()) }
}
```

Return `None` when data is unreliable — don't return unconditional `true`.

## Testing Checklist

- [ ] Unit tests for response parsing (success + error cases)
- [ ] Unit tests for error classification (`is_retryable` / `should_retry_operation`)
- [ ] Unit tests for country mapping round-trips
- [ ] Wiremock tests for Provider trait methods
- [ ] Add provider to `tests/provider_contract.rs` contract test suite
- [ ] Integration test (ignored by default, requires API key)

## Wiring Up

1. Add feature flag to `Cargo.toml`
2. Add `#[cfg(feature = "your-provider")] pub mod your_provider;` to `src/providers/mod.rs`
3. Add re-export module to `src/lib.rs`
4. Add JSON country mapping to `assets/`
5. Update README with the new provider
