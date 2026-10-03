//! The schemas the app payloads reach (ADR 30.9.26al D4), copied under their
//! own names and nothing else: an event schema no app payload reaches is
//! absent. Each copy drops the documentation keys and turns every `const`
//! into a one-value `enum`, because the 3.0 dialect refuses `const`.
//!
//! Output order is `components.schemas` document order, not discovery order,
//! so the view is byte-stable however the payloads list their references.

use std::collections::{BTreeMap, VecDeque};

use serde_json::{Map, Value, json};
use spec_codegen::walk::escape;

use crate::{Refusal, refuse};

/// Schema keys dropped from every copy. A property *named* `title` or
/// `description` is kept: property maps are walked, never filtered.
pub const DROPPED: [&str; 7] = ["title", "summary", "description", "tags", "externalDocs", "x-i18n", "examples"];

const REF_PREFIX: &str = "#/components/schemas/";

pub fn copy_reachable(doc: &Value, roots: &[String]) -> Result<Map<String, Value>, Refusal> {
    let all = doc.pointer("/components/schemas").and_then(Value::as_object);
    let all = all.ok_or_else(|| Refusal("/components/schemas: the catalogue has no schemas".into()))?;
    let mut copied: BTreeMap<String, Value> = BTreeMap::new();
    let mut queue: VecDeque<String> = roots.iter().cloned().collect();
    while let Some(name) = queue.pop_front() {
        if copied.contains_key(&name) {
            continue;
        }
        let ptr = format!("/components/schemas/{}", escape(&name));
        let schema = all.get(&name).ok_or_else(|| Refusal(format!("{ptr}: referenced, but not defined")))?;
        let mut refs = Vec::new();
        copied.insert(name, clean(schema, &ptr, &mut refs)?);
        queue.extend(refs);
    }
    Ok(all.keys().filter_map(|k| copied.remove_entry(k)).collect())
}

/// One schema, cleaned, with every `$ref` it holds pushed onto `refs`.
fn clean(schema: &Value, ptr: &str, refs: &mut Vec<String>) -> Result<Value, Refusal> {
    let Some(obj) = schema.as_object() else { return Ok(schema.clone()) };
    let mut out = Map::new();
    for (key, value) in obj {
        let at = format!("{ptr}/{}", escape(key));
        match key.as_str() {
            k if DROPPED.contains(&k) => {}
            "$defs" => return refuse(&at, "a named sub-schema: the catalogue changed shape"),
            "$ref" => {
                refs.push(internal_ref(value, &at)?);
                out.insert(key.clone(), value.clone());
            }
            "const" => {
                out.insert("enum".into(), json!([value]));
            }
            "properties" => {
                out.insert(key.clone(), clean_map(value, &at, refs)?);
            }
            "items" | "additionalProperties" | "not" => {
                out.insert(key.clone(), clean(value, &at, refs)?);
            }
            "oneOf" | "anyOf" | "allOf" => {
                out.insert(key.clone(), clean_list(value, &at, refs)?);
            }
            _ => {
                out.insert(key.clone(), value.clone());
            }
        }
    }
    Ok(Value::Object(out))
}

fn clean_map(value: &Value, ptr: &str, refs: &mut Vec<String>) -> Result<Value, Refusal> {
    let Some(map) = value.as_object() else { return refuse(ptr, "properties must be an object") };
    let mut out = Map::new();
    for (name, schema) in map {
        out.insert(name.clone(), clean(schema, &format!("{ptr}/{}", escape(name)), refs)?);
    }
    Ok(Value::Object(out))
}

fn clean_list(value: &Value, ptr: &str, refs: &mut Vec<String>) -> Result<Value, Refusal> {
    let Some(list) = value.as_array() else { return refuse(ptr, "must be an array of schemas") };
    let cleaned = list.iter().enumerate().map(|(i, s)| clean(s, &format!("{ptr}/{i}"), refs));
    cleaned.collect::<Result<Vec<_>, _>>().map(Value::Array)
}

/// The schema name a `$ref` names, when it is internal.
fn internal_ref(value: &Value, ptr: &str) -> Result<String, Refusal> {
    let name = value.as_str().and_then(|r| r.strip_prefix(REF_PREFIX)).filter(|n| !n.contains('/'));
    match name {
        Some(n) => Ok(n.replace("~1", "/").replace("~0", "~")),
        None => refuse(ptr, "not a #/components/schemas reference"),
    }
}
