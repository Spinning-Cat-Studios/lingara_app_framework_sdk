<?php

declare(strict_types=1);

namespace Lingara\Apps\Internal;

use Lingara\Apps\Limits;

/**
 * The rules a reply breaks, collected over its plain JSON tree; the reason
 * is the first in Limits::REASONS order. A port of the contract's reference
 * validator, field for field.
 *
 * @internal
 */
final class ReplyRules
{
    /** @var array<string, true> */
    private array $broken = [];

    public function when(string $reason, bool $broken): void
    {
        if ($broken) {
            $this->broken[$reason] = true;
        }
    }

    public function first(): ?string
    {
        foreach (Limits::REASONS as $reason) {
            if (isset($this->broken[$reason])) {
                return $reason;
            }
        }
        return null;
    }

    public function card(mixed $card): void
    {
        $elements = Json::items(Json::member($card, 'elements'));
        $buttons = 0;
        foreach ($elements as $element) {
            $this->element($element);
            $buttons += Json::member($element, 'type') === 'button' ? 1 : 0;
        }
        $this->when('buttons', $buttons > Limits::MAX_BUTTONS);
        $this->when('elements', count($elements) > Limits::MAX_ELEMENTS);
        $this->when('empty_card', $elements === []);
    }

    /** `\n` is legal in a note: the relay turns it into a space and trims before it counts. */
    public function tutorNote(mixed $note): void
    {
        if (!is_string($note)) {
            return;
        }
        $this->when('control_chars', Limits::hasControl($note, true));
        $line = Limits::trimWhiteSpace(str_replace("\n", ' ', $note));
        $this->when('tutor_note_length', Limits::scalarCount($line) > Limits::TUTOR_NOTE_MAX_CHARS);
    }

    private function element(mixed $e): void
    {
        match (Json::member($e, 'type')) {
            'heading' => $this->heading($e),
            'text', 'term' => $this->item($e),
            'list' => $this->list($e),
            'progress' => $this->progress($e),
            'button' => $this->button($e),
            'link' => $this->link($e),
            default => null,
        };
    }

    private function heading(mixed $e): void
    {
        $this->text(Json::member($e, 'text'), Limits::HEADING_MAX_CHARS);
        $level = Json::member($e, 'level');
        $this->when('heading_level', $level !== 1 && $level !== 2);
    }

    private function item(mixed $item): void
    {
        $type = Json::member($item, 'type');
        if ($type === 'text') {
            $this->text(Json::member($item, 'text'), Limits::TEXT_MAX_CHARS, newline: true);
        } elseif ($type === 'term') {
            $this->text(Json::member($item, 'word'), Limits::TERM_WORD_MAX_CHARS);
            $this->text(Json::member($item, 'reading'), Limits::TERM_READING_MAX_CHARS, required: false);
            $this->text(Json::member($item, 'gloss'), Limits::TERM_GLOSS_MAX_CHARS, required: false);
        } else {
            return;
        }
        $lang = $this->text(Json::member($item, 'lang'), PHP_INT_MAX, required: false);
        if ($lang !== null) {
            $this->when('lang', preg_match(Limits::LANG_PATTERN, $lang) !== 1);
        }
    }

    private function list(mixed $e): void
    {
        $items = Json::items(Json::member($e, 'items'));
        foreach ($items as $item) {
            $this->item($item);
        }
        $this->when('list_items', $items === [] || count($items) > Limits::MAX_LIST_ITEMS);
    }

    private function progress(mixed $e): void
    {
        $value = Json::member($e, 'value');
        $inRange = (is_int($value) || is_float($value)) && $value >= 0 && $value <= 1;
        $this->when('progress_range', !$inRange);
        $this->text(Json::member($e, 'label'), Limits::PROGRESS_LABEL_MAX_CHARS);
    }

    private function button(mixed $e): void
    {
        $this->text(Json::member($e, 'label'), Limits::BUTTON_LABEL_MAX_CHARS);
        $action = $this->text(Json::member($e, 'action'), PHP_INT_MAX, required: false) ?? '';
        $this->when('button_action', preg_match(Limits::ACTION_PATTERN, $action) !== 1);
    }

    private function link(mixed $e): void
    {
        $this->text(Json::member($e, 'label'), Limits::LINK_LABEL_MAX_CHARS);
        $url = $this->text(Json::member($e, 'url'), PHP_INT_MAX, required: false) ?? '';
        $this->when('link', !Limits::isSafeLink($url));
    }

    /**
     * One text field: a missing or non-string required field is empty; a
     * string is checked for controls, length and emptiness. Returns the
     * string when there is one.
     */
    private function text(mixed $value, int $limit, bool $newline = false, bool $required = true): ?string
    {
        if (!is_string($value)) {
            $this->when('empty_element', $required);
            return null;
        }
        $this->when('control_chars', Limits::hasControl($value, $newline));
        $this->when('text_length', Limits::scalarCount($value) > $limit);
        $this->when('empty_element', $required && Limits::trimWhiteSpace($value) === '');
        return $value;
    }
}
