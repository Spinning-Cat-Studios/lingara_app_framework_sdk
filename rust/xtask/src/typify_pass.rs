//! The typify pass (ADR 30.9.26am D6): `models.rs` from every component,
//! with the unions replaced by `unions.rs`'s enums and the arms' tag
//! properties removed; `unions::relax` then drops `deny_unknown_fields` from
//! the request side.

use std::collections::BTreeSet;

use serde_json::{Map, Value};
use typify::{TypeSpace, TypeSpaceSettings};

use crate::view::View;

/// The `format`s typify would map to `uuid` and `chrono` types. The kit keeps
/// the relay's strings, as the library does.
const STRING_FORMATS: [&str; 2] = ["date-time", "uuid"];

pub(crate) fn models(view: &Value, read: &View) -> Result<String, String> {
    let schemas = view
        .pointer("/components/schemas")
        .and_then(Value::as_object)
        .ok_or("the view has no components.schemas")?;
    let mut defs = Vec::new();
    for (name, schema) in schemas {
        let mut schema = schema.clone();
        if let Some(tag) = read.tag_of_arm(name) {
            strip_tag(&mut schema, tag);
        }
        strip_string_formats(&mut schema);
        let schema = serde_json::from_value(schema).map_err(|e| format!("schema {name}: {e}"))?;
        defs.push((name.clone(), schema));
    }
    let mut settings = TypeSpaceSettings::default();
    settings.with_struct_builder(false);
    for name in read.union_names() {
        settings.with_replacement(name, format!("super::unions::{name}"), std::iter::empty());
    }
    let mut space = TypeSpace::new(&settings);
    space.add_ref_types(defs).map_err(|e| format!("typify: {e}"))?;
    let mut file: syn::File = syn::parse2(space.to_stream()).map_err(|e| format!("typify's output does not parse: {e}"))?;
    refuse_written_names(&file, &read.written_names())?;
    crate::unions::relax(&mut file, &read.request_side(schemas));
    Ok(prettyplease::unparse(&file))
}

/// The arm's tag property belongs to the union's `#[serde(tag = …)]`: left
/// in, serde would read it twice and write it twice.
fn strip_tag(schema: &mut Value, tag: &str) {
    if let Some(properties) = schema.get_mut("properties").and_then(Value::as_object_mut) {
        properties.remove(tag);
    }
    if let Some(required) = schema.get_mut("required").and_then(Value::as_array_mut) {
        required.retain(|r| r != tag);
    }
}

fn strip_string_formats(value: &mut Value) {
    match value {
        Value::Object(map) => strip_in_object(map),
        Value::Array(items) => items.iter_mut().for_each(strip_string_formats),
        _ => {}
    }
}

fn strip_in_object(map: &mut Map<String, Value>) {
    if map.get("format").and_then(Value::as_str).is_some_and(|f| STRING_FORMATS.contains(&f)) {
        map.remove("format");
    }
    map.values_mut().for_each(strip_string_formats);
}

/// The name of every type typify defined.
pub(crate) fn defined_names(file: &syn::File) -> Vec<String> {
    file.items
        .iter()
        .filter_map(|item| match item {
            syn::Item::Struct(s) => Some(s.ident.to_string()),
            syn::Item::Enum(e) => Some(e.ident.to_string()),
            syn::Item::Type(t) => Some(t.ident.to_string()),
            _ => None,
        })
        .collect()
}

/// A generator-written type named after a union, a tag enum or the
/// operations would ship two types for one name, so codegen fails instead.
fn refuse_written_names(file: &syn::File, written: &BTreeSet<String>) -> Result<(), String> {
    match defined_names(file).into_iter().find(|name| written.contains(name)) {
        Some(name) => Err(format!("typify wrote {name}, a name unions.rs writes; the view must rename that component")),
        None => Ok(()),
    }
}
