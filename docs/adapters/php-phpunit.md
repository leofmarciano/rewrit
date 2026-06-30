# PHP PHPUnit Adapter

Use the PHPUnit integration when a PHP runtime uses PHPUnit directly rather
than Pest.

The SDK provides:

- `Rewrit\PHPUnitCase` trait,
- `rewrit(...)` case marker method,
- `observeRewrit(...)` observation helper,
- `Rewrit\Rewrit::observeCanonical(...)` for explicit canonical values.

## Manifest

```toml
[runtimes.reference_php]
adapter = "command"
cwd = "../reference"
command = ["vendor/bin/phpunit", "--colors=never"]
timeout_ms = 30000

[runtimes.reference_php.protocol]
output = "file"
```

## PHPUnit Configuration

Register the extension in `phpunit.xml` when you want a runner-level doctor
event:

```xml
<extensions>
  <bootstrap class="Rewrit\PHPUnitExtension" />
</extensions>
```

## Test Usage

```php
<?php

use PHPUnit\Framework\TestCase;
use Rewrit\PHPUnitCase;

final class InvoiceTest extends TestCase
{
    use PHPUnitCase;

    public function testCreatesInvoice(): void
    {
        $this->rewrit('billing.invoice.create.success', 'billing');

        $this->observeRewrit([
            'id' => 'inv_123',
            'amount' => '199.90',
            'currency' => 'BRL',
            'status' => 'open',
        ]);
    }
}
```

The SDK writes protocol events to stdout by default and appends to
`REWRIT_EVENTS_PATH` when file transport is configured.

## Notes

- Use the trait in tests that represent cross-runtime behavior.
- Keep PHPUnit assertions in the test; Rewrit compares observations across
  runtimes.
- Use explicit canonical observations when PHP type conversion would hide a
  migration-relevant difference.
