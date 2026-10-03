//! `run --lang <lang> -- <fixture app command…>` (ADR 30.9.26al D6).
//!
//! Spawns the fixture app with the two conformance secrets and port 0,
//! reads `listening <port>` as its first stdout line, sends one unjudged
//! warm-up render (a cold JVM's class loading is not the first case's
//! budget), then drives **every** case itself. There is no results file and
//! no `skip`, so a kit cannot omit a case.

use std::path::PathBuf;
use std::process::Stdio;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::Value;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};

use crate::case::{Case, Loaded, load_all};
use crate::exchange::exchange;
use crate::sign::{build, fixture_secrets};
use crate::validate::{Budgets, Schemas, judge};

/// How long a fixture app has to print `listening <port>`.
pub const HANDSHAKE: Duration = Duration::from_secs(30);
/// Grace beyond the budget before the host stops waiting; a reply inside
/// the grace still fails on time, with its real duration.
const GRACE: Duration = Duration::from_secs(2);

const WARM_UP: &str = r#"
id: warm-up
behaviours: [AK3]
request:
  operation: app.render
  body:
    json:
      install_id: 00000000-0000-4000-8000-000000000000
      subject: lgr_sub_conformance
      slot: plans.empty_detail
      locale: en
      context: []
expect:
  status: 200
"#;

pub struct Options {
    pub lang: String,
    pub cases: PathBuf,
    pub view: PathBuf,
    pub only: Vec<String>,
    pub command: Vec<String>,
}

/// One line per case, and how many failed.
#[derive(Debug, Default)]
pub struct Report {
    pub lines: Vec<String>,
    pub failed: usize,
}

/// `--only` is for local debugging: under CI every case runs.
pub fn check_only(only: &[String], ci: bool) -> Result<(), String> {
    if !only.is_empty() && ci {
        return Err("--only is refused when CI is set: every case runs".into());
    }
    Ok(())
}

pub async fn run(opts: &Options) -> Result<Report, String> {
    check_only(&opts.only, std::env::var_os("CI").is_some())?;
    let view: Value = serde_json::from_str(&std::fs::read_to_string(&opts.view).map_err(|e| format!("{}: {e}", opts.view.display()))?)
        .map_err(|e| format!("{}: {e}", opts.view.display()))?;
    let schemas = Schemas::from_view(&view)?;
    let (loaded, errors) = load_all(&opts.cases);
    if !errors.is_empty() {
        return Err(format!("cases that do not load:\n{}", errors.join("\n")));
    }
    let cases: Vec<&Loaded> =
        loaded.iter().filter(|l| opts.only.is_empty() || opts.only.contains(&l.case.id)).collect();
    let (mut child, port) = spawn(&opts.command, &opts.lang, HANDSHAKE).await?;
    let report = drive(port, &cases, &schemas, Budgets::default()).await;
    let _ = child.kill().await;
    Ok(report)
}

/// The fixture app, started, and the port it printed.
pub async fn spawn(command: &[String], lang: &str, wait: Duration) -> Result<(Child, u16), String> {
    let (program, args) = command.split_first().ok_or_else(|| format!("{lang}: no fixture app command"))?;
    let mut child = Command::new(program)
        .args(args)
        .env("LINGARA_APPS_CONFORMANCE_SECRETS", fixture_secrets())
        .env("LINGARA_APPS_CONFORMANCE_PORT", "0")
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| format!("{lang}: could not start {program}: {e}"))?;
    let mut lines = BufReader::new(child.stdout.take().expect("stdout is piped")).lines();
    let first = tokio::time::timeout(wait, lines.next_line()).await;
    let silent = || format!("{lang}: the fixture app did not print `listening <port>` within {} s", wait.as_secs());
    let line = first.map_err(|_| silent())?.map_err(|e| format!("{lang}: {e}"))?.ok_or_else(silent)?;
    let port = line.trim().strip_prefix("listening ").and_then(|p| p.parse().ok());
    let port = port.ok_or_else(|| format!("{lang}: the first stdout line was {line:?}, not `listening <port>`"))?;
    // The rest of stdout is the app's own logging: passed through, so a full
    // pipe never blocks it.
    tokio::spawn(async move {
        while let Ok(Some(l)) = lines.next_line().await {
            eprintln!("{l}");
        }
    });
    Ok((child, port))
}

/// The warm-up, then every case in order.
pub async fn drive(port: u16, cases: &[&Loaded], schemas: &Schemas, budgets: Budgets) -> Report {
    let warm_up: Case = serde_yaml::from_str(WARM_UP).expect("the warm-up case parses");
    if let Ok(built) = build(&warm_up, now()) {
        let _ = exchange(&built, port, warm_up.request.header_case, HANDSHAKE).await;
    }
    let mut report = Report::default();
    for loaded in cases {
        let mismatches = one_case(port, &loaded.case, schemas, budgets).await;
        if mismatches.is_empty() {
            report.lines.push(format!("✓ {}", loaded.case.id));
        } else {
            report.failed += 1;
            report.lines.extend(mismatches.into_iter().map(|m| format!("✗ {m}")));
        }
    }
    report
}

async fn one_case(port: u16, case: &Case, schemas: &Schemas, budgets: Budgets) -> Vec<String> {
    let budget = budgets.for_operation(case.request.operation_or_default());
    let built = match build(case, now()) {
        Ok(b) => b,
        Err(e) => return vec![format!("{}: {e}", case.id)],
    };
    match exchange(&built, port, case.request.header_case, budget + GRACE).await {
        Ok(reply) => judge(case, schemas, &reply, budget),
        Err(e) => vec![format!("{}: {e}", case.id)],
    }
}

fn now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or_default()
}
