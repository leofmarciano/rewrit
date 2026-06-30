# Rewrit Documentation

This directory contains the product and integration documentation for Rewrit.
The README at the repository root is the short introduction; these pages explain
how to model behavior, connect runtimes, and use the SDKs during a migration.

## Reading Path

If you are new to Rewrit, read these first:

1. [Parity](concepts/parity.md)
2. [Contracts](concepts/contracts.md)
3. [Observations](concepts/observations.md)
4. [Policies](concepts/policies.md)
5. [Side effects](concepts/side-effects.md)

If you are wiring a runtime, read:

1. [Adapter protocol v1](protocol/adapter-protocol-v1.md)
2. The adapter guide for your test runner or framework
3. The closest migration guide for your rewrite shape

If you are contributing to Rewrit internals, read:

1. [ADR 0001: NDJSON adapter protocol](adr/0001-ndjson-adapter-protocol.md)
2. [ADR 0002: Reference and candidate model](adr/0002-reference-candidate-model.md)
3. [ADR 0003: JSON Schema contracts](adr/0003-json-schema-contracts.md)
4. [ADR 0004: No language parser in core](adr/0004-no-language-parser-in-core.md)

## Concepts

| Page | Purpose |
| --- | --- |
| [Parity](concepts/parity.md) | Defines what Rewrit does and does not prove. |
| [Contracts](concepts/contracts.md) | Shows how to declare stable behavioral expectations. |
| [Observations](concepts/observations.md) | Explains the canonical runtime output that adapters emit. |
| [Policies](concepts/policies.md) | Describes strict defaults, normalizers, and waivers. |
| [Side effects](concepts/side-effects.md) | Covers database, queue, file, HTTP, event, email, cache, and log effects. |

## Protocol

| Page | Purpose |
| --- | --- |
| [Adapter protocol v1](protocol/adapter-protocol-v1.md) | Event stream and command/file transport contract for adapters. |
| [Observation schema v1](protocol/observation-schema-v1.md) | Shape of a canonical observation. |
| [Report schema v1](protocol/report-schema-v1.md) | Shape of machine-readable reports. |

## Adapter Guides

| Stack | Guide |
| --- | --- |
| Django | [adapters/django.md](adapters/django.md) |
| Node Encore | [adapters/node-encore.md](adapters/node-encore.md) |
| Node Jest | [adapters/node-jest.md](adapters/node-jest.md) |
| Node Vitest | [adapters/node-vitest.md](adapters/node-vitest.md) |
| PHP Pest | [adapters/php-pest.md](adapters/php-pest.md) |
| PHPUnit | [adapters/php-phpunit.md](adapters/php-phpunit.md) |
| Python Pytest | [adapters/python-pytest.md](adapters/python-pytest.md) |
| Rust Cargo test | [adapters/rust-cargo-test.md](adapters/rust-cargo-test.md) |

## Migration Guides

| Rewrite | Guide |
| --- | --- |
| Laravel to Encore | [migrations/laravel-to-encore.md](migrations/laravel-to-encore.md) |
| Laravel to Node | [migrations/laravel-to-node.md](migrations/laravel-to-node.md) |
| Django to Rust | [migrations/django-to-rust.md](migrations/django-to-rust.md) |

## Generated Schemas

Generate the current schemas from the Rust types:

```bash
cargo run -p rewrit-cli -- schema export --kind all --out-dir dist/schemas
```

The release workflow uploads those files as the `rewrit-protocol-schemas`
artifact.
