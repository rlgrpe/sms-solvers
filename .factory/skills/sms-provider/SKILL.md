---
name: sms-provider
description: Add a new SMS provider implementation to the library
---

# SMS Provider Implementation

This skill helps implement a new SMS provider for the sms-solvers library.

## Steps

1. Create provider module in `src/providers/{provider_name}/`
2. Implement the `Provider` trait from `src/providers/traits.rs`
3. Add country code mapping if provider uses custom country IDs
4. Implement error types extending `RetryableError` trait
5. Add integration tests in `tests/{provider_name}_api.rs`
6. Update `src/providers/mod.rs` to export the new provider
7. Add feature flag in `Cargo.toml` if provider should be optional

## Key Files

- `src/providers/traits.rs` - Provider trait definition
- `src/providers/hero_sms/` - Reference implementation
- `src/errors.rs` - RetryableError trait

## Provider Trait

The core trait that must be implemented:

```rust
pub trait Provider: Clone + Send + Sync + 'static {
    type Error: RetryableError + Send + Sync + 'static;

    fn get_phone_number(
        &self,
        country: Country,
        service: impl Into<String> + Send,
    ) -> impl Future<Output = Result<(TaskId, FullNumber), Self::Error>> + Send;

    fn get_sms_code(
        &self,
        task_id: &TaskId,
    ) -> impl Future<Output = Result<Option<SmsCode>, Self::Error>> + Send;

    fn finish_activation(
        &self,
        task_id: &TaskId,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;

    fn cancel_activation(
        &self,
        task_id: &TaskId,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;
}
```

## Error Handling

Implement the `RetryableError` trait to classify errors:

```rust
pub trait RetryableError: std::error::Error {
    fn is_retryable(&self) -> bool;
    fn should_retry_operation(&self) -> bool;
}
```
