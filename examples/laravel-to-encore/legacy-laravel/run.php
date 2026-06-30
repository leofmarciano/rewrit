<?php

declare(strict_types=1);

require __DIR__ . '/../../../sdks/php/src/Rewrit.php';
require __DIR__ . '/../../../sdks/php/src/Laravel.php';

use Rewrit\Laravel;
use Rewrit\Rewrit;

final class HeaderBag
{
    public function __construct(private array $headers)
    {
    }

    public function all(): array
    {
        return $this->headers;
    }
}

final class FakeLaravelResponse
{
    public HeaderBag $headers;

    public function __construct(
        private int $status,
        array $headers,
        private string $content,
    ) {
        $this->headers = new HeaderBag($headers);
    }

    public function getStatusCode(): int
    {
        return $this->status;
    }

    public function getContent(): string
    {
        return $this->content;
    }
}

$command = getenv('REWRIT_ADAPTER_COMMAND') ?: 'run';

if ($command === 'doctor') {
    Rewrit::emit([
        'schema_version' => 'rewrit.event.v1',
        'kind' => 'doctor_report',
        'runtime_id' => Rewrit::runtimeId(),
        'report' => ['ok' => true, 'checks' => ['php' => PHP_VERSION]],
    ]);
    exit(0);
}

Rewrit::case('billing.invoice.create.success', 'billing', 'creates invoice');

if ($command === 'discover') {
    exit(0);
}

$response = new FakeLaravelResponse(
    201,
    [
        'content-type' => ['application/json'],
        'x-request-id' => ['legacy-request-id'],
    ],
    json_encode([
        'id' => 'inv_123',
        'amount' => '199.90',
        'currency' => 'BRL',
        'status' => 'open',
    ], JSON_THROW_ON_ERROR),
);

Laravel::observeHttpResponse(
    $response,
    null,
    [
        Laravel::dbDelta('invoices', [[
            'id' => 'inv_123',
            'customer_id' => 'cus_123',
            'amount' => '199.90',
            'currency' => 'BRL',
            'status' => 'open',
        ]]),
    ],
);
