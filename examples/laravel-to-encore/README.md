# Laravel To Encore Fixture

This fixture is intentionally dependency-light: the reference side is a small
PHP script shaped like a Laravel/Pest emitter, and the candidate side is a Node
script shaped like an Encore/TypeScript emitter.

Run from the repository root:

```bash
cargo run -p rewrit-cli -- run --manifest examples/laravel-to-encore/rewrit.toml
```

It exercises:

- stable `case_id` discovery
- canonical HTTP response observations
- ignored request-id header noise
- DB delta comparison with a candidate schema map
- terminal, JSON, and JUnit reports under `examples/laravel-to-encore/.rewrit/reports`
