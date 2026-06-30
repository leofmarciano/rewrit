# PHP PHPUnit Adapter

The PHPUnit adapter exposes an extension plus a trait for marking tests with
stable Rewrit case IDs.

Register the extension in `phpunit.xml`:

```xml
<extensions>
  <bootstrap class="Rewrit\PHPUnitExtension" />
</extensions>
```

Use `Rewrit\PHPUnitCase` in test classes:

```php
<?php

use PHPUnit\Framework\TestCase;
use Rewrit\PHPUnitCase;

final class InvoiceTest extends TestCase
{
    use PHPUnitCase;

    public function testCreatesInvoice(): void
    {
        $this->rewrit('billing.invoice.create.success');

        $this->observeRewrit([
            'id' => 'inv_123',
            'amount' => '199.90',
            'currency' => 'BRL',
            'status' => 'open',
        ]);
    }
}
```

The SDK writes NDJSON to stdout by default and appends to `REWRIT_EVENTS_PATH`
when the command adapter is configured with file transport.
