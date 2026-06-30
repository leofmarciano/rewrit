# PHP Pest Adapter

The Pest adapter exposes a `rewrit(case_id)` marker and emits observations
through the PHP SDK. The SDK writes Rewrit NDJSON to stdout by default, or to
`REWRIT_EVENTS_PATH` when the engine runs the command adapter with file
transport.

```php
<?php

use Rewrit\Rewrit;
use function Rewrit\rewrit;

it('creates an invoice', function () {
    rewrit('billing.invoice.create.success');

    $response = [
        'id' => 'inv_123',
        'amount' => '199.90',
        'currency' => 'BRL',
        'status' => 'open',
    ];

    Rewrit::observe($response);
});
```

When Pest is installed, the Composer autoload file registers
`Rewrit\PestPlugin` with `Pest\Plugin::uses(...)`, so tests can also call the
marker through the Pest test context:

```php
it('creates an invoice', function () {
    $this->rewrit('billing.invoice.create.success');

    Rewrit::observe(['status' => 'open']);
});
```
