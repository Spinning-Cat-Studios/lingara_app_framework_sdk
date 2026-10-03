//! `app-codegen --registry <versions.toml> --source <SOURCE> --out-dir <dir> [--check]`
//! (ADR 30.9.26al D4).
//!
//! The view is built from the frozen snapshot of the registry's `current`
//! version, `<registry dir>/versions/<id>.asyncapi.json`. The live
//! `spec/asyncapi.json` describes the unpinnable development version and is
//! never read. A registry with no `current`, or a `current` with no AsyncAPI
//! snapshot, is a refusal, never a fallback.
//!
//! Exit 0: written, or `--check` found both dialects current. Exit 1:
//! `--check` found a dialect that differs. Exit 2: a refusal, or input that
//! could not be read.

use std::fs;
use std::path::{Path, PathBuf};

use crate::build_view;

const USAGE: &str = "usage: app-codegen --registry <versions.toml> --source <SOURCE file> --out-dir <dir> [--check]";

/// The two files a run writes into `--out-dir`.
pub const OUTPUTS: [&str; 2] = ["apps.3.1.json", "apps.3.0.json"];

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Args {
    pub registry: PathBuf,
    pub source: PathBuf,
    pub out_dir: PathBuf,
    pub check: bool,
}

pub fn parse(args: &[String]) -> Result<Args, String> {
    let mut out = Args::default();
    let mut it = args.iter();
    while let Some(flag) = it.next() {
        if flag == "--check" {
            out.check = true;
            continue;
        }
        let value = it.next().ok_or_else(|| format!("{flag} needs a value"))?;
        match flag.as_str() {
            "--registry" => out.registry = value.into(),
            "--source" => out.source = value.into(),
            "--out-dir" => out.out_dir = value.into(),
            _ => return Err(format!("unknown argument {flag}")),
        }
    }
    if [&out.registry, &out.source, &out.out_dir].iter().any(|p| p.as_os_str().is_empty()) {
        return Err("--registry, --source and --out-dir are all required".into());
    }
    Ok(out)
}

pub fn run(args: &[String]) -> i32 {
    match parse(args).and_then(|parsed| generate(&parsed).map(|files| (parsed, files))) {
        Err(e) => {
            eprintln!("app-codegen: {e}\n{USAGE}");
            2
        }
        Ok((parsed, files)) if parsed.check => check(&parsed.out_dir, &files),
        Ok((parsed, files)) => write(&parsed.out_dir, &files),
    }
}

/// The rendered dialects, in `OUTPUTS` order.
fn generate(parsed: &Args) -> Result<[String; 2], String> {
    let (id, snapshot) = current_snapshot(&parsed.registry)?;
    let text = fs::read_to_string(&snapshot).map_err(|e| format!("{}: {e}", snapshot.display()))?;
    let catalogue = serde_json::from_str(&text).map_err(|e| format!("{}: {e}", snapshot.display()))?;
    let source = fs::read_to_string(&parsed.source).map_err(|e| format!("{}: {e}", parsed.source.display()))?;
    let view = build_view(&catalogue, &id, source.trim()).map_err(|r| format!("refused: {r}"))?;
    Ok([spec_codegen::render(&view.v31), spec_codegen::render(&view.v30)])
}

/// The current version's id and its `<id>.asyncapi.json`.
pub fn current_snapshot(registry: &Path) -> Result<(String, PathBuf), String> {
    let text = fs::read_to_string(registry).map_err(|e| format!("{}: {e}", registry.display()))?;
    let table: toml::Table = toml::from_str(&text).map_err(|e| format!("{}: {e}", registry.display()))?;
    let id = current_id(&table).ok_or_else(|| format!("refused: {} has no supported or lts version", registry.display()))?;
    let snapshot = registry.parent().unwrap_or(Path::new(".")).join("versions").join(format!("{id}.asyncapi.json"));
    if !snapshot.is_file() {
        return Err(format!("refused: the current version {id} has no {}", snapshot.display()));
    }
    Ok((id, snapshot))
}

/// The newest `supported`/`lts` entry by `minted_at`: the rule Backend's
/// `api_versions::lifecycle::current` holds, restated as the client
/// libraries' `spec-codegen` restates it, because neither repo links
/// Backend's crates. The registry's timestamps are canonical
/// `YYYY-MM-DDTHH:MM:SSZ`, so they order as text.
fn current_id(table: &toml::Table) -> Option<String> {
    let field = |v: &toml::Value, key: &str| v.get(key).and_then(toml::Value::as_str).map(str::to_owned);
    table
        .get("version")?
        .as_array()?
        .iter()
        .filter(|v| matches!(field(v, "state").as_deref(), Some("supported" | "lts")))
        .max_by_key(|v| field(v, "minted_at"))
        .and_then(|v| field(v, "id"))
}

fn check(dir: &Path, files: &[String; 2]) -> i32 {
    let mut stale = 0;
    for (name, want) in OUTPUTS.iter().zip(files) {
        let path = dir.join(name);
        if fs::read_to_string(&path).ok().as_ref() != Some(want) {
            eprintln!("app-codegen: {} differs from the view; run make spec-view", path.display());
            stale += 1;
        }
    }
    i32::from(stale > 0)
}

fn write(dir: &Path, files: &[String; 2]) -> i32 {
    let result = fs::create_dir_all(dir)
        .and_then(|()| OUTPUTS.iter().zip(files).try_for_each(|(name, text)| fs::write(dir.join(name), text)));
    match result {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("app-codegen: {}: {e}", dir.display());
            2
        }
    }
}
