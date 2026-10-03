<?php

declare(strict_types=1);

// The PHP kit's fixture app (ADR 30.9.26al §F, 30.9.26am D9): the front
// controller `php -S` runs for every request, started by serve.php. It is
// built on the kit's public API and adapter only. LINGARA_APPS_PSR7 chooses
// the PSR-7 implementation the request is built with and the PSR-17
// factories the handler answers with: `nyholm` (nyholm/psr7 and
// nyholm/psr7-server) or `guzzle` (guzzlehttp/psr7).

require __DIR__ . '/../vendor/autoload.php';

use GuzzleHttp\Psr7\HttpFactory;
use GuzzleHttp\Psr7\ServerRequest;
use Lingara\Apps\App;
use Lingara\Apps\Card;
use Lingara\Apps\Generated\AppActionRequest;
use Lingara\Apps\Generated\AppRenderRequest;
use Lingara\Apps\Generated\AppSlotName;
use Lingara\Apps\Generated\Card as CardModel;
use Lingara\Apps\Generated\ContextSlice;
use Lingara\Apps\Item;
use Lingara\Apps\Psr15Handler;
use Lingara\Apps\Reply;
use Nyholm\Psr7\Factory\Psr17Factory;
use Nyholm\Psr7Server\ServerRequestCreator;
use Psr\Http\Message\ResponseInterface;

// `php -S` labels a response with no content-type `text/html`; the kit's
// empty 401, 405 and 413 carry none.
ini_set('default_mimetype', '');

function render(AppRenderRequest $request): CardModel|Reply
{
    $card = Card::create()->heading($request->getSlot()->value, 1);
    foreach ($request->getContext() as $slice) {
        $card->text((string) array_search($slice::class, ContextSlice::ARMS, true));
    }
    $built = $card->build();
    return $request->getSlot() === AppSlotName::HOME_SIDE ? Reply::of($built)->tutorNote('fixture note') : $built;
}

function fixtureApp(): App
{
    $secrets = array_values(array_filter(explode(',', (string) getenv('LINGARA_APPS_CONFORMANCE_SECRETS'))));
    return new App(
        secret: $secrets,
        render: render(...),
        actions: [
            'inc' => static fn(AppActionRequest $r): CardModel => Card::create()->progress(0.5, 'inc')->text($r->getCardEtag())->build(),
            'boom' => static fn(): never => throw new RuntimeException('boom'),
            'overflow' => static fn(): CardModel => Card::create()
                ->list(array_map(static fn(int $n): Lingara\Apps\Generated\ListItemText => Item::text((string) $n), range(1, 21)))
                ->build(),
            'huge' => static function (): CardModel {
                $card = Card::create();
                for ($i = 0; $i < 24; $i++) {
                    $card->text(str_repeat('漢', 600));
                }
                return $card->build();
            },
        ],
    );
}

function emit(ResponseInterface $response): void
{
    http_response_code($response->getStatusCode());
    foreach ($response->getHeaders() as $name => $values) {
        foreach ($values as $value) {
            header("{$name}: {$value}", false);
        }
    }
    echo $response->getBody();
}

$variant = getenv('LINGARA_APPS_PSR7');
if ($variant === 'guzzle') {
    $factory = new HttpFactory();
    $request = ServerRequest::fromGlobals();
} else {
    $factory = new Psr17Factory();
    $request = (new ServerRequestCreator($factory, $factory, $factory, $factory))->fromGlobals();
}
emit((new Psr15Handler(fixtureApp(), $factory, $factory))->handle($request));
