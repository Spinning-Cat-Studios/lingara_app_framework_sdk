<?php

declare(strict_types=1);

namespace LingaraAppsSnippets;

use Lingara\Apps\Generated\AppRenderRequest;
// lingara:begin context
use Lingara\Apps\Generated\ContextSliceLanguages;
use Lingara\Apps\Generated\ContextSlicePlanSummary;
use Lingara\Client;
// lingara:end

// lingara:begin context
function planLine(AppRenderRequest $request): string
{
    // The app's own client-credentials client: it speaks for the app's owner.
    $client = new Client(
        clientId: getenv('LINGARA_CLIENT_ID') ?: null,
        clientSecret: getenv('LINGARA_CLIENT_SECRET') ?: null,
    );
    $line = '';
    // A slice is present only when the learner agreed to share it.
    foreach ($request->getContext() as $slice) {
        if ($slice instanceof ContextSliceLanguages) {
            $line .= "{$slice->sourceLang} → {$slice->targetLang}. ";
        } elseif ($slice instanceof ContextSlicePlanSummary) {
            $plan = $client->getLessonPlan($slice->planId)->value;
            $line .= "{$plan->getTitle()}: {$slice->setsCompleted}/{$slice->setCount} sets.";
        }
    }
    return $line;
}
// lingara:end
