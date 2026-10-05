<?php

declare(strict_types=1);

namespace LingaraAppsSnippets;

// lingara:begin manifest
use Lingara\Apps\Generated\AppSlotName;
use Lingara\Apps\Manifest;
// lingara:end

function manifest(): void
{
    // lingara:begin manifest
    $json = Manifest::create()
        ->defaultLocale('en')
        ->name(['en' => 'Daily five', 'zh-Hans' => '每日五词'])
        ->description('Five words to review, picked from your plan.')
        ->renderUrl('https://apps.example.com/lingara/render')
        ->slots(AppSlotName::HOME_SIDE, AppSlotName::PLANS_EMPTY_DETAIL)
        ->context('languages', 'plan_summary')
        // `->scopes(…)` lists the API scopes your client uses, for the learner's consent page. This app uses none.
        ->tutorNote()
        ->toJson(); // Throws ManifestException naming the rule an upload would refuse.

    file_put_contents('manifest.json', $json); // Upload it in the console.
    // lingara:end
}
