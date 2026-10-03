<?php

declare(strict_types=1);

namespace Lingara\Apps;

/**
 * A card or reply the relay would clamp, refused with the first rule it
 * breaks: one of Limits::REASONS.
 */
final class CardLimitException extends \DomainException
{
    public function __construct(public readonly string $reason)
    {
        parent::__construct("the card breaks the {$reason} rule");
    }
}
