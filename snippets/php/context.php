<?php

declare(strict_types=1);

namespace LingaraAppsSnippets;

use Lingara\Apps\Generated\AppRenderRequest;
// lingara:begin context
use Lingara\Apps\Generated\ContextSliceLanguages;
use Lingara\Apps\Generated\ContextSlicePlanSummary;
// lingara:end

// lingara:begin context
function planLine(AppRenderRequest $request): string
{
    // The app's own client reads its owner's account, never the learner's. The
    // slices are all a render knows about the learner.
    $line = '';
    // A slice is present only when the learner agreed to share it.
    foreach ($request->getContext() as $slice) {
        if ($slice instanceof ContextSliceLanguages) {
            $line .= "{$slice->sourceLang} → {$slice->targetLang}. ";
        } elseif ($slice instanceof ContextSlicePlanSummary) {
            $line .= "{$slice->setsCompleted}/{$slice->setCount} sets.";
        }
    }
    return $line;
}
// lingara:end
