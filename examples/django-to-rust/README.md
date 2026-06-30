# Django To Rust Fixture

This fixture compares a dependency-light Python reference shaped like a Django
test boundary with a real Rust `cargo test` candidate using the Rewrit Rust SDK.

Run from the repository root:

```bash
cargo run -p rewrit-cli -- run --manifest examples/django-to-rust/rewrit.toml
cargo run -p rewrit-cli -- verify --manifest examples/django-to-rust/rewrit.toml --contracts 'contracts/**/*.json'
```

The Rust runtime writes protocol events through `REWRIT_EVENTS_PATH` because
`cargo test` prints harness output to stdout.
