//! File helpers shared by every CLI subcommand.

use std::fs;
use std::io::Read;
use std::path::Path;

use serde::de::DeserializeOwned;
use serde::Serialize;
use spectra_sim::formats::{iq_to_npy_bytes, sigmf_data_bytes, sigmf_meta_json};
use spectra_sim::{
    sigint_golden_scenario, CuasRun, IqCaptureOut, Result, Scenario, SigintRun, SimError,
};

use super::ExtraFormat;

/// Maximum accepted size of any input file (scenario, manifest, config).
const MAX_INPUT_BYTES: usize = 32 * 1024 * 1024;

/// Reads `path` fully, but never more than [`MAX_INPUT_BYTES`] plus one byte,
/// so special files (`/dev/zero`, FIFOs) cannot exhaust memory.
fn read_bounded(path: &Path) -> Result<Vec<u8>> {
    let file = fs::File::open(path).map_err(|error| io_with_path(error, path))?;
    let mut bytes = Vec::new();
    file.take(MAX_INPUT_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| io_with_path(error, path))?;
    if bytes.len() > MAX_INPUT_BYTES {
        return Err(SimError::DimensionLimit {
            actual: bytes.len(),
            max: MAX_INPUT_BYTES,
        });
    }
    Ok(bytes)
}

/// Parses JSON bytes read from `path`; errors name the file.
fn parse_json<T: DeserializeOwned>(bytes: &[u8], path: &Path) -> Result<T> {
    serde_json::from_slice(bytes).map_err(|error| {
        SimError::Json(<serde_json::Error as serde::de::Error>::custom(format!(
            "{}: {error}",
            path.display()
        )))
    })
}

/// Reads, parses and validates a scenario file.
pub fn load_scenario(path: &Path) -> Result<Scenario> {
    let scenario: Scenario = parse_json(&read_bounded(path)?, path)?;
    scenario.validate()?;
    Ok(scenario)
}

/// Reads and parses a JSON file (size-capped, no further validation).
pub fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T> {
    parse_json(&read_bounded(path)?, path)
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

/// Writes the optional I/Q interchange files next to `iq_capture.json`.
/// Duplicate formats are written once. `dir` must already exist.
pub fn write_iq_extras(dir: &Path, capture: &IqCaptureOut, formats: &[ExtraFormat]) -> Result<()> {
    if formats.contains(&ExtraFormat::Npy) {
        let path = dir.join("iq_capture.npy");
        fs::write(&path, iq_to_npy_bytes(capture)).map_err(|error| io_with_path(error, &path))?;
    }
    if formats.contains(&ExtraFormat::Sigmf) {
        let data_name = "iq_capture.sigmf-data";
        let data_path = dir.join(data_name);
        fs::write(&data_path, sigmf_data_bytes(capture))
            .map_err(|error| io_with_path(error, &data_path))?;
        write_json(
            &dir.join("iq_capture.sigmf-meta"),
            &sigmf_meta_json(capture, data_name),
        )?;
    }
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
pub(super) fn io_with_path(error: std::io::Error, path: &Path) -> SimError {
    SimError::Io(std::io::Error::new(
        error.kind(),
        format!("{}: {error}", path.display()),
    ))
}
