# Architecture Remediation Plan

## Goal
- Привести библиотеку к более строгой и устойчивой архитектуре без расползания ответственности между `service` и `providers`, уменьшить внутреннее дублирование, убрать устаревшие и небезопасные практики, и добавить недостающие контракты, тесты и документацию для дальнейшего роста.

## Assumptions
- Базовый курс: сохранить совместимость публичного API там, где это разумно, а жёсткие ломки оставить на следующий major.
- Если совместимость мешает сильно упростить код, вводить новые API с депрекейтом старых, а не держать два полноценных параллельных дизайна долго.
- Приоритет: корректность, безопасность и расширяемость выше, чем минимальный diff.

## Success Criteria
- Сервисный слой отвечает только за orchestration/polling/lifecycle policy.
- Провайдеры отвечают только за интеграцию с внешним API и provider-specific mapping/parsing.
- Конфигурация сервиса не может быть создана в невалидном состоянии без явного opt-out.
- Секретные данные и OTP не попадают в логи.
- Добавление нового провайдера требует минимального boilerplate и проходит общий contract-test suite.
- README и crate docs соответствуют реальному API и структуре crate.

## Phase 0: Baseline Decisions
- [ ] Зафиксировать стратегию совместимости:
  - `semver-safe path`: добавить новые безопасные API, старые отметить `deprecated`.
  - `next-major path`: удалить старые unchecked API и сузить публичные контракты.
- [ ] Зафиксировать желаемый целевой срез сервисного слоя:
  - `service/config.rs` только конфигурация.
  - `service/error.rs` только service-level ошибки.
  - orchestration/polling/builder/telemetry разнесены по отдельным файлам.
- [ ] Зафиксировать целевой контракт провайдера:
  - базовый `Provider` содержит только обязательные lifecycle методы.
  - discovery/capabilities вынесены в отдельный optional trait или capability-объект.

## Phase 1: P0 Hardening

### 1.1 Validate configuration on construction
- [ ] Добавить `SmsSolverService::try_new(provider, config) -> Result<Self, ConfigError>`.
- [ ] Добавить `SmsSolverServiceBuilder::try_build() -> Result<_, ConfigError>`.
- [ ] Решить судьбу unchecked API:
  - сохранить `new/build` как thin wrappers c `debug_assert!` и clear docs.
  - или депрекейтнуть `new/build` в пользу `try_new/try_build`.
- [ ] Обновить примеры и тесты под validated-path.

**Files**
- Modify: `src/service/config.rs`
- Modify: `src/service/structure.rs`
- Modify: `src/lib.rs`
- Modify: `README.md`
- Modify: `examples/*.rs`

**Verification**
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --all-features`

### 1.2 Remove sensitive OTP logging
- [ ] Убрать логирование фактического SMS-кода из service polling.
- [ ] Проверить tracing spans у клиентов и провайдеров на предмет утечки чувствительных полей.
- [ ] При необходимости ввести единый internal helper для redaction.

**Files**
- Modify: `src/service/structure.rs`
- Modify: `src/providers/hero_sms/client.rs`
- Modify: `src/providers/sms_online/client.rs`
- Modify: `src/providers/hero_sms/provider.rs`
- Modify: `src/providers/sms_online/provider.rs`

**Verification**
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --all-features`

### 1.3 Normalize timeout ownership
- [ ] Удалить provider-level timeout variants, которые не используются как реальная ответственность провайдера.
- [ ] Оставить timeout policy только в `SmsSolverServiceError`.
- [ ] Проверить `RetryableError` после удаления timeout cases.

**Files**
- Modify: `src/providers/hero_sms/errors.rs`
- Modify: `src/providers/sms_online/errors.rs`
- Modify: `src/service/error.rs`
- Modify: `src/service/structure.rs`

**Verification**
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --all-features`

## Phase 2: P1 Service Layer Decomposition

### 2.1 Split oversized orchestration module
- [ ] Разбить `src/service/structure.rs` на меньшие модули:
  - `src/service/service.rs` или `core.rs` для `SmsSolverService`
  - `src/service/polling.rs` для loop/timeouts/cancellation/cleanup
  - `src/service/builder.rs` для builder API
  - `src/service/telemetry.rs` для metrics/tracing helpers
- [ ] Обновить `src/service/mod.rs` и реэкспорты в `src/lib.rs`.
- [ ] Сохранить внешний API по именам, если не принято решение о major-break.

**Files**
- Modify: `src/service/mod.rs`
- Add: `src/service/service.rs`
- Add: `src/service/polling.rs`
- Add: `src/service/builder.rs`
- Add: `src/service/telemetry.rs`
- Remove or shrink: `src/service/structure.rs`
- Modify: `src/lib.rs`

**Verification**
- `cargo fmt --all`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --all-features`

