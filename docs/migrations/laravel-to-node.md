# Laravel to Node

Use this guide when Laravel/PHP is the reference and a Node service, worker, or
extracted module is the candidate.

This migration often starts as a command-to-command comparison before the Node
side has a full HTTP surface. That is fine: Rewrit only needs both sides to emit
canonical observations for the same case IDs.

The closest runnable fixture is
[`examples/php-to-node-monolith`](../../examples/php-to-node-monolith).

## Migration Shape

```txt
Laravel/PHP behavior
  -> PHP Rewrit observation
  -> stable case_id
  -> Node observation
  -> Rewrit report
```

Good first boundaries:

- pure domain functions,
- pricing calculations,
- validation decisions,
- command handlers,
- queue jobs,
- internal HTTP endpoints.

## Reference Side: PHP

```php
<?php

use Rewrit\Rewrit;
use function Rewrit\rewrit;

rewrit('catalog.product.price.success', 'catalog', 'calculates product price');

Rewrit::observe([
    'sku' => 'sku_123',
    'unit_price' => '49.95',
    'quantity' => 2,
    'total' => '99.90',
    'currency' => 'BRL',
]);
```

In a real Laravel app, this can live inside a Pest/PHPUnit test that calls the
existing service, action, command, or route.

## Candidate Side: Node

```ts
import { caseDiscovered, observe } from "@rewrit/node";

caseDiscovered(
  "catalog.product.price.success",
  "catalog",
  "calculates product price",
);

observe({
  sku: "sku_123",
  unit_price: "49.95",
  quantity: 2,
  total: "99.90",
  currency: "BRL",
});
```

For Vitest or Jest suites, prefer the runner-specific helpers:

- [Node Vitest adapter](../adapters/node-vitest.md)
- [Node Jest adapter](../adapters/node-jest.md)

## Manifest

```toml
[project]
name = "laravel-to-node"
reference = "reference_php"
candidate = "candidate_node"
contracts_dir = "contracts"
baselines_dir = ".rewrit/baselines"
reports_dir = ".rewrit/reports"

[runtimes.reference_php]
adapter = "command"
cwd = "../legacy"
command = ["php", "rewrit-reference.php"]
timeout_ms = 30000

[runtimes.reference_php.protocol]
output = "file"

[runtimes.candidate_node]
adapter = "command"
cwd = "../candidate"
command = ["node", "rewrit-candidate.mjs"]
timeout_ms = 30000

[runtimes.candidate_node.protocol]
output = "file"

[[reports]]
kind = "terminal"

[[reports]]
kind = "json"
path = ".rewrit/reports/latest.json"
```

Use file output because both PHP and Node runners often write diagnostic text to
stdout.

## Workflow

1. Choose a small domain boundary with deterministic inputs.
2. Give it a stable case ID.
3. Emit a PHP reference observation from the existing behavior.
4. Emit a Node candidate observation with the same case ID.
5. Run mirror mode:

   ```bash
   rewrit run --mode mirror
   ```

6. Add contracts for cases that should become long-lived compatibility
   guarantees.
7. Add normalizers only for scoped noise such as IDs and timestamps.
8. Add waivers only for known, temporary candidate gaps.

## Fixture

Run the repository fixture:

```bash
cargo run -p rewrit-cli -- run --manifest examples/php-to-node-monolith/rewrit.toml
```

Expected result:

```txt
Blocking divergences: 0
Parity: 100.00%
Exit: 0
```

## Common Pitfalls

- Do not let JavaScript numbers replace money strings.
- Do not compare raw stdout as the domain contract unless stdout is the product.
- Do not use generated test names as case IDs.
- Keep fixture data deterministic.
- Add side effects after return-value parity is stable.
