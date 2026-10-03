<?php

declare(strict_types=1);

namespace Lingara\Apps;

/**
 * A manifest the upload would refuse, with the first rule it breaks: one of
 * Manifest::RULES.
 */
final class ManifestException extends \DomainException
{
    public function __construct(public readonly string $rule)
    {
        parent::__construct("the manifest breaks the {$rule} rule");
    }
}
