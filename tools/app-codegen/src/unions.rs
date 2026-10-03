//! Tagged unions (ADR 30.9.26al D4). Every `oneOf` in the view must be a
//! component's whole body and a **tagged** union: each member an object
//! whose tag property is a one-value string `enum` (a `const`, after
//! `reach`), the same property in every member, with distinct values. An
//! inline member is lifted into `components.schemas` as `<Union><Tag>`
//! (`plan_summary` → `ContextSlicePlanSummary`); a `$ref` member is used as
//! it is. The union's component becomes `{type: object}` and the union is
//! recorded in `x-lingara-unions`, so each kit's emitter writes the sealed
//! type from the record, as the client libraries' stream emitters do.
//!
//! The only other composition allowed is the nullable
//! `anyOf: [{$ref}, {type: 'null'}]`, which the 3.0 dialect rewrites.

use serde_json::{Map, Value, json};
use spec_codegen::walk::escape;

use crate::{Refusal, refuse, schema_ref};

pub fn lift(schemas: &mut Map<String, Value>) -> Result<Vec<Value>, Refusal> {
    for (name, schema) in schemas.iter() {
        let ptr = format!("/components/schemas/{}", escape(name));
        check_compositions(schema, &ptr, true)?;
    }
    let mut out = Map::new();
    let mut records = Vec::new();
    for (name, schema) in schemas.iter() {
        let Some(members) = schema.get("oneOf") else {
            out.insert(name.clone(), schema.clone());
            continue;
        };
        let union = lift_one(name, schema, members, schemas)?;
        out.insert(name.clone(), json!({ "type": "object" }));
        for (arm, body) in union.lifted {
            if schemas.contains_key(&arm) || out.contains_key(&arm) {
                return refuse(&format!("/components/schemas/{}", escape(&arm)), "a lifted arm collides with a component");
            }
            out.insert(arm, body);
        }
        records.push(union.record);
    }
    *schemas = out;
    Ok(records)
}

/// A `oneOf` only as a component's root; an `anyOf` only in the nullable
/// shape; both anywhere below are walked.
fn check_compositions(schema: &Value, ptr: &str, root: bool) -> Result<(), Refusal> {
    let Some(obj) = schema.as_object() else { return Ok(()) };
    if obj.contains_key("oneOf") && !root {
        return refuse(&format!("{ptr}/oneOf"), "a oneOf is allowed only as a component's whole body");
    }
    if obj.get("anyOf").is_some_and(|a| !is_nullable_ref(a)) {
        return refuse(&format!("{ptr}/anyOf"), "only the nullable anyOf: [{$ref}, {type: 'null'}]");
    }
    for (key, value) in obj {
        let at = format!("{ptr}/{}", escape(key));
        match (key.as_str(), value) {
            ("properties", Value::Object(map)) => {
                for (name, s) in map {
                    check_compositions(s, &format!("{at}/{}", escape(name)), false)?;
                }
            }
            ("items" | "additionalProperties" | "not", s) => check_compositions(s, &at, false)?,
            ("oneOf" | "anyOf" | "allOf", Value::Array(list)) => {
                for (i, s) in list.iter().enumerate() {
                    check_compositions(s, &format!("{at}/{i}"), false)?;
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn is_nullable_ref(any_of: &Value) -> bool {
    let Some([a, b]) = any_of.as_array().map(Vec::as_slice) else { return false };
    let is_ref = |v: &Value| v.as_object().is_some_and(|o| o.len() == 1 && o.contains_key("$ref"));
    let is_null = |v: &Value| v == &json!({ "type": "null" });
    (is_ref(a) && is_null(b)) || (is_null(a) && is_ref(b))
}

struct Lifted {
    record: Value,
    lifted: Vec<(String, Value)>,
}

/// One member: its tag candidates, and the schema the arm records.
struct Member {
    candidates: Vec<(String, String)>,
    arm: Arm,
}

enum Arm {
    Inline(Value),
    Ref(String),
}

fn lift_one(union: &str, schema: &Value, members: &Value, schemas: &Map<String, Value>) -> Result<Lifted, Refusal> {
    let ptr = format!("/components/schemas/{}", escape(union));
    if schema.as_object().is_some_and(|o| o.len() > 1) {
        return refuse(&ptr, "a union's component holds its oneOf and nothing else");
    }
    let list = members.as_array().filter(|l| !l.is_empty());
    let list = list.ok_or_else(|| Refusal(format!("{ptr}/oneOf: an empty union")))?;
    let members = list
        .iter()
        .enumerate()
        .map(|(i, m)| member(m, &format!("{ptr}/oneOf/{i}"), schemas))
        .collect::<Result<Vec<_>, _>>()?;
    let tag = shared_tag(&members, &ptr)?;
    let mut arms = Vec::new();
    let mut lifted = Vec::new();
    for m in members {
        let value = m.candidates.into_iter().find(|(p, _)| *p == tag).map(|(_, v)| v).expect("shared_tag checked");
        if arms.iter().any(|a: &Value| a["value"] == value.as_str()) {
            return refuse(&format!("{ptr}/oneOf"), &format!("the tag value {value} repeats"));
        }
        let schema = match m.arm {
            Arm::Ref(target) => schema_ref(&target),
            Arm::Inline(body) => {
                let name = format!("{union}{}", pascal(&value));
                lifted.push((name.clone(), body));
                schema_ref(&name)
            }
        };
        arms.push(json!({ "value": value, "schema": schema }));
    }
    let record = json!({ "name": union, "tag": tag, "arms": arms });
    Ok(Lifted { record, lifted })
}

fn member(m: &Value, ptr: &str, schemas: &Map<String, Value>) -> Result<Member, Refusal> {
    let target = m.as_object().filter(|o| o.len() == 1).and_then(|o| o.get("$ref")?.as_str());
    let (body, arm) = match target.and_then(|r| r.strip_prefix("#/components/schemas/")) {
        Some(name) => {
            let body = schemas.get(name).ok_or_else(|| Refusal(format!("{ptr}/$ref: {name} is not in the view")))?;
            (body, Arm::Ref(name.to_string()))
        }
        None => (m, Arm::Inline(m.clone())),
    };
    let props = body.get("properties").and_then(Value::as_object);
    let props = props.ok_or_else(|| Refusal(format!("{ptr}: a union member must be an object with properties")))?;
    let candidates = props
        .iter()
        .filter_map(|(p, s)| match s.get("enum")?.as_array()?.as_slice() {
            [Value::String(v)] => Some((p.clone(), v.clone())),
            _ => None,
        })
        .collect();
    Ok(Member { candidates, arm })
}

/// The one property every member tags with.
fn shared_tag(members: &[Member], ptr: &str) -> Result<String, Refusal> {
    let first: Vec<&String> = members[0].candidates.iter().map(|(p, _)| p).collect();
    let shared: Vec<&String> =
        first.into_iter().filter(|p| members.iter().all(|m| m.candidates.iter().any(|(q, _)| q == *p))).collect();
    match shared.as_slice() {
        [one] => Ok((*one).clone()),
        [] => refuse(&format!("{ptr}/oneOf"), "the members share no one-value tag property"),
        _ => refuse(&format!("{ptr}/oneOf"), "the members share more than one candidate tag property"),
    }
}

/// `plan_summary` → `PlanSummary`.
pub fn pascal(tag: &str) -> String {
    tag.split('_')
        .map(|part| {
            let mut chars = part.chars();
            chars.next().map(|c| c.to_ascii_uppercase().to_string() + chars.as_str()).unwrap_or_default()
        })
        .collect()
}
