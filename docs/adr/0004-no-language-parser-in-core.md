# ADR 0004: No Language Parser in Core

Status: accepted

## Context

Rewrit targets migrations across PHP, Node, Python, Rust, and other runtimes.
One possible design is to make the Rust core parse source code or framework
metadata from each stack.

That would make the core large, brittle, and tied to framework release cycles.

## Decision

The core does not parse application source code or framework internals.

Framework and runner knowledge lives in adapters and SDKs. The core receives
canonical observations, normalizes them, compares them, applies policies and
waivers, and writes reports.

## Consequences

Positive:

- The core stays small and testable.
- Adding a new framework does not require changing comparison logic.
- The protocol boundary is explicit and versioned.
- Users can write custom adapters with simple command scripts.

Tradeoffs:

- Adapters are responsible for accurate observation capture.
- Rewrit cannot infer missing behavior unless cases, contracts, or discovery
  events expose it.
- Some convenience requires SDK work per ecosystem.

## Design Rule

Do not teach the core about Laravel, Django, Vitest, Pest, Pytest, Cargo test,
or Encore. Teach adapters to emit better observations.
