<?php

declare(strict_types=1);

require __DIR__ . '/../../../sdks/php/src/Rewrit.php';

use Rewrit\Rewrit;

$command = getenv('REWRIT_ADAPTER_COMMAND') ?: 'run';
$caseId = 'catalog.product.price.success';

if ($command === 'doctor') {
    Rewrit::emit([
        'schema_version' => 'rewrit.event.v1',
        'kind' => 'doctor_report',
        'runtime_id' => Rewrit::runtimeId(),
        'report' => ['ok' => true, 'checks' => ['php' => PHP_VERSION]],
    ]);
    exit(0);
}

Rewrit::case($caseId, 'catalog', 'calculates product price');

if ($command === 'discover') {
    exit(0);
}

Rewrit::observe([
    'sku' => 'sku_123',
    'unit_price' => '49.95',
    'quantity' => 2,
    'total' => '99.90',
    'currency' => 'BRL',
]);
