<?php

declare(strict_types=1);

namespace LingaraAppsSnippets;

use Lingara\Apps\Generated\AppRenderRequest;
use Lingara\Apps\Generated\Card as CardModel;
// lingara:begin tutorNote
use Lingara\Apps\Reply;
// lingara:end

// lingara:begin tutorNote
function withNote(AppRenderRequest $request): CardModel|Reply
{
    $note = planLine($request);
    // Plain text, at most 280 characters; the tutor reads it, the learner does not.
    return $note === '' ? todayCard(0) : Reply::of(todayCard(0))->tutorNote($note);
}
// lingara:end
