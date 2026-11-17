//! File helpers shared by every CLI subcommand.

use std::fs;
use std::path::Path;

use serde::de::DeserializeOwned;
use serde::Serialize;
use spectra_sim::{sigint_golden_scenario, CuasRun, Result, Scenario, SigintRun, SimError};

/// Maximum accepted scenario file size.
const MAX_SCENARIO_BYTES: usize = 32 * 1024 * 1024;

/// Reads, parses and validates a scenario file.
pub fn load_scenario(path: &Path) -> Result<Scenario> {
    let bytes = fs::read(path).map_err(|error| io_with_path(error, path))?;
    if bytes.len() > MAX_SCENARIO_BYTES {
        return Err(SimError::DimensionLimit {
            actual: bytes.len(),
            max: MAX_SCENARIO_BYTES,
        });
    }
    let scenario: Scenario = serde_json::from_slice(&bytes)?;
    scenario.validate()?;
    Ok(scenario)
}

/// Reads and parses a JSON file (no size cap or validation).
pub fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let bytes = fs::read(path).map_err(|error| io_with_path(error, path))?;
    Ok(serde_json::from_slice(&bytes)?)
}

/// Writes pretty-printed JSON to `path`. Does not create parent directories.
pub fn write_json(path: &Path, value: &impl Serialize) -> Result<()> {
    fs::write(path, serde_json::to_vec_pretty(value)?)
        .map_err(|error| io_with_path(error, path))?;
    Ok(())
}

/// Creates `path` and any missing parents.
pub fn ensure_dir(path: &Path) -> Result<()> {
    fs::create_dir_all(path).map_err(|error| io_with_path(error, path))?;
    Ok(())
}

/// Writes the artifacts of a SIGINT run into `dir` (created if missing).
pub fn write_sigint_outputs(dir: &Path, scenario: &Scenario, run: &SigintRun) -> Result<()> {
    ensure_dir(dir)?;
    write_json(&dir.join("iq_capture.json"), &run.simulation.capture)?;
    let sdk_scenario = sigint_golden_scenario(scenario, &run.simulation.capture)?;
    write_json(&dir.join("sigint_scenario.json"), &sdk_scenario)?;
    write_json(&dir.join("truth.json"), &run.simulation.truth)?;
    write_json(&dir.join("replay_manifest.json"), &run.manifest)?;
    Ok(())
}

/// Writes the artifacts of a C-UAS run into `dir` (created if missing).
pub fn write_cuas_outputs(dir: &Path, run: &CuasRun) -> Result<()> {
    ensure_dir(dir)?;
    write_json(&dir.join("radar_truth.json"), &run.radar_measurement)?;
    write_json(&dir.join("cuas_scenario.json"), &run.recorded_scenario)?;
    if let Some(value) = &run.isac_measurement {
        write_json(&dir.join("isac_truth.json"), value)?;
    }
    if let Some(value) = &run.isac_scenario {
        write_json(&dir.join("cuas_isac_scenario.json"), value)?;
    }
    write_json(&dir.join("replay_manifest.json"), &run.manifest)?;
    Ok(())
}

/// Re-wraps an I/O error so the message names the offending path.
fn io_with_path(error: std::io::Error, path: &Path) -> SimError {
    SimError::Io(std::io::Error::new(
        error.kind(),
        format!("{}: {error}", path.display()),
    ))
}
