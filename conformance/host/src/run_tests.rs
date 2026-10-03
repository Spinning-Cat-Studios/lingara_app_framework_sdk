use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use crate::case::{Loaded, load_all};
use crate::run::{check_only, drive, spawn};
use crate::validate::{Budgets, Schemas};
use crate::validate_tests::view;

/// How the in-test fixture answers.
#[derive(Clone, Copy)]
enum Mode {
    /// Reads the whole request, then a valid card.
    Card,
    /// Answers 413 from the head alone and closes without reading the body.
    EarlyRefusal,
    /// Sends its headers at once and its body after `delay`.
    SlowBody(Duration),
}

const CARD: &[u8] = br#"{"card":{"elements":[{"type":"heading","text":"home.side","level":1}]}}"#;

/// A raw TCP fixture, so the tests see header names exactly as sent. It
/// records each request's head.
async fn fixture(mode: Mode) -> (u16, Arc<Mutex<Vec<String>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let heads = Arc::new(Mutex::new(Vec::new()));
    let seen = heads.clone();
    tokio::spawn(async move {
        while let Ok((mut stream, _)) = listener.accept().await {
            let head = read_head(&mut stream).await;
            seen.lock().unwrap().push(head.clone());
            answer(&mut stream, &head, mode).await;
        }
    });
    (port, heads)
}

async fn read_head(stream: &mut tokio::net::TcpStream) -> String {
    let mut buf = Vec::new();
    let mut byte = [0u8; 1];
    while !buf.ends_with(b"\r\n\r\n") && stream.read(&mut byte).await.unwrap_or(0) == 1 {
        buf.push(byte[0]);
    }
    String::from_utf8_lossy(&buf).into_owned()
}

async fn answer(stream: &mut tokio::net::TcpStream, head: &str, mode: Mode) {
    let length = head
        .lines()
        .find_map(|l| l.to_ascii_lowercase().strip_prefix("content-length:").map(|v| v.trim().parse::<usize>().unwrap()))
        .unwrap_or(0);
    let ok_head = format!("HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n", CARD.len());
    match mode {
        Mode::Card => {
            let mut body = vec![0; length];
            let _ = stream.read_exact(&mut body).await;
            let _ = stream.write_all(&[ok_head.as_bytes(), CARD].concat()).await;
        }
        Mode::EarlyRefusal => {
            let _ = stream.write_all(b"HTTP/1.1 413 Payload Too Large\r\ncontent-length: 0\r\n\r\n").await;
        }
        Mode::SlowBody(delay) => {
            let mut body = vec![0; length];
            let _ = stream.read_exact(&mut body).await;
            let _ = stream.write_all(ok_head.as_bytes()).await;
            tokio::time::sleep(delay).await;
            let _ = stream.write_all(CARD).await;
        }
    }
}

const RENDER: &str = "  operation: app.render\n  body: { json: { install_id: 7c1d2e3f-4a5b-4c6d-8e7f-9a0b1c2d3e4f, subject: s, slot: home.side, locale: en, context: [] } }\n";

fn write(dir: &Path, name: &str, request: &str, expect: &str) {
    std::fs::create_dir_all(dir.join("op")).unwrap();
    let yaml = format!("id: op.{name}\nbehaviours: [AK3]\nrequest:\n{request}expect:\n{expect}\n");
    std::fs::write(dir.join("op").join(format!("{name}.yaml")), yaml).unwrap();
}

async fn drive_dir(dir: &Path, port: u16, budgets: Budgets) -> crate::run::Report {
    let (loaded, errors) = load_all(dir);
    assert!(errors.is_empty(), "{errors:?}");
    let cases: Vec<&Loaded> = loaded.iter().collect();
    drive(port, &cases, &Schemas::from_view(&view()).unwrap(), budgets).await
}

/// ADR 30.9.26al AC12: against an in-test fixture, `run` sends one unjudged
/// warm-up and then drives every case; a `header_case: title` case arrives
/// with title-case names; a fixture that answers 413 and closes mid-body is
/// judged on its status; a fixture that sends its headers at once and its
/// body after the budget fails on time; `run` fails when the app never
/// prints `listening`, and refuses `--only` under CI.
#[tokio::test]
async fn every_case_runs_and_a_silent_app_fails() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let schema = "  status: 200\n  body: { matches_schema: reply }";
    write(dir, "lower", RENDER, schema);
    write(dir, "title", &format!("{RENDER}  header_case: title\n"), schema);
    let (port, heads) = fixture(Mode::Card).await;
    let report = drive_dir(dir, port, Budgets::default()).await;
    assert_eq!(report.failed, 0, "{:?}", report.lines);
    assert_eq!(report.lines, ["✓ op.lower", "✓ op.title"]);
    let heads = heads.lock().unwrap().clone();
    assert_eq!(heads.len(), 3, "one warm-up, then one request per case");
    assert!(heads[1].contains("\r\nwebhook-signature: ") && heads[1].contains("\r\nuser-agent: Lingara-Apps/1"));
    assert!(heads[2].contains("\r\nWebhook-Signature: ") && heads[2].contains("\r\nWebhook-Id: lgr_msg_"), "{}", heads[2]);

    let refusal = tempfile::tempdir().unwrap();
    let pad = "  body:\n    raw_pad: { json: { type: app.render }, to_bytes: 2000000 }\n";
    write(refusal.path(), "oversized", pad, "  status: 413");
    let (port, _) = fixture(Mode::EarlyRefusal).await;
    let report = drive_dir(refusal.path(), port, Budgets::default()).await;
    assert_eq!(report.failed, 0, "a 413 before the body is judged on its status: {:?}", report.lines);

    let fast = Budgets { render: Duration::from_millis(200), action: Duration::from_millis(200) };
    let (port, _) = fixture(Mode::SlowBody(Duration::from_millis(500))).await;
    let report = drive_dir(dir, port, fast).await;
    assert_eq!(report.failed, 2);
    assert!(report.lines.iter().any(|l| l.contains("over the relay's 200 ms budget")), "{:?}", report.lines);

    let silent = ["sh", "-c", "sleep 5"].map(String::from);
    let err = spawn(&silent, "silent", Duration::from_millis(300)).await.map(|_| ()).unwrap_err();
    assert!(err.contains("silent: the fixture app did not print `listening <port>`"), "{err}");
    let chatty = ["sh", "-c", "echo listening 4321; sleep 1"].map(String::from);
    let (_child, port) = spawn(&chatty, "chatty", Duration::from_secs(5)).await.expect("a handshake");
    assert_eq!(port, 4321);

    assert!(check_only(&["op.lower".into()], true).is_err(), "--only under CI");
    assert!(check_only(&["op.lower".into()], false).is_ok());
    assert!(check_only(&[], true).is_ok());
}
