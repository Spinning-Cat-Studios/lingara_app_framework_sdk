use std::fs;
use std::path::Path;

use serde_json::{Value, json};

use crate::cli::run;
use crate::test_support::{all_keys, catalogue};

/// A registry whose current is `2026-09-b` (newest supported by minted_at),
/// beside a development version and a live bundle that must never be read.
fn spec_dir(dir: &Path, current: &Value) {
    fs::create_dir_all(dir.join("versions")).unwrap();
    let registry = r#"
[[version]]
id = "2026-09-a"
minted_at = "2026-09-01T00:00:00Z"
state = "supported"

[[version]]
id = "2026-09-b"
minted_at = "2026-09-02T00:00:00Z"
state = "supported"

[[version]]
id = "2026-10-c"
minted_at = "2026-10-01T00:00:00Z"
state = "development"
"#;
    fs::write(dir.join("versions.toml"), registry).unwrap();
    fs::write(dir.join("versions/2026-09-b.asyncapi.json"), current.to_string()).unwrap();
    fs::write(dir.join("asyncapi.json"), "not json: the live bundle is never read").unwrap();
    fs::write(dir.join("SOURCE"), "backend@0123\n").unwrap();
}

fn args(dir: &Path, check: bool) -> Vec<String> {
    let path = |name: &str| dir.join(name).to_string_lossy().into_owned();
    let mut a = vec![
        "--registry".to_string(),
        path("versions.toml"),
        "--source".to_string(),
        path("SOURCE"),
        "--out-dir".to_string(),
        path("generator"),
    ];
    if check {
        a.push("--check".into());
    }
    a
}

fn read(dir: &Path, name: &str) -> Value {
    serde_json::from_str(&fs::read_to_string(dir.join("generator").join(name)).unwrap()).unwrap()
}

/// ADR 30.9.26al AC6: the CLI reads `current` from the registry and never
/// the live bundle; it exits 2 on a refusal and on a current without an app
/// channel; `--check` exits 1 on a hand-edited dialect and 0 on a match.
#[test]
fn refusal_exits_2_and_check_catches_a_hand_edit() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    spec_dir(dir, &catalogue());
    assert_eq!(run(&args(dir, false)), 0);
    assert_eq!(read(dir, "apps.3.1.json")["info"]["version"], "2026-09-b");
    assert_eq!(read(dir, "apps.3.1.json")["info"]["x-lingara-view"]["source"], "backend@0123");
    assert_eq!(run(&args(dir, true)), 0, "a fresh view is current");

    let path = dir.join("generator/apps.3.0.json");
    fs::write(&path, fs::read_to_string(&path).unwrap().replace("3.0.3", "3.0.2")).unwrap();
    assert_eq!(run(&args(dir, true)), 1, "a hand edit is drift");

    let mut no_app = catalogue();
    no_app["channels"].as_object_mut().unwrap().remove("app");
    spec_dir(dir, &no_app);
    assert_eq!(run(&args(dir, false)), 2, "a current without an app channel");

    let mut refused = catalogue();
    refused["channels"]["app"]["bindings"] = json!({});
    spec_dir(dir, &refused);
    assert_eq!(run(&args(dir, false)), 2, "a refusal");

    fs::remove_file(dir.join("versions/2026-09-b.asyncapi.json")).unwrap();
    assert_eq!(run(&args(dir, false)), 2, "a current with no snapshot is never a fallback to the live bundle");
}

/// ADR 30.9.26al AC7: the 3.0 dialect is the shared crate's pass over the
/// 3.1 view: a type-array nullable and a nullable `$ref` are rewritten, and
/// no `const` survives into it.
#[test]
fn the_30_dialect_comes_from_the_shared_crate() {
    let tmp = tempfile::tempdir().unwrap();
    spec_dir(tmp.path(), &catalogue());
    assert_eq!(run(&args(tmp.path(), false)), 0);
    let v30 = read(tmp.path(), "apps.3.0.json");
    assert_eq!(v30["openapi"], "3.0.3");
    let schemas = &v30["components"]["schemas"];
    assert_eq!(schemas["ListItemText"]["properties"]["lang"], json!({ "type": "string", "nullable": true }));
    assert_eq!(
        schemas["CardElementButton"]["properties"]["style"],
        json!({ "type": "string", "nullable": true, "allOf": [{ "$ref": "#/components/schemas/ButtonStyle" }] })
    );
    let mut keys = Vec::new();
    all_keys(&v30, &mut keys);
    assert!(!keys.iter().any(|k| k == "const"), "a const reached the 3.0 dialect");
}
