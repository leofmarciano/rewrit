<?php

declare(strict_types=1);

namespace Rewrit;

trait PestPlugin
{
    public function rewrit(string $caseId, ?string $suiteId = null): static
    {
        Rewrit::case($caseId, $suiteId);

        return $this;
    }
}
