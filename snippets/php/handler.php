<?php

declare(strict_types=1);

namespace LingaraAppsSnippets;

use Lingara\Apps\Generated\Card as CardModel;
use Lingara\Apps\Generated\AppSlotName;
// lingara:begin handler
use Lingara\Apps\App;
use Lingara\Apps\Generated\AppActionRequest;
use Lingara\Apps\Generated\AppRenderRequest;
// lingara:end

/** @param array<string, int> $streaks the app's own per-learner store */
function app(array &$streaks): App
{
    // lingara:begin handler
    $app = new App(
        // lgr_whsec_…, from the app's settings; pass a list of two during a rotation.
        secret: (string) getenv('LINGARA_APP_SECRET'),
        // Hold per-learner state on `subject`, the same value your events carry.
        render: static fn(AppRenderRequest $request) => $request->getSlot() === AppSlotName::HOME_SIDE
            ? withNote($request)
            : todayCard($streaks[$request->getSubject()] ?? 0),
        actions: [
            // The button's `action`; may arrive twice, so make it safe to repeat.
            'done' => static function (AppActionRequest $request) use (&$streaks): CardModel {
                $streaks[$request->getSubject()] = ($streaks[$request->getSubject()] ?? 0) + 1;
                return todayCard($streaks[$request->getSubject()]);
            },
        ],
    );
    // lingara:end
    return $app;
}
