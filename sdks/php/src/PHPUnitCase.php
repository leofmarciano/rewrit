<?php

declare(strict_types=1);

namespace Rewrit;

trait PHPUnitCase
{
    protected function rewrit(string $caseId, ?string $suiteId = null, ?string $title = null): void
    {
        Rewrit::case($caseId, $suiteId, $title);
    }

    protected function observeRewrit(mixed $value = null, ?string $caseId = null, string $status = 'passed'): void
    {
        Rewrit::observe($value, $caseId, $status);
    }
}
