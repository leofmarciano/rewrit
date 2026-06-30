# PHP Pest Adapter

Use the Pest integration when a PHP reference runtime already has Pest tests or
Laravel feature tests that can emit Rewrit observations.

The PHP SDK provides:

- `Rewrit\rewrit($caseId, $suiteId = null, $title = null)`,
- `Rewrit\Rewrit::observe(...)`,
- `Rewrit\Rewrit::observeCanonical(...)`,
- `Rewrit\Laravel::observeHttpResponse(...)`,
- `Rewrit\Laravel::observeDbDelta(...)`.

## Manifest

```toml
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
```

File output avoids mixing Pest's reporter output with protocol events.

## Basic Pest Usage

```php
<?php

use Rewrit\Rewrit;
use function Rewrit\rewrit;

it('creates an invoice', function () {
    rewrit('billing.invoice.create.success', 'billing');

    Rewrit::observe([
        'id' => 'inv_123',
        'amount' => '199.90',
        'currency' => 'BRL',
        'status' => 'open',
    ]);
});
```

The Composer autoload file also registers `Rewrit\PestPlugin` when Pest is
available, so tests can call the marker from the Pest test context:

```php
it('creates an invoice', function () {
    $this->rewrit('billing.invoice.create.success', 'billing');

    Rewrit::observe(['status' => 'open']);
});
```

## Laravel Feature Test Usage

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
            'customer_id' => 'cus_123',
            'amount' => '199.90',
            'currency' => 'BRL',
            'status' => 'open',
        ]]),
    ]);

    $response->assertCreated();
});
```

`Laravel::observeHttpResponse(...)` emits canonical `status`, lowercase
`headers`, and parsed JSON body when available.

## Notes

- Keep Laravel assertions in Pest; Rewrit observes behavior for comparison.
- Use decimal strings for money.
- Use DB deltas when persistence is part of the contract.
- Use waivers for temporary candidate gaps, not broad normalizers.
