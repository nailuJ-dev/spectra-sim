use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{run_cuas, run_sigint, Result, Scenario};

/// Version of the serialized output contract that `output_sha256` covers.
pub const OUTPUT_SCHEMA_VERSION: &str = "spectra-sim-output-v1";

fn legacy_output_schema() -> String {
    "unversioned".into()
}

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplayManifest {
    pub schema_version: String,
    pub seed: u64,
    pub scenario_sha256: String,
    pub output_sha256: String,
    pub simulator_version: String,
    #[serde(default = "legacy_output_schema")]
    pub output_schema_version: String,
}

pub fn canonical_sha256<T: Serialize>(value: &T) -> Result<String> {
    let bytes = serde_json::to_vec(value)?;
    let digest = Sha256::digest(bytes);
    Ok(hex_lower(&digest))
}

pub fn replay_manifest<T: Serialize>(scenario: &Scenario, output: &T) -> Result<ReplayManifest> {
    Ok(ReplayManifest {
        schema_version: "spectra-sim-replay-v1".into(),
        seed: scenario.seed,
        scenario_sha256: canonical_sha256(scenario)?,
        output_sha256: canonical_sha256(output)?,
        simulator_version: env!("CARGO_PKG_VERSION").into(),
        output_schema_version: OUTPUT_SCHEMA_VERSION.into(),
    })
}

/// Which simulation job a manifest was produced by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RunKind {
    Sigint,
    Cuas,
}

/// Outcome of re-running a scenario against a stored replay manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ReplayVerdict {
    Match,
    OutputMismatch {
        expected: String,
        actual: String,
    },
    ScenarioMismatch {
        expected: String,
        actual: String,
    },
    VersionMismatch {
        manifest: String,
        current: String,
    },
    /// The output hash differs and the manifest's output schema is not the current one.
    SchemaMismatch {
        manifest: String,
        current: String,
    },
}

/// Re-runs `scenario` and compares the result against `manifest`.
///
/// Checks the scenario hash first, then re-runs and compares the output hash.
/// A differing output is reported as `VersionMismatch` when the manifest was
/// produced by a different simulator version, as `SchemaMismatch` when it
/// was produced under a different output schema (including legacy manifests
/// without one), otherwise as `OutputMismatch`. A manifest whose output hash
/// agrees is a `Match` regardless of its schema tag.
pub fn verify_replay(
    scenario: &Scenario,
    manifest: &ReplayManifest,
    kind: RunKind,
) -> Result<ReplayVerdict> {
    let scenario_hash = canonical_sha256(scenario)?;
    if scenario_hash != manifest.scenario_sha256 {
        return Ok(ReplayVerdict::ScenarioMismatch {
            expected: manifest.scenario_sha256.clone(),
            actual: scenario_hash,
        });
    }
    let fresh = match kind {
        RunKind::Sigint => run_sigint(scenario)?.manifest,
        RunKind::Cuas => run_cuas(scenario)?.manifest,
    };
    if fresh.output_sha256 == manifest.output_sha256 {
        return Ok(ReplayVerdict::Match);
    }
    let current = env!("CARGO_PKG_VERSION");
    if manifest.simulator_version != current {
        return Ok(ReplayVerdict::VersionMismatch {
            manifest: manifest.simulator_version.clone(),
            current: current.into(),
        });
    }
    if manifest.output_schema_version != OUTPUT_SCHEMA_VERSION {
        return Ok(ReplayVerdict::SchemaMismatch {
            manifest: manifest.output_schema_version.clone(),
            current: OUTPUT_SCHEMA_VERSION.into(),
        });
    }
    Ok(ReplayVerdict::OutputMismatch {
        expected: manifest.output_sha256.clone(),
        actual: fresh.output_sha256,
    })
}

fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}
