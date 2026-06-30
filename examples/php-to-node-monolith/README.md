# PHP To Node Monolith Fixture

This fixture compares a PHP reference function with a Node candidate function.
It models a monolith extraction where both sides emit canonical observations for
the same stable `case_id`.

Run from the repository root:

```bash
cargo run -p rewrit-cli -- run --manifest examples/php-to-node-monolith/rewrit.toml
cargo run -p rewrit-cli -- verify --manifest examples/php-to-node-monolith/rewrit.toml --contracts 'contracts/**/*.json'
```

The contract keeps money as decimal strings and will reject a Node candidate
that returns `99.9` as a number.
