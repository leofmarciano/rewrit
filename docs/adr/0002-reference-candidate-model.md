# ADR 0002: Reference and Candidate Model

Status: accepted

## Context

Rewrite tools often call the two sides `legacy` and `new`. That vocabulary is
too narrow for Rewrit.

The same comparison model should work for:

- legacy-to-new migrations,
- framework swaps,
- service extractions,
- modern-to-modern comparisons,
- refactors inside one stack,
- baseline verification in CI.

## Decision

Rewrit uses:

- `reference`: the implementation treated as the source of truth.
- `candidate`: the implementation being validated.

These terms appear in manifests, reports, examples, and docs.

## Consequences

Positive:

- The model applies beyond legacy rewrites.
- Reports are clearer in CI: the candidate is what must change.
- Baseline workflows stay natural: capture reference, verify candidate.

Tradeoffs:

- Some migration guides still need to explain which side is the legacy app.
- Users may initially expect `legacy`/`new` terminology from migration tooling.

## Examples

```txt
reference = Laravel/PHP
candidate = Encore/TypeScript

reference = Django/Python
candidate = Rust

reference = current service
candidate = refactored service
```
