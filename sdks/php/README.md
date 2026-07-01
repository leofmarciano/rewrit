# rewrit/rewrit

PHP SDK for emitting Rewrit observations from Pest, PHPUnit and Laravel tests.

```bash
composer require rewrit/rewrit
```

```php
<?php

use Rewrit\Rewrit;

Rewrit::case('billing.invoice.create.success', 'billing');
Rewrit::observe([
    'status' => 'open',
    'amount' => '199.90',
]);
```

The SDK emits Rewrit adapter protocol events to `REWRIT_EVENTS_PATH` when it is
set, or to stdout otherwise.

The Packagist package is published from the repository root because Packagist
expects `composer.json` at the top of the package repository.