### 2.2 Add activation handle
- [ ] Добавить `ActivationHandle`/`SmsActivation` как user-facing lifecycle object.
- [ ] Перенести в него `task_id`, `country`, `full_number`, `dial_code`, `number`.
- [ ] Добавить методы:
  - `wait_for_code()`
  - `wait_for_code_cancellable()`
  - `finish()`
  - `cancel()`
- [ ] Оставить существующий API `get_number` + `wait_for_sms_code` как совместимый слой или thin wrapper.

**Files**
- Add: `src/service/activation.rs`
- Modify: `src/service/traits.rs`
- Modify: `src/service/error.rs`
- Modify: `src/service/mod.rs`
- Modify: `src/lib.rs`
- Modify: `examples/*.rs`
- Modify: `README.md`

**Verification**
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --all-features`

## Phase 3: P1 Provider Contract Cleanup

### 3.1 Slim down core Provider trait
- [ ] Оставить в `Provider` только обязательные lifecycle методы и policy, без слабых discovery-defaults.
- [ ] Вынести capability/discovery API в отдельный optional trait, например:
  - `ProviderCapabilities`
  - или `ProviderCatalog`
- [ ] Перенести туда:
  - `supports_service`
  - `available_countries`
  - `supported_services`
- [ ] Решить судьбу `is_dial_code_supported`:
  - оставить как provider policy.
  - либо вынести в отдельный policy trait, если хотим чистый transport adapter.

**Files**
- Modify: `src/providers/traits.rs`
- Modify: `src/providers/retryable/mod.rs`
- Modify: `src/providers/hero_sms/provider.rs`
- Modify: `src/providers/sms_online/provider.rs`
- Modify: `src/lib.rs`
- Modify: `README.md`

**Verification**
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --all-features`

### 3.2 Make capability data honest
- [ ] Перестать возвращать unconditional `true` и "все страны/все сервисы", если это не гарантировано API.
- [ ] Если достоверных provider-side capability данных нет:
  - явно документировать “best-effort static catalog”.
  - или возвращать `None/Unknown` вместо ложной точности.
- [ ] При необходимости добавить тип:
  - `CapabilitySupport::{Known(bool), Unknown}`
  - или `ProviderCatalogSnapshot`.

**Files**
- Modify: `src/providers/hero_sms/provider.rs`
- Modify: `src/providers/sms_online/provider.rs`
- Add if needed: `src/providers/capabilities.rs`
- Modify: `README.md`

**Verification**
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --all-features`

## Phase 4: P2 Provider Internal Deduplication

### 4.1 Extract common provider internals
- [ ] Создать internal shared module для повторяющихся паттернов:
  - request URL assembly
  - plain-text response wrapping
  - service error parsing helpers
  - redaction helpers
  - maybe shared status parsing helpers
- [ ] Не строить чрезмерно абстрактный framework; выносить только реальный repetition.
- [ ] Свести к минимуму зеркальный код в `hero_sms` и `sms_online`.

**Files**
- Add: `src/providers/common/mod.rs`
- Add: `src/providers/common/http.rs`
- Add: `src/providers/common/parsing.rs`
- Add: `src/providers/common/redaction.rs`
- Modify: `src/providers/mod.rs`
- Modify: `src/providers/hero_sms/client.rs`
- Modify: `src/providers/sms_online/client.rs`
- Modify: `src/providers/hero_sms/response.rs`
- Modify: `src/providers/sms_online/response.rs`
- Modify: `src/providers/hero_sms/errors.rs`
- Modify: `src/providers/sms_online/errors.rs`

**Verification**
- `cargo fmt --all`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --all-features`

### 4.2 Remove obsolete dependencies and legacy patterns
- [ ] Заменить `once_cell` на `std::sync::LazyLock`.
- [ ] Проверить весь crate на устаревшие/ненужные зависимости и удалить лишние.
- [ ] После миграции обновить `Cargo.toml` и related imports.

