<?php

declare(strict_types=1);

// The PHP app kit's examples the documentation site shows. Each marked
// region is vendored at a released tag; `make test-php` parses every file
// here and type-checks it against the kit with PHPStan. A `use` line is legal
// only at namespace scope, so a key that shows one has two regions: the
// `use` lines, then the code.

namespace LingaraAppsSnippets;

// lingara:begin card
use Lingara\Apps\Card;
use Lingara\Apps\Generated\Card as CardModel;
use Lingara\Apps\Item;
// lingara:end

// lingara:begin card
function todayCard(int $streak): CardModel
{
    return Card::create()
        ->heading("Today's five", 1)
        ->term('雨', reading: 'yǔ', gloss: 'rain', lang: 'zh')
        ->list([Item::text('Review 3 words'), Item::term('二', reading: 'èr')])
        ->button("Done ({$streak})", 'done')
        ->build(); // Throws CardLimitException naming the rule the relay would clamp.
}
// lingara:end
