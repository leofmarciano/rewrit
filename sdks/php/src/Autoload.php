<?php

declare(strict_types=1);

namespace Rewrit;

if (class_exists(\Pest\Plugin::class)) {
    \Pest\Plugin::uses(PestPlugin::class);
}

function rewrit(string $caseId, ?string $suiteId = null, ?string $title = null): void
{
    Rewrit::case($caseId, $suiteId, $title);
}
