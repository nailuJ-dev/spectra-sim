use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{Result, Scenario};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplayManifest {
    pub schema_version: String,
    pub seed: u64,
    pub scenario_sha256: String,
    pub output_sha256: String,
    pub simulator_version: String,
}

pub fn canonical_sha256<T: Serialize>(value: &T) -> Result<String> {
    let bytes = serde_json::to_vec(value)?;
    let digest = Sha256::digest(bytes);
    Ok(hex_lower(&digest))
}

pub fn replay_manifest<T: Serialize>(scenario: &Scenario, output: &T) -> Result<ReplayManifest> {
    Ok(ReplayManifest { schema_version: "spectra-sim-replay-v1".into(), seed: scenario.seed, scenario_sha256: canonical_sha256(scenario)?, output_sha256: canonical_sha256(output)?, simulator_version: env!("CARGO_PKG_VERSION").into() })
}

fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes { out.push(HEX[(byte >> 4) as usize] as char); out.push(HEX[(byte & 0x0f) as usize] as char); }
    out
}
