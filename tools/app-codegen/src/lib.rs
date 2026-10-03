//! The app view of the Lingara AsyncAPI catalogue (ADR 30.9.26al D4).
//!
//! Each kit generates its models with the OpenAPI generator its language's
//! client library already uses, and those read OpenAPI, not AsyncAPI. So this
//! crate writes the app half of the catalogue as an OpenAPI document with
//! empty `paths`: the schemas the app operations reach (`reach`), the tagged
//! unions among them lifted into named arms (`unions`), and a record of the
//! operations (`select`). The 3.0 dialect is the client libraries' own pass.
//! Every pass refuses what it cannot say, naming the JSON pointer, rather
//! than approximating it.

pub mod cli;
pub mod reach;
pub mod select;
pub mod unions;

#[cfg(test)]
mod cli_tests;
#[cfg(test)]
mod reach_tests;
#[cfg(test)]
mod select_tests;
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod unions_tests;
#[cfg(test)]
mod view_tests;

use serde_json::{Value, json};
pub use spec_codegen::Refusal;

/// Both dialects of one view.
pub struct View {
    pub v31: Value,
    pub v30: Value,
}

/// The view of `catalogue`, the frozen AsyncAPI snapshot of `version`, naming
/// `source` (the `spec/SOURCE` line) as its origin.
pub fn build_view(catalogue: &Value, version: &str, source: &str) -> Result<View, Refusal> {
    let selection = select::select(catalogue)?;
    let mut schemas = reach::copy_reachable(catalogue, &selection.payload_roots())?;
    let unions = unions::lift(&mut schemas)?;
    let v31 = json!({
        "openapi": "3.1.0",
        "info": {
            "title": "Lingara apps",
            "version": version,
            "x-lingara-view": { "source": source, "from": "asyncapi 3.0" },
        },
        "paths": {},
        "components": { "schemas": schemas },
        "x-lingara-app-operations": selection.records(),
        "x-lingara-unions": unions,
    });
    let v30 = spec_codegen::dialect30::dialect30(&v31)?;
    Ok(View { v31, v30 })
}

/// A `Refusal` naming `pointer`.
pub(crate) fn refuse<T>(pointer: &str, why: &str) -> Result<T, Refusal> {
    Err(Refusal(format!("{pointer}: {why}")))
}

/// `#/components/schemas/<name>` for a schema name.
pub(crate) fn schema_ref(name: &str) -> String {
    format!("#/components/schemas/{}", spec_codegen::walk::escape(name))
}
