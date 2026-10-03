<?php

declare(strict_types=1);

namespace Lingara\Apps;

use Lingara\Apps\Generated\Card as CardModel;

/**
 * A card and, optionally, a tutor note: what a render or action function
 * may return instead of a bare card.
 *
 *     return Reply::of($card)->tutorNote('The learner is reviewing weather words.');
 */
final class Reply
{
    private ?string $tutorNote = null;

    private function __construct(public readonly CardModel $card) {}

    public static function of(CardModel $card): self
    {
        return new self($card);
    }

    /**
     * Plain text for the tutor, at most 280 characters once `\n` becomes a
     * space and the ends are trimmed; no control or bidi character but `\n`.
     *
     * @throws CardLimitException tutor_note_length or control_chars
     */
    public function tutorNote(string $text): self
    {
        $reason = Limits::tutorNoteReason($text);
        if ($reason !== null) {
            throw new CardLimitException($reason);
        }
        $this->tutorNote = $text;
        return $this;
    }

    /**
     * The wire form, `{card, tutor_note?}`: an absent note is omitted.
     *
     * @return array{card: CardModel, tutor_note?: string}
     */
    public function toWire(): array
    {
        return $this->tutorNote === null ? ['card' => $this->card] : ['card' => $this->card, 'tutor_note' => $this->tutorNote];
    }
}
