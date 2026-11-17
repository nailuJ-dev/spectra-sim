//! `replay` subcommand: verify a scenario against a stored replay manifest.

use std::path::Path;

use clap::ValueEnum;
use spectra_sim::{verify_replay, ReplayManifest, ReplayVerdict, Result, RunKind, SimError};

use super::io::{load_scenario, read_json};

/// Command-line spelling of [`RunKind`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum KindArg {
    Sigint,
    Cuas,
}

impl From<KindArg> for RunKind {
    fn from(kind: KindArg) -> Self {
        match kind {
            KindArg::Sigint => RunKind::Sigint,
            KindArg::Cuas => RunKind::Cuas,
        }
    }
}

/// One-line description of a non-matching verdict.
fn describe(verdict: &ReplayVerdict) -> Option<String> {
    match verdict {
        ReplayVerdict::Match => None,
        ReplayVerdict::OutputMismatch { expected, actual } => Some(format!(
            "output mismatch: manifest output_sha256 {expected}, recomputed {actual}"
        )),
        ReplayVerdict::ScenarioMismatch { expected, actual } => Some(format!(
            "scenario mismatch: manifest scenario_sha256 {expected}, scenario file {actual}"
        )),
        ReplayVerdict::VersionMismatch { manifest, current } => Some(format!(
            "version mismatch: manifest from simulator {manifest}, running {current}"
        )),
    }
}

/// Verifies `scenario_path` against `manifest_path`; any mismatch is an error (exit 2).
pub fn run(scenario_path: &Path, manifest_path: &Path, kind: KindArg) -> Result<()> {
    let scenario = load_scenario(scenario_path)?;
    let manifest: ReplayManifest = read_json(manifest_path)?;
    let verdict = verify_replay(&scenario, &manifest, kind.into())?;
    match describe(&verdict) {
        None => {
            println!("MATCH");
            Ok(())
        }
        Some(reason) => Err(SimError::InvalidArgument(reason)),
    }
}
