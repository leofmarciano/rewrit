# Laravel to Encore

Use this guide when Laravel/PHP is the reference implementation and Encore.ts is
the candidate implementation.

Start with stable HTTP or service-boundary cases. Add database deltas and other
side effects once the response shape is under control.

The runnable fixture is in [`examples/laravel-to-encore`](../../examples/laravel-to-encore).

## Migration Shape

```txt
Laravel feature test
  -> Rewrit PHP/Laravel observation
  -> stable case_id
  -> Encore/Vitest or Node observation
  -> Rewrit comparison report
```

The important invariant is that both sides emit observations for the same
domain case ID:

```txt
billing.invoice.create.success
```

## Reference Side: Laravel

Use Pest or PHPUnit to keep Laravel assertions and emit Rewrit observations:

```php
<?php

use Rewrit\Laravel;
use function Rewrit\rewrit;

it('creates an invoice', function () {
    rewrit('billing.invoice.create.success', 'billing');

    $response = $this->postJson('/api/invoices', [
        'customer_id' => 'cus_123',
        'amount' => '199.90',
        'currency' => 'BRL',
    ]);

    Laravel::observeHttpResponse($response, effects: [
        Laravel::dbDelta('invoices', inserted: [[
            'id' => 'inv_123',
            'customer_id' => 'cus_123',
            'amount' => '199.90',
            'currency' => 'BRL',
            'status' => 'open',
        ]]),
    ]);

    $response->assertCreated();
});
```

The Laravel helper emits a canonical HTTP value:

- status,
- lowercase headers,
- JSON body when available,
- optional side effects.

## Candidate Side: Encore

For service-level tests, emit the same case ID and service result:

```ts
import {
  encoreCase,
  observeDbDelta,
  observeServiceResult,
} from "@rewrit/node/encore";

test("creates invoice", async () => {
  encoreCase("billing.invoice.create.success", "billing");

  const result = await invoiceService.create({
    customer_id: "cus_123",
    amount: "199.90",
    currency: "BRL",
  });

  observeServiceResult(result);
  observeDbDelta("billing_invoices", {
    inserted: [{
      invoice_id: "inv_123",
      customer_ref: "cus_123",
      total_amount: "199.90",
      currency_code: "BRL",
      state: "open",
    }],
  });
});
```

For HTTP tests, use `observeHttpResponse(response)` from
`@rewrit/node/encore`.

## Manifest

```toml
[project]
name = "laravel-to-encore"
reference = "legacy_laravel"
candidate = "encore_ts"
contracts_dir = "contracts"
baselines_dir = ".rewrit/baselines"
reports_dir = ".rewrit/reports"

[runtimes.legacy_laravel]
adapter = "command"
cwd = "../legacy"
command = ["vendor/bin/pest", "--colors=never"]
timeout_ms = 30000

[runtimes.legacy_laravel.env]
APP_ENV = "testing"
CACHE_DRIVER = "array"
QUEUE_CONNECTION = "sync"

[runtimes.legacy_laravel.protocol]
output = "file"

[runtimes.encore_ts]
adapter = "command"
cwd = "../candidate"
command = ["npm", "run", "test:rewrit"]
timeout_ms = 30000

[runtimes.encore_ts.env]
NODE_ENV = "test"

[runtimes.encore_ts.protocol]
output = "file"

[[suites]]
id = "billing"
title = "Billing domain"
policy = "http_api_strict"
required = true

[policies.http_api_strict]
compare_exit_code = true
decimal_as_string = true

[policies.http_api_strict.headers]
ignore = ["date", "x-request-id", "server"]

[effects.db.maps.invoices]
target_table = "billing_invoices"

[effects.db.maps.invoices.fields]
id = "invoice_id"
customer_id = "customer_ref"
amount = "total_amount"
currency = "currency_code"
status = "state"
```

## Workflow

1. Add Rewrit case IDs to the Laravel tests that define the migration boundary.
2. Emit Laravel HTTP observations and only the side effects that matter.
3. Add matching Encore tests with the same case IDs.
4. Run mirror mode while both sides are easy to execute:

   ```bash
   rewrit run --mode mirror
   ```

5. Capture a reference baseline when Laravel becomes expensive or slow to run:

   ```bash
   rewrit capture --runtime legacy_laravel
   rewrit verify --runtime encore_ts
   ```

6. Run audit mode to catch missing candidate cases:

   ```bash
   rewrit audit
   ```

## Fixture

Run the repository fixture from the repo root:

```bash
cargo run -p rewrit-cli -- run --manifest examples/laravel-to-encore/rewrit.toml
```

Expected result:

```txt
Blocking divergences: 0
Parity: 100.00%
Exit: 0
```

## Common Pitfalls

- Do not compare Laravel response text to Encore response text directly. Emit
  canonical HTTP observations.
- Do not normalize the whole response just to hide `x-request-id`; ignore or
  normalize the specific path/header.
- Keep money as strings or canonical decimals.
- Use DB maps when Encore uses a new schema.
- Use waivers for known incomplete candidate behavior with an owner and expiry.
