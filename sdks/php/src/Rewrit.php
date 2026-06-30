<?php

declare(strict_types=1);

namespace Rewrit;

final class Rewrit
{
    private static ?string $currentCaseId = null;
    private static ?string $currentSuiteId = null;
    /** @var array<string, mixed>|null */
    private static ?array $lastObservation = null;

    public static function case(string $caseId, ?string $suiteId = null, ?string $title = null): void
    {
        self::$currentCaseId = $caseId;
        self::$currentSuiteId = $suiteId ?? self::suiteFromCaseId($caseId);

        self::emit([
            'schema_version' => 'rewrit.event.v1',
            'kind' => 'case_discovered',
            'runtime_id' => self::runtimeId(),
            'case' => [
                'id' => $caseId,
                'suite_id' => self::$currentSuiteId,
                'title' => $title ?? $caseId,
                'source_location' => null,
                'tags' => [],
                'contract_ref' => null,
                'required' => true,
            ],
        ]);
    }

    public static function observe(mixed $value = null, ?string $caseId = null, string $status = 'passed', array $effects = []): void
    {
        self::observeCanonical(
            $value === null ? null : ['kind' => 'json', 'value' => $value],
            $caseId,
            $status,
            $effects,
        );
    }

    public static function observeCanonical(
        ?array $value = null,
        ?string $caseId = null,
        string $status = 'passed',
        array $effects = [],
    ): void
    {
        $caseId ??= self::$currentCaseId;
        if ($caseId === null) {
            throw new \RuntimeException('Rewrit case id is missing. Call rewrit($caseId) first.');
        }

        self::emitObservation([
            'schema_version' => 'rewrit.event.v1',
            'kind' => 'observation',
            'case_id' => $caseId,
            'runtime_id' => self::runtimeId(),
            'status' => $status,
            'value' => $value,
            'error' => null,
            'stdout' => ['text' => '', 'truncated' => false],
            'stderr' => ['text' => '', 'truncated' => false],
            'exit_code' => 0,
            'duration_ms' => 0,
            'effects' => $effects,
            'artifacts' => [],
            'metadata' => self::$currentSuiteId === null ? [] : ['suite_id' => self::$currentSuiteId],
        ]);
    }

    public static function addEffect(array $effect, ?string $caseId = null): void
    {
        $caseId ??= self::$currentCaseId;
        if ($caseId === null) {
            throw new \RuntimeException('Rewrit case id is missing. Call rewrit($caseId) first.');
        }

        if (self::$lastObservation !== null && self::$lastObservation['case_id'] === $caseId) {
            self::$lastObservation['effects'][] = $effect;
            self::emit(self::$lastObservation);

            return;
        }

        self::observeCanonical(null, $caseId, 'passed', [$effect]);
    }

    public static function runtimeId(): string
    {
        return getenv('REWRIT_RUNTIME_ID') ?: 'reference';
    }

    public static function emit(array $event): void
    {
        $encoded = json_encode($event, JSON_THROW_ON_ERROR) . PHP_EOL;
        $eventsPath = getenv('REWRIT_EVENTS_PATH');

        if (is_string($eventsPath) && $eventsPath !== '') {
            file_put_contents($eventsPath, $encoded, FILE_APPEND | LOCK_EX);

            return;
        }

        fwrite(STDOUT, $encoded);
    }

    private static function emitObservation(array $event): void
    {
        self::$lastObservation = $event;
        self::emit($event);
    }

    private static function suiteFromCaseId(string $caseId): string
    {
        $separator = strpos($caseId, '.');

        return $separator === false ? 'default' : substr($caseId, 0, $separator);
    }
}
