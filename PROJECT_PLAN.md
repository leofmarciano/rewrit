# Rewrit Project Plan

This file is the implementation checklist for the full Rewrit architecture. The

README explains the product; this file tracks the build scope until every item

from the original architecture plan is implemented and tested.

## Architecture Thesis

Rewrit is a Rust parity engine that compares canonical observations, not

framework internals. The core must stay hexagonal and must not know Laravel,

Pest, Vitest, Django, Encore, Pytest, Cargo test, PHP, Node, Python, or Rust

runtime details.

Pipeline:

```txt
runtime/framework specific code
  -> adapter
  -> Rewrit Protocol
  -> Rust engine
  -> normalization
  -> schema validation
  -> comparison
  -> policy and waiver application
  -> reports
  -> CI/agent exit code
```

## Implementation Checklist

### 1. Canonical Domain Model

- [x] Use `reference` and `candidate`, never `legacy`/`new`, in internal APIs.
- [x] Define stable IDs for cases, suites, runtimes, and adapters.
- [x] Implement `Case`, `Contract`, `Observation`, `CanonicalValue`,

  `CanonicalError`, `Effect`, `Divergence`, and `Report`.
- [x] Preserve `Null` vs `Absent`.
- [x] Model money as decimal/string capable values; do not treat float money as

  equivalent by default.
- [x] Use `#[non_exhaustive]` on public enums that must evolve safely.

### 2. Protocol

- [x] Define versioned NDJSON adapter events.
- [x] Define adapter requests for `doctor`, `discover`, and `run`.
- [x] Reject unsupported or missing protocol versions.
- [ ] Add file-based NDJSON input/output mode in addition to stdout.
- [ ] Export protocol schemas in release artifacts.

### 3. Core

- [x] Implement pure normalizer/comparator/policy/waiver boundaries.
- [x] Implement path, regex/uuid, timestamp, ordering, HTTP header, and PHP

  array normalizer hooks.
- [x] Classify JSON/canonical divergences by path with `type_mismatch` where

  type differs.
- [x] Honor configured JSON `ignore_paths`.
- [x] Honor ignored HTTP headers.
- [x] Keep waivers visible and make expired waivers blocking.
- [x] Implement path-scoped normalizer application for configured paths.
- [x] Implement JSON unordered array paths.
- [x] Implement DB delta field/table mapping in comparison.
- [x] Implement queue/event/file/cache/email/log comparators.
- [x] Add property tests for idempotent normalization and `compare(a, a)`.

### 4. Engine

- [x] Load and validate `rewrit.toml`.
- [x] Discover cases from adapter events.
- [x] Execute command adapters.
- [x] Execute HTTP contract adapters with start/healthcheck.
- [x] Capture and verify baselines.
- [x] Audit missing/orphan case IDs.
- [x] Apply timeouts and output truncation.
- [x] Redact configured secret patterns.
- [x] Write configured reports.
- [x] Map failures to documented exit codes.
- [x] Persist timestamped baselines beside `current.jsonl`.
- [ ] Add global cancellation/timeout handling.
- [x] Add lock files to prevent concurrent writes to the same store.
- [x] Validate contract expectations against observations and report

  `schema_mismatch`.

### 5. Reports

- [x] Terminal report.
- [x] JSON report.
- [x] NDJSON report.
- [x] JUnit XML report.
- [x] SARIF report.
- [x] HTML report.
- [x] Markdown report.
- [ ] Add richer minimal reproduction commands per divergence.
- [ ] Add suite rollups and worst-suite sorting from real suite metadata.
- [x] Add report snapshots.

### 6. CLI

- [x] `rewrit init`
- [x] `rewrit doctor`
- [x] `rewrit discover`
- [x] `rewrit capture`
- [x] `rewrit verify`
- [x] `rewrit run`
- [x] `rewrit audit`
- [x] `rewrit explain`
- [x] `rewrit schema export`
- [x] `rewrit report open`
- [ ] Add stricter CLI UX around unsupported modes/features.
- [ ] Add shell completions/manpage generation.

### 7. Adapters And SDKs

