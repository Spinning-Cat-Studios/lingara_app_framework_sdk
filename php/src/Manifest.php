<?php

declare(strict_types=1);

namespace Lingara\Apps;

use Lingara\Apps\Generated\AppSlotName;
use Lingara\Apps\Generated\ContextSlice;
use Lingara\Apps\Internal\Json;

/**
 * The manifest builder (ADR 30.9.26am D5), hand-written against the
 * manifest's wire form, which is in no spec the view reads. Its closed
 * values still come from the view: slots from the generated AppSlotName,
 * context from the ContextSlice kinds the emitter writes.
 *
 *     $json = Manifest::create()
 *         ->defaultLocale('en')
 *         ->name(['en' => 'Daily five', 'zh-Hans' => '每日五词'])
 *         ->description('Five words to review.')
 *         ->renderUrl('https://apps.example.com/lingara/render')
 *         ->slots(AppSlotName::HOME_SIDE)
 *         ->context('languages', 'plan_summary')
 *         ->toJson();
 *
 * validateManifest() refuses what the upload would refuse or normalise away
 * where a kit can know it, naming the first broken rule in the order
 * conformance/vectors/manifest.json describes.
 */
final class Manifest
{
    public const NAME_MAX_CHARS = 40;
    public const DESCRIPTION_MAX_CHARS = 280;
    public const STORED_MAX_BYTES = 65536;

    /** The rules, in order: a refusal names the first one broken. */
    public const RULES = [
        'manifest_version', 'default_locale', 'name', 'description', 'render_url', 'slots', 'context', 'duplicate', 'too_large',
    ];

    // Zero-width characters, beyond the card rules' controls and bidi marks.
    private const ZERO_WIDTH = '/[\x{200B}-\x{200F}\x{FEFF}]/u';
    private const RENDER_URL = '/\Ahttps:\/\/[^\/?#]/i';

    private string $defaultLocale = '';
    /** @var string|array<string, string> */
    private string|array $name = '';
    /** @var string|array<string, string> */
    private string|array $description = '';
    private string $renderUrl = '';
    /** @var list<AppSlotName> */
    private array $slots = [];
    /** @var list<string> */
    private array $context = [];
    /** @var list<string> */
    private array $scopes = [];
    private bool $tutorNote = false;
    private bool $listed = false;

    public static function create(): self
    {
        return new self();
    }

    public function defaultLocale(string $locale): self
    {
        $this->defaultLocale = $locale;
        return $this;
    }

    /** @param string|array<string, string> $name a locale map, or a bare string for the default locale's */
    public function name(string|array $name): self
    {
        $this->name = $name;
        return $this;
    }

    /** @param string|array<string, string> $description a locale map, or a bare string for the default locale's */
    public function description(string|array $description): self
    {
        $this->description = $description;
        return $this;
    }

    public function renderUrl(string $url): self
    {
        $this->renderUrl = $url;
        return $this;
    }

    public function slots(AppSlotName ...$slots): self
    {
        $this->slots = array_values($slots);
        return $this;
    }

    /** @param value-of<ContextSlice::KINDS> ...$kinds */
    public function context(string ...$kinds): self
    {
        $this->context = array_values($kinds);
        return $this;
    }

    public function scopes(string ...$scopes): self
    {
        $this->scopes = array_values($scopes);
        return $this;
    }

    /** Whether the app's replies may carry a tutor note. */
    public function tutorNote(bool $enabled = true): self
    {
        $this->tutorNote = $enabled;
        return $this;
    }

    /** Whether the app appears in the catalogue; false unless set. */
    public function listed(bool $listed = true): self
    {
        $this->listed = $listed;
        return $this;
    }

    /**
     * The manifest's wire form.
     *
     * @return array<string, mixed>
     *
     * @throws ManifestException naming the first rule it breaks
     */
    public function build(): array
    {
        $manifest = [
            'manifest_version' => 1,
            'default_locale' => $this->defaultLocale,
            'name' => $this->localized($this->name),
            'description' => $this->localized($this->description),
            'render_url' => $this->renderUrl,
            'slots' => array_map(static fn(AppSlotName $slot): string => $slot->value, $this->slots),
            'context' => $this->context,
            'scopes' => $this->scopes,
            'tutor_note' => $this->tutorNote,
            'listed' => $this->listed,
        ];
        self::validateManifest($manifest);
        return $manifest;
    }

    /**
     * The validated manifest as the JSON the console accepts as an upload.
     *
     * @throws ManifestException
     */
    public function toJson(): string
    {
        return json_encode($this->build(), Limits::JSON_FLAGS | JSON_PRETTY_PRINT) . "\n";
    }

