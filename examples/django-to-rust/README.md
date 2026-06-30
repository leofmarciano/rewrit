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

What this fixture exercises:

- stable `case_id` binding across Django-shaped reference and Rust candidate
- command reference runtime with file protocol output
- `rust:cargo_test` candidate runtime
- Rust SDK `#[rewrit::case]` plus canonical HTTP-shaped observations
- JSON contract verification for the same case

The SDK-level pytest/Django helper behavior is covered by:

```bash
cargo test -p rewrit-engine --test sdk_observations python_sdk_emits_pytest_and_django_observations -- --nocapture
```
