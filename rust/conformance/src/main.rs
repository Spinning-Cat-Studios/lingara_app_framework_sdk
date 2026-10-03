//! The fixture app (CONTRACT.md §F; ADR 30.9.26am D9), on the kit's public
//! API and its axum adapter only. Every reply below is exact: the host judges
//! it JSON-equal.

use std::io::Write as _;

use lingara_apps::{App, AppActionRequest, AppRenderRequest, AppSlotName, BoxError, Reply, card, item, reply};

/// A heading naming the slot, then one text per slice received, in order;
/// on `home.side`, a tutor note too.
fn render(request: AppRenderRequest) -> Result<Reply, BoxError> {
    let card = request.context.iter().fold(card().heading(request.slot.to_string(), 1), |card, slice| card.text(slice.tag().as_str()));
    let reply = reply(card.build()?);
    Ok(if request.slot == AppSlotName::HomeSide { reply.tutor_note("fixture note")? } else { reply })
}

fn inc(action: AppActionRequest) -> Result<Reply, BoxError> {
    Ok(card().progress(0.5, "inc").text(action.card_etag).build()?.into())
}

/// Built with the public builder, so `build` refuses it (`list_items`).
fn overflow(_: AppActionRequest) -> Result<Reply, BoxError> {
    Ok(card().list((1..=21).map(|n| item::text(n.to_string()))).build()?.into())
}

/// Passes every card rule; the encoded reply is over 32 768 bytes.
fn huge(_: AppActionRequest) -> Result<Reply, BoxError> {
    Ok((0..24).fold(card(), |card, _| card.text("漢".repeat(600))).build()?.into())
}

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    let secrets = std::env::var("LINGARA_APPS_CONFORMANCE_SECRETS")?;
    let port: u16 = std::env::var("LINGARA_APPS_CONFORMANCE_PORT").unwrap_or_else(|_| "0".into()).parse()?;
    let app = App::new(secrets.split(','), |r| async move { render(r) })?
        .action("inc", |a| async move { inc(a) })
        .action("boom", |_| async { Err::<Reply, BoxError>("boom".into()) })
        .action("overflow", |a| async move { overflow(a) })
        .action("huge", |a| async move { huge(a) });
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port)).await?;
    println!("listening {}", listener.local_addr()?.port());
    std::io::stdout().flush()?;
    axum::serve(listener, lingara_apps::axum::router(app)).await?;
    Ok(())
}
