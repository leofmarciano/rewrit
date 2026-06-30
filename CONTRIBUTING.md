# Contributing

Rewrit uses a Rust Cargo workspace. Keep framework-specific code in adapter or
SDK crates; the core crates must remain framework-agnostic.

Before opening a pull request:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo doc --workspace --all-features --no-deps
```

Public protocol, report and contract changes require a schema update and an ADR
when the decision changes architecture.