- [x] Command adapter boundary.
- [x] HTTP adapter boundary and MVP execution path.
- [x] PHP SDK package skeleton.
- [x] Node SDK package skeleton.
- [x] Python SDK package skeleton.
- [x] Rust SDK package skeleton.
- [ ] Pest plugin with `rewrit(case_id)`.
- [ ] PHPUnit extension.
- [ ] Laravel helpers for HTTP response and DB deltas.
- [ ] Vitest reporter and `test.rewrit`.
- [ ] Jest reporter.
- [ ] Encore helper(you will need to search about [encore.dev](http://encore.dev) ! its a runtime dont have .env and some props).
- [ ] Pytest plugin collection hooks.
- [ ] Django helpers.
- [ ] Rust cargo-test adapter and explicit helper.
- [ ] Rust `#[rewrit::case]` macro.

### 8. Operation Modes

- [x] Mirror mode.
- [x] Baseline capture/verify mode.
- [x] Contract mode for HTTP contracts.
- [x] Audit mode for missing/orphan cases.
- [ ] Contract mode for non-HTTP commands/jobs/functions.

### 9. Examples And Showcases

- [x] `examples/command-to-command`.
- [x] `examples/http-to-http`.
- [x] `examples/laravel-to-encore` manifest.
- [x] `examples/django-to-rust` manifest.
- [x] `examples/php-to-node-monolith` manifest.
- [ ] Executable Laravel-to-Encore fixture.
- [ ] Executable Django-to-Rust fixture.
- [ ] Executable PHP-to-Node fixture.

### 10. Security And Isolation

- [x] Trusted-repo execution model documented.
- [x] Explicit cwd per runtime.
- [x] Timeouts.
- [x] Process kill-on-drop.
- [x] Env redaction patterns for captured output.
- [x] Stdout/stderr byte limits.
- [x] Env allowlist enforcement.
- [x] Network control configuration.
- [x] Temp dir isolation per run.
- [ ] Optional Docker/Podman sandbox after MVP.

### 11. Quality Gates

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- [x] `cargo test --workspace --all-features`
- [x] `cargo doc --workspace --all-features --no-deps`
- [x] Command adapter integration test.
- [x] HTTP adapter integration test.
- [x] Snapshot tests for report formats.
- [x] Property tests for core invariants.
- [ ] E2E fixtures for PHP/Node/Python/Rust SDKs.

## Milestones

### MVP 1: Protocol And Minimal Engine

Acceptance:

- [x] Two scripts emit observations.
- [x] Rewrit compares by `case_id`.
- [x] Detects pass, mismatch, missing case, and timeout/infrastructure paths.
- [x] Generates JSON and terminal reports.
- [x] Returns correct exit codes for parity/config/divergence.

### MVP 2: HTTP Adapter

Acceptance:

- [x] Start/stop HTTP servers.
- [x] Healthcheck before requests.
- [x] Requests declared in contracts.
- [x] Compare status/headers/body observations.
- [x] Detect type mismatch in body JSON.
- [x] Validate JSON Schema expectations and classify `schema_mismatch`.
- [x] Detect status mismatch from contract expectation even if reference and

  candidate agree on the wrong status.

### MVP 3: Laravel To Encore

Acceptance:

- [ ] `rewrit init --template laravel-to-encore` creates a working project

  template with SDK instructions.
- [ ] PHP SDK emits Pest/PHPUnit/Laravel observations.
- [ ] Node SDK emits Vitest/Jest/Encore observations.
- [ ] Baseline generated from Laravel example.
- [ ] Encore/TS candidate emits equivalent observations.
- [ ] Missing Encore test is reported as `missing_candidate_case`.
- [ ] Payload incompatibility is reported with path and hint.
- [ ] JUnit and JSON reports are generated.

### MVP 4: Django To Rust

Acceptance:

- [ ] Pytest plugin emits observations.
- [ ] Django helper emits HTTP observations.
- [ ] Rust SDK emits observations from tests.
- [ ] Cargo test adapter runs candidate.
- [ ] HTTP-first migration guide is backed by executable fixtures.

## Anti-Patterns To Keep Out

- Core knowing Laravel, Django, Vitest, Pest, Encore, Pytest, PHP, Node, Python,

  or Rust-specific test behavior.
- Parsing human stdout as the contract.
- Raw string comparison as the default behavioral model.
- Broad normalizers that hide real bugs.
- Retries hiding flakiness.
- Eternal waivers.
- One giant adapter for all languages.
- Universal AST parser ambitions.
- AI inside the library.
- Treating every failure as “test failed.”
- Mixing infra errors with semantic divergences.
- Unversioned protocols or schemas.
- Collapsing `null`, missing, and runtime-specific undefined into one value.
- Treating money as float by default.
