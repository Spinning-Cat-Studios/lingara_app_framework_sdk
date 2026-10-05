//! A complete app: its manifest, a render function and one action keyed on
//! the learner, a card, the context slices, a tutor note, and the server.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use lingara_apps::{
    App, AppActionRequest, AppRenderRequest, AppSlotName, BoxError, ContextSlice, ContextSliceKind, Reply, Term, card, item, manifest, reply,
};

/// Words reviewed per learner, keyed on `subject`.
type Seen = Arc<Mutex<HashMap<String, u32>>>;

// lingara:begin context
// The app's own client reads its owner's account, never the learner's. The
// slices are all a render knows about the learner.
/// The learner's target language and, when they shared their plan, its
/// `(sets_completed, set_count)`.
fn shared_plan(request: &AppRenderRequest) -> (String, Option<(u32, u32)>) {
    let (mut target_lang, mut progress) = (String::from("zh"), None);
    for slice in &request.context {
        match slice {
            ContextSlice::Languages(languages) => target_lang = languages.target_lang.clone(),
            ContextSlice::PlanSummary(summary) => progress = Some((summary.sets_completed, summary.set_count)),
            _ => {}
        }
    }
    (target_lang, progress)
}
// lingara:end

// lingara:begin card
fn today_card(title: &str, lang: &str) -> Result<lingara_apps::Card, BoxError> {
    Ok(card()
        .heading(title, 1)
        .term(Term::new("雨").reading("yǔ").gloss("rain").lang(lang))
        .list([item::text("Say it aloud"), item::term(Term::new("下雨").gloss("to rain"))])
        .button("Next word", "next")
        .build()?)
}
// lingara:end

// lingara:begin tutorNote
async fn render(seen: Seen, request: AppRenderRequest) -> Result<Reply, BoxError> {
    let (lang, progress) = shared_plan(&request);
    let count = seen.lock().map_err(|_| "poisoned")?.get(&request.subject).copied().unwrap_or(0);
    let card = today_card("Today's five", &lang)?;
    // Plain text the learner's tutor can read: at most 280 characters.
    let note = match progress {
        Some((done, total)) => format!("The learner has reviewed {count} words with this app today; {done} of {total} sets done."),
        None => format!("The learner has reviewed {count} words with this app today."),
    };
    Ok(reply(card).tutor_note(note)?)
}
// lingara:end

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    // lingara:begin manifest
    let manifest = manifest()
        .default_locale("en")
        .name("Daily five")
        .description("Five words to review, picked from your plan.")
        .render_url("https://apps.example.com/lingara/render")
        .slots([AppSlotName::HomeSide, AppSlotName::PlansEmptyDetail])
        .context([ContextSliceKind::Languages, ContextSliceKind::PlanSummary])
        // `.scopes(…)` lists the API scopes your client uses, for the learner's consent page. This app uses none.
        .tutor_note(true)
        .build()?;
    std::fs::write("manifest.json", manifest.to_json())?;
    // lingara:end

    // lingara:begin handler
    // The app's signing secret (lgr_whsec_…), from the environment.
    let seen = Seen::default();
    let (render_seen, action_seen) = (Arc::clone(&seen), Arc::clone(&seen));
    let app = App::new([std::env::var("LINGARA_APP_SECRET")?], move |request: AppRenderRequest| {
        render(Arc::clone(&render_seen), request)
    })?
    // The button's `action` comes back as `action_id`. An action can arrive
    // twice, so keep it safe to repeat.
    .action("next", move |request: AppActionRequest| {
        let seen = Arc::clone(&action_seen);
        async move {
            *seen.lock().map_err(|_| "poisoned")?.entry(request.subject).or_default() += 1;
            today_card("Next word", "zh")
        }
    });
    // lingara:end

    // lingara:begin serve
    // Nest the adapter at the path your manifest's render_url names.
    let routes = axum::Router::new().nest("/lingara/render", lingara_apps::axum::router(app));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
    axum::serve(listener, routes).await?;
    // lingara:end
    Ok(())
}
