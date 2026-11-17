//! `replay` subcommand: verify a scenario against a stored replay manifest.

use std::path::Path;

use spectra_sim::{verify_replay, ReplayManifest, ReplayVerdict, Result, SimError};

use super::io::{load_scenario, read_json};
use super::KindArg;

/// One-line description of a non-matching verdict.
fn describe(verdict: &ReplayVerdict, manifest: &ReplayManifest) -> Option<String> {
    match verdict {
        ReplayVerdict::Match => None,
        ReplayVerdict::OutputMismatch { expected, actual } => Some(format!(
            "output mismatch: manifest output_sha256 {expected}, recomputed {actual}"
        )),
        ReplayVerdict::ScenarioMismatch { expected, actual } => {
            let mut message = format!(
                "scenario mismatch: manifest scenario_sha256 {expected}, scenario file {actual}"
            );
            let current = env!("CARGO_PKG_VERSION");
            if manifest.simulator_version != current {
                message.push_str(&format!(
                    " (manifest from simulator {}, running {current})",
                    manifest.simulator_version
                ));
            }
            Some(message)
        }
        ReplayVerdict::VersionMismatch { manifest, current } => Some(format!(
            "version mismatch: manifest from simulator {manifest}, running {current}"
        )),
        ReplayVerdict::SchemaMismatch { manifest, current } => Some(format!(
            "schema mismatch: manifest output schema \"{manifest}\", current \"{current}\""
        )),
        // `ReplayVerdict` is non-exhaustive: treat unknown verdicts as failures.
        _ => Some("replay verification failed with an unrecognized verdict".into()),
    }
}

/// Verifies `scenario_path` against `manifest_path`; any mismatch is an error (exit 2).
pub fn run(scenario_path: &Path, manifest_path: &Path, kind: KindArg) -> Result<()> {
    let scenario = load_scenario(scenario_path)?;
    let manifest: ReplayManifest = read_json(manifest_path)?;
    let verdict = verify_replay(&scenario, &manifest, kind.into())?;
    match describe(&verdict, &manifest) {
        None => {
            println!("MATCH");
            Ok(())
        }
        Some(reason) => Err(SimError::InvalidArgument(reason)),
    }
}
