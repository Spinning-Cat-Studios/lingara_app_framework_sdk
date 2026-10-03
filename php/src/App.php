<?php

declare(strict_types=1);

namespace Lingara\Apps;

use Lingara\Apps\Generated\AppActionRequest;
use Lingara\Apps\Generated\AppRenderRequest;
use Lingara\Apps\Generated\Card as CardModel;
use Lingara\Events\Webhook;

/**
 * An app: the library's verifier, built once, a render function and zero or
 * more action functions keyed on the button `action` they come back as.
 *
 *     $app = new App(
 *         secret: getenv('LINGARA_APP_SECRET'),       // lgr_whsec_…; a list of two during a rotation
 *         render: fn(AppRenderRequest $request) => Card::create()->heading('Today', 1)->build(),
 *         actions: ['done' => fn(AppActionRequest $request) => Card::create()->text('Done')->build()],
 *     );
 *
 * A function returns a built card, or Reply::of($card)->tutorNote($text).
 */
final class App
{
    private readonly Webhook $webhook;
    /** @var \Closure(AppRenderRequest): (CardModel|Reply) */
    private readonly \Closure $render;
    /** @var array<string, \Closure(AppActionRequest): (CardModel|Reply)> */
    private readonly array $actions;

    /**
     * The library's Webhook is constructed here, once, so a malformed secret
     * fails at startup rather than on the first request.
     *
     * @param string|list<string>                                       $secret  the app's signing secret, or each live one during a rotation
     * @param callable(AppRenderRequest): (CardModel|Reply)             $render
     * @param array<string, callable(AppActionRequest): (CardModel|Reply)> $actions one function per button `action`
     *
     * @throws \InvalidArgumentException for a secret that is not `lgr_whsec_` and padded base64
     */
    public function __construct(
        #[\SensitiveParameter]
        string|array $secret,
        callable $render,
        array $actions = [],
    ) {
        $this->webhook = new Webhook($secret);
        $this->render = $render(...);
        $this->actions = array_map(static fn(callable $fn): \Closure => $fn(...), $actions);
    }

    /**
     * The signature check, the library's own.
     *
     * @internal
     *
     * @param array<string, string|list<string>>|\Psr\Http\Message\MessageInterface $headers
     *
     * @throws \Lingara\Events\VerificationException
     */
    public function verifySignature(string $body, array|\Psr\Http\Message\MessageInterface $headers): void
    {
        $this->webhook->verifySignature($body, $headers);
    }

    /** @internal */
    public function render(AppRenderRequest $request): CardModel|Reply
    {
        return ($this->render)($request);
    }

    /**
     * The function registered for an action's `action_id`, or null.
     *
     * @internal
     *
     * @return (\Closure(AppActionRequest): (CardModel|Reply))|null
     */
    public function action(string $actionId): ?\Closure
    {
        return $this->actions[$actionId] ?? null;
    }

    /** @return array<string, mixed> */
    public function __debugInfo(): array
    {
        return ['actions' => array_keys($this->actions)];
    }
}
