<?php

declare(strict_types=1);

namespace Lingara\Apps;

use Lingara\Apps\Generated\ButtonStyle;
use Lingara\Apps\Generated\Card as CardModel;
use Lingara\Apps\Generated\CardElement;
use Lingara\Apps\Generated\CardElementButton;
use Lingara\Apps\Generated\CardElementDivider;
use Lingara\Apps\Generated\CardElementHeading;
use Lingara\Apps\Generated\CardElementLink;
use Lingara\Apps\Generated\CardElementList;
use Lingara\Apps\Generated\CardElementProgress;
use Lingara\Apps\Generated\CardElementTerm;
use Lingara\Apps\Generated\CardElementText;
use Lingara\Apps\Generated\ListItem;

/**
 * A fluent card (ADR 30.9.26am D5): one method per element, each with its
 * generated arm's closed field set, and build(), which runs the card rules
 * and throws CardLimitException with the first one the card breaks. A kit
 * refuses what the relay would clamp, so the clamp never fires on a
 * kit-built card. An optional member left null is omitted, never `null`.
 *
 *     $card = Card::create()
 *         ->heading('Today', 1)
 *         ->term('雨', reading: 'yǔ', gloss: 'rain', lang: 'zh')
 *         ->list([Item::text('one'), Item::term('二')])
 *         ->button('Next', 'next')
 *         ->build();
 */
final class Card
{
    /** @var list<CardElement> */
    private array $elements = [];

    public static function create(): self
    {
        return new self();
    }

    public function heading(string $text, int $level = 1): self
    {
        return $this->push(new CardElementHeading($text, $level));
    }

    /** `\n` is allowed in a text element. */
    public function text(string $text, ?string $lang = null): self
    {
        return $this->push(new CardElementText($text, $lang));
    }

    public function term(string $word, ?string $reading = null, ?string $gloss = null, ?string $lang = null): self
    {
        return $this->push(new CardElementTerm($word, $reading, $gloss, $lang));
    }

    /** @param list<ListItem> $items built with Item::text() and Item::term() */
    public function list(array $items): self
    {
        return $this->push(new CardElementList($items));
    }

    /** `$value` from 0 to 1 inclusive. */
    public function progress(float $value, string $label): self
    {
        return $this->push(new CardElementProgress($value, $label));
    }

    public function divider(): self
    {
        return $this->push(new CardElementDivider());
    }

    /** `$action` comes back as the request's `action_id`. */
    public function button(string $label, string $action, ?ButtonStyle $style = null): self
    {
        return $this->push(new CardElementButton($label, $action, $style));
    }

    /** `https`, no userinfo, a host name (never an IP address), at most 2 048 bytes. */
    public function link(string $label, string $url): self
    {
        return $this->push(new CardElementLink($label, $url));
    }

    /**
     * The card, as the generated model.
     *
     * @throws CardLimitException naming the first card rule it breaks
     */
    public function build(): CardModel
    {
        $card = new CardModel(['elements' => $this->elements]);
        $reason = Limits::cardReason($card);
        if ($reason !== null) {
            throw new CardLimitException($reason);
        }
        return $card;
    }

    private function push(CardElement $element): self
    {
        $this->elements[] = $element;
        return $this;
    }
}
