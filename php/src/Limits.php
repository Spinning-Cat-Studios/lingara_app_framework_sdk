<?php

declare(strict_types=1);

namespace Lingara\Apps;

use Lingara\Apps\Internal\Json;
use Lingara\Apps\Internal\ReplyRules;

/**
 * The card rules (ADR 30.9.26am D5): the relay's clamp restated as refusals,
 * held to conformance/vectors/card-limits.json and to the contract's
 * reference validator. Every limit is a constant here and nowhere else,
 * because the view carries none of them: they are post-parse clamps.
 *
 * A "character" is a Unicode scalar value (`mb_strlen($s, 'UTF-8')`), never
 * a UTF-16 unit or a grapheme. "Empty" is empty after trimming Unicode
 * White_Space, which PHP's own trim() does not do (it trims ASCII only).
 */
final class Limits
{
    /** The largest reply body the relay reads, in bytes. */
    public const REPLY_MAX_BYTES = 32768;
    /** The largest request body a kit reads, in bytes. */
    public const REQUEST_MAX_BYTES = 65536;
    public const MAX_ELEMENTS = 24;
    public const MAX_BUTTONS = 4;
    public const MAX_LIST_ITEMS = 20;
    public const TUTOR_NOTE_MAX_CHARS = 280;
    public const MAX_URL_BYTES = 2048;

    public const HEADING_MAX_CHARS = 80;
    public const TEXT_MAX_CHARS = 600;
    public const TERM_WORD_MAX_CHARS = 60;
    public const TERM_READING_MAX_CHARS = 120;
    public const TERM_GLOSS_MAX_CHARS = 160;
    public const PROGRESS_LABEL_MAX_CHARS = 60;
    public const BUTTON_LABEL_MAX_CHARS = 32;
    public const LINK_LABEL_MAX_CHARS = 60;

    /** Whole-string patterns: `\A…\z`, never per line. */
    public const LANG_PATTERN = '/\A[A-Za-z]{2,3}(-[A-Za-z0-9]{2,8}){0,3}\z/D';
    public const ACTION_PATTERN = '/\A[A-Za-z0-9_.:-]{1,64}\z/D';

    /** The refusal reasons, in the contract's order: a reply's reason is the first it breaks. */
    public const REASONS = [
        'control_chars',
        'text_length',
        'heading_level',
        'progress_range',
        'lang',
        'link',
        'button_action',
        'buttons',
        'list_items',
        'empty_element',
        'elements',
        'empty_card',
        'tutor_note_length',
        'reply_too_large',
    ];

    /**
     * The flags every reply is encoded with: compact JSON, raw UTF-8 (U+2028
     * and U+2029 included), `/` unescaped, and `1.0` kept a float, as the
     * reference's serde_json writes it.
     */
    public const JSON_FLAGS = JSON_UNESCAPED_UNICODE | JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_LINE_TERMINATORS
        | JSON_PRESERVE_ZERO_FRACTION | JSON_THROW_ON_ERROR;

    // Unicode White_Space, which the relay's `str::trim` trims, spelled out:
    // `\p{White_Space}` needs a newer PCRE2 than every PHP 8.2 build links.
    private const WHITE_SPACE = '[\x{0009}-\x{000D}\x{0020}\x{0085}\x{00A0}\x{1680}\x{2000}-\x{200A}\x{2028}\x{2029}\x{202F}\x{205F}\x{3000}]';
    // C0 and C1 controls, the bidi overrides and the bidi isolates; the
    // second leaves `\n` out.
    private const STRIPPED = '/[\x{0000}-\x{001F}\x{007F}-\x{009F}\x{202A}-\x{202E}\x{2066}-\x{2069}]/u';
    private const STRIPPED_BUT_NEWLINE = '/[\x{0000}-\x{0009}\x{000B}-\x{001F}\x{007F}-\x{009F}\x{202A}-\x{202E}\x{2066}-\x{2069}]/u';

