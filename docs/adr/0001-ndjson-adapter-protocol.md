# ADR 0001: NDJSON Adapter Protocol

Status: accepted

## Context

Rewrit must compare behavior across runtimes that may live in different
languages, frameworks, package managers, and test runners. A Laravel/Pest
reference, a Django/Pytest reference, a Node/Vitest candidate, and a Rust Cargo
test candidate all need a shared boundary with the Rust engine.

The boundary must support:

- simple scripts,
- existing test runners,
- large monoliths,
- streaming output,
- easy debugging,
- no FFI requirement,
- no permanent adapter service.

## Decision

Adapters communicate with the engine using newline-delimited JSON events.

The default command adapter reads events from stdout. Runtimes that write human
runner output to stdout can use file transport through `REWRIT_EVENTS_PATH`.
Adapters that need structured input can read an adapter request from
`REWRIT_REQUEST_PATH`.

## Consequences

Positive:

- Every supported language can emit the protocol.
- Events can stream as cases run.
- Failed protocol output is easy to inspect by hand.
- SDKs can stay thin.
- The Rust core remains framework-agnostic.

Tradeoffs:

- Adapters must avoid mixing human output with protocol stdout.
- The protocol needs schema versioning and compatibility discipline.
- Rich bidirectional adapter control is intentionally limited in v1.

## Alternatives Considered

- FFI bindings: rejected because they would make each language integration more
  complex and harder to distribute.
- Long-running adapter servers: rejected for the MVP because they add lifecycle
  and port-management complexity.
- Parsing test runner output: rejected because human output is unstable and
  framework-specific.
