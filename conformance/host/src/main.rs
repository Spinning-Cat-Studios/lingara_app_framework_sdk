//! The Lingara app kits' conformance host (ADR 30.9.26al D6): the relay's
//! stand-in. `run` drives a kit's fixture app through every case;
//! `check-coverage` proves the cases cover the view and the contract.

mod case;
mod coverage;
mod exchange;
mod limits;
mod run;
mod sign;
mod validate;

#[cfg(test)]
mod coverage_tests;
#[cfg(test)]
mod limits_tests;
#[cfg(test)]
mod run_tests;
#[cfg(test)]
mod sign_tests;
#[cfg(test)]
mod validate_tests;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "lingara-apps-conformance-host", about = "The Lingara relay's stand-in for app kit conformance")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start a fixture app and drive every case against it.
    Run {
        #[arg(long)]
        lang: String,
        #[arg(long, default_value = "conformance/cases")]
        cases: PathBuf,
        #[arg(long, default_value = "spec/generator/apps.3.1.json")]
        view: PathBuf,
        /// Comma-separated case ids, for local debugging; refused under CI.
        #[arg(long, value_delimiter = ',')]
        only: Vec<String>,
        /// The fixture app's command line.
        #[arg(last = true, required = true)]
        command: Vec<String>,
    },
    /// Fail when an app operation, a behaviour or a case is uncovered.
    CheckCoverage {
        #[arg(long)]
        view: PathBuf,
        #[arg(long, default_value = "conformance/cases")]
        cases: PathBuf,
    },
}

#[tokio::main]
async fn main() -> ExitCode {
    let result = match Cli::parse().command {
        Commands::Run { lang, cases, view, only, command } => {
            run::run(&run::Options { lang, cases, view, only, command }).await.map(|report| {
                report.lines.iter().for_each(|l| println!("{l}"));
                report.failed == 0
            })
        }
        Commands::CheckCoverage { view, cases } => check_coverage(&view, &cases),
    };
    match result {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(e) => {
            eprintln!("conformance host: {e}");
            ExitCode::from(2)
        }
    }
}

fn check_coverage(view: &std::path::Path, cases: &std::path::Path) -> Result<bool, String> {
    let text = std::fs::read_to_string(view).map_err(|e| format!("{}: {e}", view.display()))?;
    let view: serde_json::Value = serde_json::from_str(&text).map_err(|e| format!("{}: {e}", view.display()))?;
    let (loaded, errors) = case::load_all(cases);
    let gaps = coverage::gaps(&view, &loaded, &errors)?;
    gaps.iter().for_each(|g| println!("✗ {g}"));
    if gaps.is_empty() {
        println!("✓ {} cases cover every app operation and AK1–AK5", loaded.len());
    }
    Ok(gaps.is_empty())
}
