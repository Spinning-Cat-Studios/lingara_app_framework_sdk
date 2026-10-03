use std::fs;
use std::path::Path;

use crate::case::load_all;
use crate::coverage::gaps;
use crate::validate_tests::view;

const RENDER: &str = "  operation: app.render\n  body: { json: { install_id: 7c1d2e3f-4a5b-4c6d-8e7f-9a0b1c2d3e4f, subject: s, slot: home.side, locale: en, context: [] } }\n";
const ACTION: &str = "  operation: app.action\n  body: { json: { install_id: 7c1d2e3f-4a5b-4c6d-8e7f-9a0b1c2d3e4f, subject: s, slot: home.side, locale: en, context: [], action_id: inc, card_etag: c1_0 } }\n";

/// `at` is `<group>/<name>`, the case's path under `dir` and so its id.
fn write(dir: &Path, at: &str, behaviours: &str, request: &str, status: u16) {
    let (group, name) = at.split_once('/').expect("group/name");
    fs::create_dir_all(dir.join(group)).unwrap();
    let yaml = format!("id: {group}.{name}\nbehaviours: [{behaviours}]\nrequest:\n{request}expect: {{ status: {status} }}\n");
    fs::write(dir.join(group).join(format!("{name}.yaml")), yaml).unwrap();
}

/// A complete set: both operations and all five behaviours.
fn complete(dir: &Path) {
    write(dir, "op/render", "AK1, AK2, AK3", RENDER, 200);
    write(dir, "op/action", "AK4, AK5", ACTION, 200);
}

fn gaps_in(dir: &Path) -> Vec<String> {
    let (loaded, errors) = load_all(dir);
    gaps(&view(), &loaded, &errors).expect("the view compiles")
}

fn fails_with(dir: &Path, needle: &str) {
    let found = gaps_in(dir);
    assert!(found.iter().any(|g| g.contains(needle)), "no gap mentions {needle:?}: {found:?}");
}

/// ADR 30.9.26al AC13: `check-coverage` fails an app operation with no
/// case, a case naming an unknown message, a behaviour with no case, a case
/// whose id does not match its path, a `200`-expecting `json` body that is
/// not a valid request, and an unknown case key.
#[test]
fn coverage_fails_every_gap() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    complete(dir);
    assert_eq!(gaps_in(dir), Vec::<String>::new(), "a complete set has no gap");

    fs::remove_file(dir.join("op/action.yaml")).unwrap();
    fails_with(dir, "no case exercises the app operation app.action");
    fails_with(dir, "no case covers AK4");
    complete(dir);

    write(dir, "op/unknown", "AK1", &RENDER.replace("app.render", "app.unknown"), 400);
    fails_with(dir, "names the message app.unknown, which the view lacks");
    fs::remove_file(dir.join("op/unknown.yaml")).unwrap();

    fs::write(dir.join("op/moved.yaml"), fs::read_to_string(dir.join("op/render.yaml")).unwrap()).unwrap();
    fails_with(dir, "op.render: the id does not match its path (op.moved)");
    fs::remove_file(dir.join("op/moved.yaml")).unwrap();

    write(dir, "op/bad-body", "AK3", &RENDER.replace("home.side", "nowhere"), 200);
    fails_with(dir, "op.bad-body: the json body is not a valid app.render request");
    write(dir, "op/bad-body", "AK3", &RENDER.replace("home.side", "nowhere"), 400);
    assert_eq!(gaps_in(dir), Vec::<String>::new(), "only a 200-expecting body is held to the schema");
    fs::remove_file(dir.join("op/bad-body.yaml")).unwrap();

    write(dir, "op/extra-key", "AK3", &format!("{RENDER}  retries: 1\n"), 200);
    fails_with(dir, "does not load");
}
