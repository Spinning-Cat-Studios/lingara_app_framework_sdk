//! `check-coverage` (ADR 30.9.26al D6): the cases cover the view and the
//! contract. It fails when an app operation has no case, when a case names a
//! message the view lacks, when one of AK1–AK5 has no case, when a case does
//! not load or its id does not match its path, and when a `json` body that
//! expects a `200` is not a request the relay could send.

use serde_json::Value;

use crate::case::{BEHAVIOURS, Body, Loaded};
use crate::validate::Schemas;

/// A placeholder `id` for schema checks: the shape the relay mints.
const PLACEHOLDER_ID: &str = "lgr_msg_00000000000000000000000000000000";

/// Every gap, in a stable order; empty when the cases cover everything.
pub fn gaps(view: &Value, loaded: &[Loaded], load_errors: &[String]) -> Result<Vec<String>, String> {
    let schemas = Schemas::from_view(view)?;
    let messages: Vec<&str> =
        view["x-lingara-app-operations"].as_array().into_iter().flatten().filter_map(|o| o["message"].as_str()).collect();
    let mut out: Vec<String> = load_errors.iter().map(|e| format!("does not load: {e}")).collect();
    for message in &messages {
        if !loaded.iter().any(|l| l.case.request.operation.as_deref() == Some(*message)) {
            out.push(format!("no case exercises the app operation {message}"));
        }
    }
    for behaviour in BEHAVIOURS {
        if !loaded.iter().any(|l| l.case.behaviours.contains(&behaviour)) {
            out.push(format!("no case covers {behaviour:?}"));
        }
    }
    for l in loaded {
        out.extend(case_gaps(l, &messages, &schemas));
    }
    Ok(out)
}

fn case_gaps(l: &Loaded, messages: &[&str], schemas: &Schemas) -> Vec<String> {
    let case = &l.case;
    let mut out = Vec::new();
    if case.id != l.path_id() {
        out.push(format!("{}: the id does not match its path ({})", case.id, l.path_id()));
    }
    if let Some(op) = &case.request.operation
        && !messages.contains(&op.as_str())
    {
        out.push(format!("{}: names the message {op}, which the view lacks", case.id));
    }
    let Body::Json(json) = &case.request.body else { return out };
    let Some(op) = &case.request.operation else {
        out.push(format!("{}: a json body needs request.operation", case.id));
        return out;
    };
    if case.expect.status == 200 {
        let mut body = json.clone();
        body["type"] = Value::from(op.as_str());
        body["id"] = Value::from(PLACEHOLDER_ID);
        let valid = schemas.request.get(op).is_some_and(|s| s.is_valid(&body));
        if !valid {
            out.push(format!("{}: the json body is not a valid {op} request", case.id));
        }
    }
    out
}
