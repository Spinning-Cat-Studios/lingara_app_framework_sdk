<?php

declare(strict_types=1);

namespace Lingara\Apps;

use Lingara\Apps\Generated\ListItemTerm;
use Lingara\Apps\Generated\ListItemText;

/**
 * List items for Card::list(): `Item::text(…)` and `Item::term(…)`, each
 * with its generated arm's fields. An optional member left null is omitted.
 */
final class Item
{
    private function __construct() {}

    public static function text(string $text, ?string $lang = null): ListItemText
    {
        return new ListItemText($text, $lang);
    }

    public static function term(string $word, ?string $reading = null, ?string $gloss = null, ?string $lang = null): ListItemTerm
    {
        return new ListItemTerm($word, $reading, $gloss, $lang);
    }
}