    /**
     * Throws ManifestException naming the first rule `$manifest` breaks.
     * Accepts any JSON value: a decoded `stdClass` tree or arrays.
     *
     * @throws ManifestException
     */
    public static function validateManifest(mixed $manifest): void
    {
        $rule = self::firstBrokenRule(Json::plain($manifest));
        if ($rule !== null) {
            throw new ManifestException($rule);
        }
    }

    private static function firstBrokenRule(mixed $m): ?string
    {
        $kept = [
            'manifest_version' => static fn(): bool => Json::member($m, 'manifest_version') === 1,
            'default_locale' => static fn(): bool => self::defaultLocaleKept($m),
            'name' => static fn(): bool => self::localizedKept(Json::member($m, 'name'), self::NAME_MAX_CHARS),
            'description' => static fn(): bool => self::localizedKept(Json::member($m, 'description'), self::DESCRIPTION_MAX_CHARS),
            'render_url' => static fn(): bool => is_string($url = Json::member($m, 'render_url')) && preg_match(self::RENDER_URL, $url) === 1,
            'slots' => static fn(): bool => self::within($m, 'slots', self::slotNames(), optional: false),
            'context' => static fn(): bool => self::within($m, 'context', ContextSlice::KINDS, optional: true),
            'duplicate' => static fn(): bool => !self::hasDuplicate($m),
            'too_large' => static fn(): bool => strlen(Json::encode(self::stored($m)) ?? '') <= self::STORED_MAX_BYTES,
        ];
        foreach ($kept as $rule => $check) {
            if (!$check()) {
                return $rule;
            }
        }
        return null;
    }

    private static function defaultLocaleKept(mixed $m): bool
    {
        $locale = Json::member($m, 'default_locale');
        if (!is_string($locale) || $locale === '') {
            return false;
        }
        foreach (['name', 'description'] as $key) {
            $map = Json::member($m, $key);
            if (Json::isObject($map) && !Json::has($map, $locale)) {
                return false;
            }
        }
        return true;
    }

    /** A non-empty locale map whose every value is 1–`$max` clean characters once trimmed. */
    private static function localizedKept(mixed $value, int $max): bool
    {
        if (!Json::isObject($value) || Json::members($value) === []) {
            return false;
        }
        foreach (Json::members($value) as $text) {
            if (!is_string($text)) {
                return false;
            }
            $trimmed = Limits::trimWhiteSpace($text);
            $n = Limits::scalarCount($trimmed);
            if ($n < 1 || $n > $max || Limits::hasControl($trimmed, false) || preg_match(self::ZERO_WIDTH, $trimmed) === 1) {
                return false;
            }
        }
        return true;
    }

    /** @param list<string> $allowed */
    private static function within(mixed $m, string $key, array $allowed, bool $optional): bool
    {
        if (!Json::has($m, $key)) {
            return $optional;
        }
        $value = Json::member($m, $key);
        if (!is_array($value) || !array_is_list($value) || (!$optional && $value === [])) {
            return false;
        }
        foreach ($value as $item) {
            if (!is_string($item) || !in_array($item, $allowed, true)) {
                return false;
            }
        }
        return true;
    }

    private static function hasDuplicate(mixed $m): bool
    {
        foreach (['slots', 'context', 'scopes'] as $key) {
            $seen = array_map(static fn(mixed $v): string => Json::encode($v) ?? '', Json::items(Json::member($m, $key)));
            if (count(array_unique($seen)) !== count($seen)) {
                return true;
            }
        }
        return false;
    }

    /** A1's seven stored keys, with their defaults. */
    private static function stored(mixed $m): \stdClass
    {
        return (object) [
            'default_locale' => Json::member($m, 'default_locale'),
            'name' => Json::member($m, 'name'),
            'description' => Json::member($m, 'description'),
            'slots' => Json::member($m, 'slots'),
            'context' => Json::member($m, 'context') ?? [],
            'scopes' => Json::member($m, 'scopes') ?? [],
            'tutor_note' => Json::member($m, 'tutor_note') ?? false,
        ];
    }

    /** @return list<string> every slot name the view lists */
    private static function slotNames(): array
    {
        $names = [];
        foreach (AppSlotName::cases() as $slot) {
            if ($slot !== AppSlotName::UNKNOWN_DEFAULT_OPEN_API) {
                $names[] = $slot->value;
            }
        }
        return $names;
    }

    /**
     * @param string|array<string, string> $value
     *
     * @return \stdClass a locale map, an object even when its keys look numeric
     */
    private function localized(string|array $value): \stdClass
    {
        return (object) (is_string($value) ? [$this->defaultLocale => $value] : $value);
    }
}
