# Rewrit

[![CI](https://github.com/leofmarciano/rewrit/actions/workflows/ci.yml/badge.svg)](https://github.com/leofmarciano/rewrit/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/rewrit-core.svg)](https://crates.io/crates/rewrit-core)
[![npm](https://img.shields.io/npm/v/%40rewrit%2Fnode.svg)](https://www.npmjs.com/package/@rewrit/node)
[![Packagist](https://img.shields.io/packagist/v/rewrit/rewrit.svg)](https://packagist.org/packages/rewrit/rewrit)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)
[![Rust 1.80+](https://img.shields.io/badge/rust-1.80%2B-orange.svg)](Cargo.toml)

Rewrit is a parity engine for software rewrites.

It compares the observable behavior of a **reference** implementation and a
**candidate** implementation so teams can move code across languages,
frameworks, services, and architectures without silently changing behavior.

Use Rewrit when "the tests pass" is not enough. It checks the values, errors,
HTTP responses, side effects, contracts, and policy decisions that matter during
a migration.

```txt
reference runtime          candidate runtime
       |                         |
       v                         v
    adapter                   adapter
       |                         |
       +------ observations -----+
                  |
                  v
        normalize, compare, report
                  |
                  v
          CI-friendly exit code
```

## Why Rewrit?

Large rewrites usually fail in the gaps between test suites:

- a Laravel endpoint becomes an Encore service,
- a Django path moves to Rust,
- a PHP monolith function is extracted to Node,
- an HTTP API keeps the same shape but changes internals,
- a "harmless" refactor changes an error, header, decimal, queue message, or
  database side effect.

Rewrit gives those gaps names and evidence. Instead of flattening everything
into "test failed", it reports structured divergences such as
`type_mismatch`, `schema_mismatch`, `missing_candidate_case`,
`side_effect_mismatch`, `timeout`, and `waiver_expired`.

## Current Status

Rewrit is pre-1.0. The MVP workspace is implemented and covered by tests, but
the public protocol and SDK ergonomics may still change before a stable 1.0
release.

Today the repository includes:

- Rust CLI and engine,
- versioned NDJSON adapter protocol,
- JSON contract and report schemas,
- command and HTTP adapters,
- PHP, Node, Python, and Rust SDK surfaces,
- terminal, JSON, NDJSON, JUnit, SARIF, HTML, and Markdown report code,
- runnable migration examples.

## Packages

Rewrit packages are published across the registries used by each supported
ecosystem. The CLI is not published as a standalone crates.io package yet; use
the source build in the quickstart below.

| Ecosystem | Package | Install |
| --- | --- | --- |
| Rust crates on crates.io | [`rewrit-core`](https://crates.io/crates/rewrit-core), [`rewrit-model`](https://crates.io/crates/rewrit-model), [`rewrit-protocol`](https://crates.io/crates/rewrit-protocol), [`rewrit-report`](https://crates.io/crates/rewrit-report), [`rewrit-macros`](https://crates.io/crates/rewrit-macros) | Example: `cargo add rewrit-core` |
| Node SDK on npm | [`@rewrit/node`](https://www.npmjs.com/package/@rewrit/node) | `npm install @rewrit/node` |
| PHP SDK on Packagist | [`rewrit/rewrit`](https://packagist.org/packages/rewrit/rewrit) | `composer require rewrit/rewrit` |

## Quickstart

Install Rust 1.80 or newer, then build the CLI from source:

```bash
git clone https://github.com/leofmarciano/rewrit.git
cd rewrit
cargo build -p rewrit-cli --release
./target/release/rewrit --help
```

Run a passing Laravel-to-Encore shaped fixture:

```bash
cargo run -p rewrit-cli -- run --manifest examples/laravel-to-encore/rewrit.toml
```

Expected result:

```txt
Equivalent: 1
Blocking divergences: 0
Parity: 100.00%
Exit: 0
```

Run an intentionally failing command-to-command fixture:

```bash
cargo run -p rewrit-cli -- run --manifest examples/command-to-command/rewrit.toml
```

That fixture exits with `1` on purpose. It demonstrates how Rewrit reports a
candidate returning money as a number instead of a decimal string, plus a
missing candidate case.

## Create a Project

Generate a runnable scaffold:

```bash
cargo build -p rewrit-cli
export PATH="$PWD/target/debug:$PATH"

rm -rf /tmp/rewrit-demo
mkdir /tmp/rewrit-demo
cd /tmp/rewrit-demo

rewrit init --template laravel-to-encore
rewrit run --mode mirror
rewrit capture --runtime legacy_laravel
rewrit verify --runtime encore_ts
rewrit audit
```

Available templates:

- `command-to-command`
- `laravel-to-encore`
- `django-to-rust`

## Core Ideas

### Reference and Candidate

The reference is the implementation treated as the source of truth. The
candidate is the implementation being validated.

### Cases

A case is one stable behavior ID, such as
`billing.invoice.create.success` or `auth.login.invalid_password`.

### Contracts

Contracts declare what must stay equivalent: inputs, outputs, status codes,
schemas, headers, expected errors, side effects, tolerances, and policies.

### Observations

Adapters translate framework-specific execution into canonical Rewrit
observations. The Rust core does not need to understand Laravel, Django,
Vitest, Pest, Pytest, Cargo test, or any other test runner.

### Policies, Normalizers, and Waivers

Policies define what counts as a real difference. Normalizers remove accepted
noise such as generated IDs or timestamps. Waivers are explicit, owned, and
expiring exceptions that remain visible in reports.

## Example Manifest

Rewrit projects are configured with `rewrit.toml`.

```toml
[project]
name = "billing-migration"
reference = "legacy_laravel"
candidate = "encore_ts"
contracts_dir = "contracts"
baselines_dir = ".rewrit/baselines"
reports_dir = ".rewrit/reports"

[runtimes.legacy_laravel]
adapter = "command"
cwd = "../legacy"
command = ["vendor/bin/pest", "--rewrit"]
timeout_ms = 30000

[runtimes.encore_ts]
adapter = "command"
cwd = "../candidate"
command = ["npm", "run", "test:rewrit"]
timeout_ms = 30000

[[reports]]
kind = "terminal"

[[reports]]
kind = "json"
path = ".rewrit/reports/latest.json"
```

Adapters emit versioned NDJSON events:

```json
{"schema_version":"rewrit.event.v1","kind":"observation","case_id":"billing.invoice.create.success","runtime_id":"reference","status":"passed","value":{"kind":"json","value":{"status":"open","amount":"199.90"}}}
```

## CLI

```bash
rewrit init --template laravel-to-encore
rewrit doctor
rewrit discover
rewrit run --mode mirror
rewrit capture --runtime legacy_laravel
rewrit verify --runtime encore_ts
rewrit audit
rewrit explain billing.invoice.create.success
rewrit schema export --kind all --out-dir dist/schemas
rewrit report open
rewrit completions --shell zsh
```

Common modes:

- `run --mode mirror`: run reference and candidate in the same execution.
- `capture --runtime <reference>`: store a baseline reference observation.
- `verify --runtime <candidate>`: compare a candidate against the stored
  baseline or declared contracts.
- `audit`: check for missing required cases.

## Examples

| Example | What it shows |
| --- | --- |
| [`examples/command-to-command`](examples/command-to-command) | Generic command adapter and intentional divergences |
| [`examples/http-to-http`](examples/http-to-http) | HTTP contract comparison and schema/type mismatches |
| [`examples/laravel-to-encore`](examples/laravel-to-encore) | PHP-shaped reference to Encore/Node-shaped candidate |
| [`examples/django-to-rust`](examples/django-to-rust) | Django/Python-shaped reference to Rust candidate |
| [`examples/php-to-node-monolith`](examples/php-to-node-monolith) | Monolith function extraction with stable case IDs |

## Documentation

Start with the [documentation index](docs/README.md).

Common entry points:

- New to Rewrit: [Parity](docs/concepts/parity.md),
  [contracts](docs/concepts/contracts.md), and
  [observations](docs/concepts/observations.md).
- Wiring a runner or framework: [Adapter protocol v1](docs/protocol/adapter-protocol-v1.md)
  and the adapter guide for your stack.
- Using an existing SDK: [PHP Pest](docs/adapters/php-pest.md),
  [PHPUnit](docs/adapters/php-phpunit.md),
  [Node Vitest](docs/adapters/node-vitest.md),
  [Node Jest](docs/adapters/node-jest.md),
  [Node Encore](docs/adapters/node-encore.md),
  [Python Pytest](docs/adapters/python-pytest.md),
  [Django](docs/adapters/django.md), or
  [Rust Cargo test](docs/adapters/rust-cargo-test.md).
- Planning a migration: [Laravel to Encore](docs/migrations/laravel-to-encore.md),
  [Laravel to Node](docs/migrations/laravel-to-node.md), or
  [Django to Rust](docs/migrations/django-to-rust.md).
- Reviewing design decisions: [ADRs](docs/adr).

## Repository Layout

```txt
crates/
  rewrit-model             canonical data model
  rewrit-core              normalization, comparison, policy, validation
  rewrit-engine            discovery, scheduling, runtime execution, baselines
  rewrit-protocol          adapter protocol types
  rewrit-report            terminal and machine-readable reports
  rewrit-cli               command-line interface
  rewrit-adapter-*         built-in adapter crates

sdks/
  php/
  node/
  python/
  rust/

docs/
  concepts/
  protocol/
  adapters/
  migrations/
  adr/

examples/
  command-to-command/
  http-to-http/
  laravel-to-encore/
  django-to-rust/
  php-to-node-monolith/
```

## Development

Run the full Rust gate before opening a pull request:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo doc --workspace --all-features --no-deps
```

Generate protocol and report schemas:

```bash
cargo run -p rewrit-cli -- schema export --kind all --out-dir dist/schemas
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for contribution guidelines.

## Security

Rewrit executes project code through adapters. Treat configured runtimes as
trusted project code unless you explicitly run them inside a sandbox.

Supported guardrails include timeouts, stdout/stderr limits, secret redaction,
temporary run directories, environment allowlists, and optional Docker or
Podman sandboxing.

See [SECURITY.md](SECURITY.md) for details.

## License

Rewrit is licensed under either of:

- [MIT](LICENSE-MIT)
- [Apache License 2.0](LICENSE-APACHE)
