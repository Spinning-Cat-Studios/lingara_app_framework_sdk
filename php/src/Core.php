<?php

declare(strict_types=1);

namespace Lingara\Apps;

use Lingara\Apps\Generated\AppActionRequest;
use Lingara\Apps\Generated\AppOperation;
use Lingara\Apps\Generated\AppRenderRequest;
use Lingara\Apps\Generated\AppSlotName;
use Lingara\Apps\Generated\Card as CardModel;
use Lingara\Apps\Generated\ContextSlice;
use Lingara\Apps\Internal\Json;
use Lingara\Events\VerificationException;
use Psr\Http\Message\MessageInterface;

/**
 * The handler core (ADR 30.9.26am D4), framework-neutral: method, size,
 * signature, lenient decode, dispatch, validate, encode. An adapter supplies
 * the method, the headers and a body reader, and writes back what handle()
 * returns. The signature check is the library's (verifySignature), never
 * the kit's own.
 */
final class Core
{
    /**
     * Each operation's handler. Its keys are AppOperation's values, which a
     * unit test holds them to: a new operation in the view fails it until
     * it is handled here.
     */
    public const DISPATCH = [
        'app.render' => 'render',
        'app.action' => 'action',
    ];

    private const COMMON = ['id', 'install_id', 'subject', 'locale'];
    private const ACTION = ['action_id', 'card_etag'];

    private function __construct() {}

    /**
     * One request, start to finish (AK1–AK4). `$body` reads at most the
     * byte count it is given, and may stop there.
     *
     * @param array<string, string|list<string>>|MessageInterface $headers
     * @param \Closure(int): string                                 $body
     */
    public static function handle(App $app, string $method, array|MessageInterface $headers, \Closure $body): CoreResponse
    {
        if ($method !== 'POST') {
            return new CoreResponse(405);
        }
        $bytes = $body(Limits::REQUEST_MAX_BYTES + 1);
        if (strlen($bytes) > Limits::REQUEST_MAX_BYTES) {
            return new CoreResponse(413);
        }
        try {
            $app->verifySignature($bytes, $headers);
        } catch (VerificationException) {
            return new CoreResponse(401);
        }
        $request = self::decode($bytes);
        $fn = $request === null ? null : self::route($app, $request);
        if ($request === null || $fn === null) {
            return self::badRequest();
        }
        return self::answer($request, $fn);
    }

    /**
     * The request, decoded leniently into its generated model, or null (a
     * `400`). An unknown member is ignored, and a context slice of an
     * unknown kind is skipped with a log line.
     */
    public static function decode(string $body): AppRenderRequest|AppActionRequest|null
    {
        try {
            $raw = json_decode($body, false, 512, JSON_THROW_ON_ERROR);
        } catch (\JsonException) {
            return null;
        }
        $type = Json::member($raw, 'type');
        $operation = is_string($type) ? AppOperation::tryFrom($type) : null;
        if (!$raw instanceof \stdClass || $operation === null || !self::fieldsKept($raw, $operation)) {
            return null;
        }
        $context = self::decodeContext($raw->context ?? null);
        if ($context === null) {
            return null;
        }
        $known = clone $raw;
        unset($known->context);
        $request = ObjectSerializer::deserialize($known, $operation->request());
        if (!$request instanceof AppRenderRequest && !$request instanceof AppActionRequest) {
            return null;
        }
        return $request->setContext($context);
    }

    private static function fieldsKept(\stdClass $raw, AppOperation $operation): bool
    {
        $fields = $operation === AppOperation::ACTION ? [...self::COMMON, ...self::ACTION] : self::COMMON;
        foreach ($fields as $field) {
            if (!is_string($raw->{$field} ?? null)) {
                return false;
            }
        }
        $slot = is_string($raw->slot ?? null) ? AppSlotName::tryFrom($raw->slot) : null;
        return $slot !== null && $slot !== AppSlotName::UNKNOWN_DEFAULT_OPEN_API;
    }

    /**
     * The context, element by element, before anything reads it as the
     * union: null when it does not decode.
     *
     * @return list<ContextSlice>|null
     */
    private static function decodeContext(mixed $raw): ?array
    {
        if (!is_array($raw) || !array_is_list($raw)) {
            return null;
        }
        $slices = [];
        foreach ($raw as $element) {
            $kind = Json::member($element, ContextSlice::TAG);
            if (!$element instanceof \stdClass || !is_string($kind)) {
                return null;
            }
            $arm = ContextSlice::ARMS[$kind] ?? null;
            if ($arm === null) {
                error_log('lingara-apps: skipped a context slice of unknown kind ' . json_encode(mb_substr($kind, 0, 64), Limits::JSON_FLAGS));
                continue;
            }
            $slice = $arm::fromValue($element);
            if ($slice === null) {
                return null;
            }
            $slices[] = $slice;
        }
        return $slices;
    }

    /**
     * The developer's function for a request, or null for an unregistered
     * action.
     *
     * @return (\Closure(): (CardModel|Reply))|null
     */
    private static function route(App $app, AppRenderRequest|AppActionRequest $request): ?\Closure
    {
        $handler = self::DISPATCH[$request->getType()] ?? null;
        if ($handler === 'render' && $request instanceof AppRenderRequest) {
            return static fn(): CardModel|Reply => $app->render($request);
        }
        if ($handler === 'action' && $request instanceof AppActionRequest) {
            $fn = $app->action($request->getActionId());
            return $fn === null ? null : static fn(): CardModel|Reply => $fn($request);
        }
        return null;
    }

    /** @param \Closure(): (CardModel|Reply) $fn */
    private static function answer(AppRenderRequest|AppActionRequest $request, \Closure $fn): CoreResponse
    {
        $operation = $request->getType();
        $installId = $request->getInstallId();
        try {
            $result = $fn();
            $wire = $result instanceof Reply ? $result->toWire() : ['card' => $result];
            return CoreResponse::json(200, Limits::validateReply($wire));
        } catch (CardLimitException $e) {
            // build() inside the function, or the whole reply here: either way the card rules refused it.
            self::log($operation, $installId, "card refused: {$e->reason}");
        } catch (\Throwable $e) {
            self::log($operation, $installId, 'raised ' . $e::class . ': ' . $e->getMessage());
        }
        return self::handlerFailed();
    }

    /** The operation, the install and the reason: never the body, a secret or a signature. */
    private static function log(string $operation, string $installId, string $reason): void
    {
        error_log("lingara-apps: {$operation} for install {$installId} failed: {$reason}");
    }

    private static function badRequest(): CoreResponse
    {
        return CoreResponse::json(400, '{"error":"bad_request"}');
    }

    private static function handlerFailed(): CoreResponse
    {
        return CoreResponse::json(500, '{"error":"handler_failed"}');
    }
}
