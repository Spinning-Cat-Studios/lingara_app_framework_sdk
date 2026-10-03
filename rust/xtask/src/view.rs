//! The two extensions the emitter reads (ADR 30.9.26am D6):
//! `x-lingara-unions` (each union's name, tag and arms) and
//! `x-lingara-app-operations` (each request message, its request component
//! and the reply message), checked against `components.schemas`.

use std::collections::{BTreeSet, VecDeque};

use serde_json::{Map, Value};

use crate::unions::tag_enum_name;

const SCHEMA_REF: &str = "#/components/schemas/";

pub struct Union {
    pub(crate) name: String,
    pub(crate) tag: String,
    pub(crate) arms: Vec<Arm>,
}

pub struct Arm {
    pub(crate) value: String,
    pub(crate) schema: String,
}

pub struct Operation {
    pub(crate) message: String,
    pub(crate) request: String,
    pub(crate) reply_message: String,
}

/// What the two extensions say, as the emitter needs it.
pub struct View {
    pub(crate) unions: Vec<Union>,
    pub(crate) operations: Vec<Operation>,
}

impl View {
    pub fn union_names(&self) -> impl Iterator<Item = &str> {
        self.unions.iter().map(|u| u.name.as_str())
    }

    /// The tag an arm component carries, when it is an arm.
    pub fn tag_of_arm(&self, component: &str) -> Option<&str> {
        let union = self.unions.iter().find(|u| u.arms.iter().any(|a| a.schema == component))?;
        Some(union.tag.as_str())
    }

    /// Every type name this file defines: typify must define none of them.
    pub fn written_names(&self) -> BTreeSet<String> {
        let unions = self.unions.iter().flat_map(|u| [u.name.clone(), tag_enum_name(u)]);
        unions.chain(["Operation".to_owned()]).collect()
    }

    /// Every component a request reaches, through `$ref`s and through a
    /// union to each of its arms (D6's lenient decoding).
    pub fn request_side(&self, schemas: &Map<String, Value>) -> BTreeSet<String> {
        let mut seen = BTreeSet::new();
        let mut queue: VecDeque<String> = self.operations.iter().map(|o| o.request.clone()).collect();
        while let Some(name) = queue.pop_front() {
            if !seen.insert(name.clone()) {
                continue;
            }
            if let Some(union) = self.unions.iter().find(|u| u.name == name) {
                queue.extend(union.arms.iter().map(|a| a.schema.clone()));
            }
            let mut refs = Vec::new();
            collect_refs(schemas.get(&name).unwrap_or(&Value::Null), &mut refs);
            queue.extend(refs);
        }
        seen
    }
}

fn collect_refs(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            if let Some(name) = map.get("$ref").and_then(Value::as_str).and_then(|r| r.strip_prefix(SCHEMA_REF)) {
                out.push(name.to_owned());
            }
            map.values().for_each(|v| collect_refs(v, out));
        }
        Value::Array(items) => items.iter().for_each(|v| collect_refs(v, out)),
        _ => {}
    }
}

pub fn read(view: &Value) -> Result<View, String> {
    let schemas = view.pointer("/components/schemas").and_then(Value::as_object).ok_or("the view has no components.schemas")?;
    let list = |key: &str| view.get(key).and_then(Value::as_array).ok_or(format!("the view has no {key}"));
    let unions: Vec<Union> = list("x-lingara-unions")?.iter().map(read_union).collect::<Result<_, _>>()?;
    let operations: Vec<Operation> = list("x-lingara-app-operations")?.iter().map(read_operation).collect::<Result<_, _>>()?;
    let named = unions.iter().flat_map(|u| u.arms.iter().map(|a| &a.schema)).chain(operations.iter().map(|o| &o.request));
    for name in named {
        if !schemas.contains_key(name) {
            return Err(format!("no component {name}"));
        }
    }
    Ok(View { unions, operations })
}

fn text(entry: &Value, key: &str) -> Result<String, String> {
    entry.get(key).and_then(Value::as_str).map(str::to_owned).ok_or(format!("an entry without {key}: {entry}"))
}

fn component(entry: &Value, key: &str) -> Result<String, String> {
    let reference = text(entry, key)?;
    reference.strip_prefix(SCHEMA_REF).map(str::to_owned).ok_or(format!("{reference} is not under {SCHEMA_REF}"))
}

fn read_union(entry: &Value) -> Result<Union, String> {
    let name = text(entry, "name")?;
    let arms = entry.get("arms").and_then(Value::as_array).ok_or(format!("{name}: no arms"))?;
    let arms = arms.iter().map(|a| Ok(Arm { value: text(a, "value")?, schema: component(a, "schema")? }));
    Ok(Union { tag: text(entry, "tag")?, arms: arms.collect::<Result<_, String>>()?, name })
}

fn read_operation(entry: &Value) -> Result<Operation, String> {
    Ok(Operation { message: text(entry, "message")?, request: component(entry, "request")?, reply_message: text(entry, "reply_message")? })
}

