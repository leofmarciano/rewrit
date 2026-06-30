<?php

declare(strict_types=1);

namespace Rewrit;

final class Laravel
{
    public static function observeHttpResponse(mixed $response, ?string $caseId = null, array $effects = []): void
    {
        $status = self::statusCode($response);
        Rewrit::observeCanonical(
            [
                'kind' => 'object',
                'fields' => [
                    'status' => ['kind' => 'integer', 'value' => (string) $status],
                    'headers' => [
                        'kind' => 'object',
                        'fields' => self::canonicalHeaders(self::headers($response)),
                    ],
                    'body' => self::body($response),
                ],
            ],
            $caseId,
            $status < 500 ? 'passed' : 'failed',
            $effects,
        );
    }

    public static function observeDbDelta(
        string $table,
        array $inserted = [],
        array $updated = [],
        array $deleted = [],
        string $connection = 'default',
        ?string $caseId = null,
    ): void {
        Rewrit::addEffect(self::dbDelta($table, $inserted, $updated, $deleted, $connection), $caseId);
    }

    public static function dbDelta(
        string $table,
        array $inserted = [],
        array $updated = [],
        array $deleted = [],
        string $connection = 'default',
    ): array {
        return [
            'kind' => 'db_delta',
            'connection' => $connection,
            'table' => $table,
            'inserted' => array_map(self::canonicalRow(...), $inserted),
            'updated' => array_map(self::canonicalRow(...), $updated),
            'deleted' => array_map(self::canonicalRow(...), $deleted),
        ];
    }

    private static function statusCode(mixed $response): int
    {
        if (is_object($response) && method_exists($response, 'getStatusCode')) {
            return (int) $response->getStatusCode();
        }
        if (is_object($response) && method_exists($response, 'status')) {
            return (int) $response->status();
        }

        return 0;
    }

    private static function headers(mixed $response): array
    {
        $headers = null;
        if (is_object($response) && isset($response->headers)) {
            $headers = $response->headers;
        } elseif (is_object($response) && isset($response->baseResponse->headers)) {
            $headers = $response->baseResponse->headers;
        }

        if (is_object($headers) && method_exists($headers, 'all')) {
            return $headers->all();
        }

        return [];
    }

    private static function canonicalHeaders(array $headers): array
    {
        $canonical = [];
        foreach ($headers as $name => $values) {
            $value = is_array($values) ? implode(', ', array_map('strval', $values)) : (string) $values;
            $canonical[strtolower((string) $name)] = ['kind' => 'string', 'value' => $value];
        }

        return $canonical;
    }

    private static function body(mixed $response): array
    {
        if (is_object($response) && method_exists($response, 'json')) {
            try {
                $json = $response->json();
                if ($json !== null) {
                    return ['kind' => 'json', 'value' => $json];
                }
            } catch (\Throwable) {
            }
        }

        $content = '';
        if (is_object($response) && method_exists($response, 'getContent')) {
            $content = (string) $response->getContent();
        } elseif (is_object($response) && method_exists($response, 'content')) {
            $content = (string) $response->content();
        }

        $decoded = json_decode($content, true);
        if (json_last_error() === JSON_ERROR_NONE) {
            return ['kind' => 'json', 'value' => $decoded];
        }

        return ['kind' => 'string', 'value' => $content];
    }

    private static function canonicalRow(array $row): array
    {
        $canonical = [];
        foreach ($row as $field => $value) {
            $canonical[(string) $field] = self::canonicalValue($value);
        }

        return $canonical;
    }

    private static function canonicalValue(mixed $value): array
    {
        if ($value === null) {
            return ['kind' => 'null'];
        }
        if (is_bool($value)) {
            return ['kind' => 'bool', 'value' => $value];
        }
        if (is_int($value)) {
            return ['kind' => 'integer', 'value' => (string) $value];
        }
        if (is_float($value)) {
            return ['kind' => 'float', 'value' => (string) $value];
        }
        if (is_string($value)) {
            return ['kind' => 'string', 'value' => $value];
        }
        if (is_array($value)) {
            if (array_is_list($value)) {
                return ['kind' => 'array', 'items' => array_map(self::canonicalValue(...), $value)];
            }

            return ['kind' => 'object', 'fields' => self::canonicalRow($value)];
        }

        return ['kind' => 'string', 'value' => (string) $value];
    }
}
