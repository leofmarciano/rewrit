<?php

namespace Rewrit;

final class Rewrit
{
    public static function observe(string $caseId, string $runtimeId, mixed $value = null): void
    {
        $event = [
            'schema_version' => 'rewrit.event.v1',
            'kind' => 'observation',
            'case_id' => $caseId,
            'runtime_id' => $runtimeId,
            'status' => 'passed',
            'value' => $value === null ? null : ['kind' => 'json', 'value' => $value],
            'error' => null,
            'stdout' => ['text' => '', 'truncated' => false],
            'stderr' => ['text' => '', 'truncated' => false],
            'exit_code' => 0,
            'duration_ms' => 0,
            'effects' => [],
            'artifacts' => [],
            'metadata' => [],
        ];

        fwrite(STDOUT, json_encode($event, JSON_THROW_ON_ERROR) . PHP_EOL);
    }
}