**Files**
- Modify: `Cargo.toml`
- Modify: `src/providers/hero_sms/errors.rs`
- Modify: `src/providers/hero_sms/countries.rs`
- Scan: `src/**/*.rs`

**Verification**
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --all-features`

## Phase 5: P2 Tests and Contract Coverage

### 5.1 Add provider contract test suite
- [ ] Вынести общий набор ожиданий для любого `Provider`:
  - get number returns valid `TaskId` and `FullNumber`
  - polling semantics: `Ok(None)` means continue
  - terminal provider errors map correctly
  - cancellation/finish semantics are respected
- [ ] Добавить reusable mock utilities и shared assertions.
- [ ] Прогнать оба провайдера через один набор тестов.

**Files**
- Add: `tests/provider_contract.rs`
- Add: `tests/support/mod.rs`
- Modify: `src/providers/hero_sms/provider.rs`
- Modify: `src/providers/sms_online/provider.rs`
- Modify if needed: `src/providers/traits.rs`

**Verification**
- `cargo test --all-features`

### 5.2 Improve docs/doctest fidelity
- [ ] Убрать старые упоминания `SmsActivate*`, `isocountry`, неполные списки провайдеров.
- [ ] Перевести ключевые примеры с `ignore` на компилируемые doctests там, где возможно.
- [ ] Оставить `ignore` только там, где реально нужны API keys или network calls.

**Files**
- Modify: `src/lib.rs`
- Modify: `src/service/structure.rs` or replacement modules
- Modify: `src/providers/traits.rs`
- Modify: `src/providers/hero_sms/mod.rs`
- Modify: `src/providers/sms_online/mod.rs`
- Modify: `README.md`

**Verification**
- `cargo test --doc --all-features`

## Phase 6: P2 Documentation and Contributor Experience

### 6.1 Add human-facing architecture docs
- [ ] Написать `docs/architecture.md`:
  - layering
  - error ownership
  - retry policy
  - lifecycle model
  - extension points
- [ ] Написать `docs/adding-provider.md`:
  - minimum files
  - required invariants
  - testing checklist
  - error classification rules

**Files**
- Add: `docs/architecture.md`
- Add: `docs/adding-provider.md`

### 6.2 Update README to match project standards
- [ ] Добавить badges:
  - CI status
  - crate version
  - docs.rs
- [ ] Обновить quick start под актуальный API.
- [ ] Добавить короткий architecture section и extension story.
- [ ] Добавить раздел migration/deprecation, если будут новые API рядом со старыми.

**Files**
- Modify: `README.md`
- Inspect: `.github/workflows/*`

**Verification**
- `cargo doc --all-features --no-deps`

## Add / Remove / Change Summary

### Add
- [ ] `try_new` / `try_build`
- [ ] `ActivationHandle` / `SmsActivation`
- [ ] optional capability trait (`ProviderCapabilities`/`ProviderCatalog`)
- [ ] internal `providers/common/*`
- [ ] provider contract-test suite
- [ ] `docs/architecture.md`
- [ ] `docs/adding-provider.md`

### Remove
- [ ] provider-level `SolutionTimeout` variants
- [ ] `once_cell` dependency
- [ ] stale docs/examples with `SmsActivate*` and `isocountry`
- [ ] misleading unconditional capability answers where API does not guarantee them
- [ ] oversized single-file `src/service/structure.rs` as primary implementation bucket

### Change
- [ ] validated service construction path
- [ ] service module boundaries
- [ ] provider trait boundaries
- [ ] logging/redaction policy
- [ ] README and public docs
- [ ] internal provider implementation structure

## Suggested Execution Order
1. Phase 1.1, 1.2, 1.3
2. Phase 2.1
3. Phase 3.1, 3.2
4. Phase 2.2
5. Phase 4.1, 4.2
6. Phase 5.1, 5.2
7. Phase 6.1, 6.2

## Review Notes
- План intentionally staged so that public contract hardening happens before deep refactors.
- Capability cleanup should happen before building richer runtime discovery APIs.
- Internal deduplication should happen after trait boundaries stabilize; otherwise common code will calcify the wrong abstractions.

## Results
- Plan drafted; implementation not started.
