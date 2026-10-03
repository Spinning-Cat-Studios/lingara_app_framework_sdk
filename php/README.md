# spinningcatstudios/lingara-apps

The official Lingara app kit for PHP. An app is a server that answers two
signed requests from Lingara, `app.render` and `app.action`, with a card.
This package verifies each request, decodes it, calls your function and
sends back the card, refusing at build time anything Lingara would otherwise
cut at run time.

It depends on [`spinningcatstudios/lingara`](https://packagist.org/packages/spinningcatstudios/lingara),
whose signature verifier it reuses, and on two PSR interface packages:
`psr/http-server-handler` and `psr/http-factory`. Bring any PSR-7
implementation with PSR-17 factories (`nyholm/psr7`, `guzzlehttp/psr7`, …).
PHP 8.2 or newer.

## Install

```sh
composer require spinningcatstudios/lingara-apps nyholm/psr7 nyholm/psr7-server
```

While the kit and the library are pre-releases, a project on
`minimum-stability: stable` asks for them explicitly:
`composer require "spinningcatstudios/lingara-apps:@alpha" "spinningcatstudios/lingara:@alpha"`.

## Quick start

```php
use Lingara\Apps\App;
use Lingara\Apps\Card;
use Lingara\Apps\Generated\AppActionRequest;
use Lingara\Apps\Generated\AppRenderRequest;
use Lingara\Apps\Generated\AppSlotName;
use Lingara\Apps\Psr15Handler;
use Lingara\Apps\Reply;
use Nyholm\Psr7\Factory\Psr17Factory;

$app = new App(
    // The app's signing secret, lgr_whsec_…, from its settings. Pass a list of
    // two during a rotation. A malformed secret throws here, at startup.
    secret: (string) getenv('LINGARA_APP_SECRET'),
    render: function (AppRenderRequest $request) {
        $today = Card::create()
            ->heading("Today's five", 1)
            ->term('雨', reading: 'yǔ', gloss: 'rain', lang: 'zh')
            ->button('Done', 'done')
            ->build();
        return $request->getSlot() === AppSlotName::HOME_SIDE
            ? Reply::of($today)->tutorNote('The learner is reviewing weather words.')
            : $today;
    },
    actions: [
        'done' => fn(AppActionRequest $request) => Card::create()->text('Well done')->build(),
    ],
);

$factory = new Psr17Factory();
$handler = new Psr15Handler($app, $factory, $factory); // a PSR-15 RequestHandlerInterface
```

- **`render`** receives the decoded request (`AppRenderRequest`): `getId()`,
  `getInstallId()`, `getSubject()` (the per-learner key your events also
  carry: hold state on it), `getSlot()`, `getLocale()` and `getContext()`, a
  list of the slices the learner agreed to share (`ContextSliceLanguages`,
  `ContextSlicePlanSummary`, …). A slice of a kind this version does not know
  is skipped.
- **`actions`** are keyed on a button's `action`, which comes back as the
  request's `getActionId()`. An action also carries `getCardEtag()`, an opaque
  token naming the card the learner pressed; it is passed to you unchanged.
  Lingara may deliver an action twice, so make each one safe to receive twice.
- **A function returns a card** (`Card::create()->…->build()`), or
  `Reply::of($card)->tutorNote($text)` to add a plain-text note for the tutor.
  Throwing, or returning a card that breaks a rule, answers
  `500 {"error":"handler_failed"}` and is logged through `error_log()` with
  the operation, the install id and the reason, never the body or a secret.
- **The builders refuse.** `build()` throws `CardLimitException` with the rule
  in `$e->reason` (`text_length`, `buttons`, …), and the reply is checked
  again, encoded, before it is sent (at most 32 768 bytes).
  `Limits::truncate($text, $limit)` gives you Lingara's own cut when you want
  a shorter string.

## The manifest

```php
use Lingara\Apps\Generated\AppSlotName;
use Lingara\Apps\Manifest;

$json = Manifest::create()->defaultLocale('en')->name('Daily five')->description('Five words to review.')
    ->renderUrl('https://apps.example.com/lingara/render')->slots(AppSlotName::HOME_SIDE)
    ->context('languages', 'plan_summary')
    ->toJson(); // Throws ManifestException naming the rule an upload would refuse.
```

Upload the result in the console; the icon is uploaded there too.

## Context and your plan data

The context is read-only. To read the plan behind a `ContextSlicePlanSummary`
slice, call `getLessonPlan($slice->planId)` on a `Lingara\Client`. For a
private app, use the app's own client-credentials client, which speaks for
its owner; for any other learner, it needs that learner's O5 token.

## Frameworks: the raw body

The signature covers the bytes Lingara sent, so the kit reads the request's
body stream itself, from the start, before anything parses it. Mount
`Psr15Handler` on the route Lingara posts to, ahead of any body-parsing
middleware; a parsed body (`getParsedBody()`) is never read. Without a
framework, a front controller builds the request from PHP's globals:

```php
$request = (new Nyholm\Psr7Server\ServerRequestCreator($factory, $factory, $factory, $factory))->fromGlobals();
$response = $handler->handle($request);
```

Anything else can call the framework-neutral core,
`Core::handle($app, $method, $headers, fn(int $limit): string => …)`.

## What Lingara sends and expects

- Requests carry `user-agent: Lingara-Apps/1 (+https://getlingara.com/docs/apps/)`.
  Use it to recognise Lingara in your logs; it is unsigned, so it is never a gate.
- Lingara waits **3 s** for a render and **5 s** for an action. A slow render
  shows the learner a stale or fallback card. The kit enforces no time limit.
- Events (`app.installed`, `app.uninstalled`, …) are not app requests: verify
  them with the library's own `Lingara\Events\Webhook::verify` on your
  webhook endpoint.

This kit keeps the [app kit contract](../conformance/CONTRACT.md).
