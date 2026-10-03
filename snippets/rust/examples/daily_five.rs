//! A complete app: its manifest, a render function and one action keyed on
//! the learner, a card, the context slices, a tutor note, and the server.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use lingara_apps::lingara::Client;
use lingara_apps::{
    App, AppActionRequest, AppRenderRequest, AppSlotName, BoxError, ContextSlice, ContextSliceKind, Reply, Term, card, item, manifest, reply,
};

/// Words reviewed per learner, keyed on `subject`.
type Seen = Arc<Mutex<HashMap<String, u32>>>;

// lingara:begin context
/// The learner's target language and the title of the plan they shared, read
/// from the context slices. The plan itself comes from the library: the
/// app's own client-credentials client speaks for the app's owner; a plan of
/// any other learner needs that learner's token.
async fn shared_plan(client: &Client, request: &AppRenderRequest) -> Result<(String, Option<String>), BoxError> {
    let (mut target_lang, mut plan_id) = (String::from("zh"), None);
    for slice in &request.context {
        match slice {
            ContextSlice::Languages(languages) => target_lang = languages.target_lang.clone(),
            ContextSlice::PlanSummary(summary) => plan_id = Some(summary.plan_id.clone()),
            _ => {}
        }
    }
    let title = match plan_id {
        Some(id) => client.get_lesson_plan(&id).await?.title.clone(),
        None => None,
    };
    Ok((target_lang, title))
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
async fn render(client: Client, seen: Seen, request: AppRenderRequest) -> Result<Reply, BoxError> {
    let (lang, plan) = shared_plan(&client, &request).await?;
    let count = seen.lock().map_err(|_| "poisoned")?.get(&request.subject).copied().unwrap_or(0);
    let card = today_card(plan.as_deref().unwrap_or("Today's five"), &lang)?;
    // Plain text the learner's tutor can read: at most 280 characters.
    Ok(reply(card).tutor_note(format!("The learner has reviewed {count} words with this app today."))?)
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
        .scopes(["plans:read"])
        .tutor_note(true)
        .build()?;
    std::fs::write("manifest.json", manifest.to_json())?;
    // lingara:end

    // lingara:begin handler
    // The app's signing secret (lgr_whsec_…) and its API credentials, from
    // the environment.
    let client = Client::builder()
        .client_credentials(std::env::var("LINGARA_CLIENT_ID")?, std::env::var("LINGARA_CLIENT_SECRET")?)
        .build()?;
    let seen = Seen::default();
    let (render_seen, action_seen) = (Arc::clone(&seen), Arc::clone(&seen));
    let app = App::new([std::env::var("LINGARA_APP_SECRET")?], move |request: AppRenderRequest| {
        render(client.clone(), Arc::clone(&render_seen), request)
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