    private function __construct() {}

    /**
     * Every card rule, the tutor note, and the size of the reply encoded
     * exactly as it will be sent. Returns those bytes, or throws
     * CardLimitException with the first reason the reply breaks. Accepts any
     * JSON value (a decoded `stdClass` tree, arrays, the generated arms), so
     * a vector feeds it as is.
     *
     * @throws CardLimitException
     */
    public static function validateReply(mixed $reply): string
    {
        $rules = new ReplyRules();
        $plain = Json::plain($reply);
        $rules->card(Json::member($plain, 'card'));
        $rules->tutorNote(Json::member($plain, 'tutor_note'));
        $bytes = Json::encode($plain);
        $rules->when('reply_too_large', $bytes === null || strlen($bytes) > self::REPLY_MAX_BYTES);
        $reason = $rules->first();
        if ($reason !== null || $bytes === null) {
            throw new CardLimitException($reason ?? 'reply_too_large');
        }
        return $bytes;
    }

    /**
     * The card rules alone (reasons control_chars to empty_card): what
     * Card::build() runs. Null when the card keeps every rule.
     */
    public static function cardReason(mixed $card): ?string
    {
        $rules = new ReplyRules();
        $rules->card(Json::plain($card));
        return $rules->first();
    }

    /** The tutor note's rules alone: what Reply::tutorNote() runs. */
    public static function tutorNoteReason(string $note): ?string
    {
        $rules = new ReplyRules();
        $rules->tutorNote($note);
        return $rules->first();
    }

    /**
     * The relay's cut, never applied implicitly: over `$limit` scalar
     * values, the first `$limit − 1` and `…`; otherwise unchanged.
     */
    public static function truncate(string $text, int $limit): string
    {
        if (self::scalarCount($text) <= $limit) {
            return $text;
        }
        return mb_substr($text, 0, max($limit - 1, 0), 'UTF-8') . '…';
    }

    /** The length in Unicode scalar values. */
    public static function scalarCount(string $text): int
    {
        return mb_strlen($text, 'UTF-8');
    }

    /** Trims Unicode White_Space from both ends. */
    public static function trimWhiteSpace(string $text): string
    {
        $ws = self::WHITE_SPACE;
        return preg_replace("/\\A{$ws}+|{$ws}+\\z/u", '', $text) ?? $text;
    }

    /** Whether `$text` holds a control or bidi character, `\n` aside when `$newline` allows it. */
    public static function hasControl(string $text, bool $newline): bool
    {
        return preg_match($newline ? self::STRIPPED_BUT_NEWLINE : self::STRIPPED, $text) === 1;
    }

    /** `https`, no userinfo, a named host (never an IP literal), at most MAX_URL_BYTES. */
    public static function isSafeLink(string $raw): bool
    {
        $parts = strlen($raw) > self::MAX_URL_BYTES ? false : parse_url($raw);
        if ($parts === false || strtolower($parts['scheme'] ?? '') !== 'https') {
            return false;
        }
        $host = $parts['host'] ?? '';
        $userinfo = ($parts['user'] ?? '') !== '' || ($parts['pass'] ?? '') !== '';
        return !$userinfo && $host !== '' && !self::isIpHost($host);
    }

    /**
     * An IP literal to a WHATWG parser, as to the relay: a bracketed IPv6
     * host, or a host whose last label (a trailing dot ignored) is a number,
     * decimal or 0x-hex, which WHATWG reads as IPv4 (127.1, 0x7f.1).
     */
    private static function isIpHost(string $host): bool
    {
        if (str_starts_with($host, '[') || filter_var($host, FILTER_VALIDATE_IP) !== false) {
            return true;
        }
        $labels = explode('.', rtrim($host, '.'));
        $last = strtolower((string) end($labels));
        return $last !== '' && preg_match('/\A(0x[0-9a-f]*|[0-9]+)\z/D', $last) === 1;
    }
}
