//! Python bindings for `spectra-sim` (module `spectra_sim`).
//!
//! The scenario is passed as a JSON string, parsed and validated exactly as the
//! CLI does (`Scenario` deserialization followed by `validate()`), and the
//! results come back as numpy arrays or JSON strings.

#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

use numpy::{Complex32, PyArray1};
use pyo3::exceptions::{PyOSError, PyValueError};
use pyo3::prelude::*;
use spectra_sim::{ReplayManifest, ReplayVerdict, RunKind, Scenario, SimError};

/// Maximum accepted size of any JSON input string (same cap as the CLI).
const MAX_INPUT_BYTES: usize = 32 * 1024 * 1024;

/// Maps a library error to a Python exception: `Io` -> `OSError`, all others
/// -> `ValueError` carrying the error text.
fn to_py_err(error: SimError) -> PyErr {
    match error {
        SimError::Io(io) => PyOSError::new_err(io.to_string()),
        other => PyValueError::new_err(other.to_string()),
    }
}

fn check_size(input: &str) -> Result<(), SimError> {
    if input.len() > MAX_INPUT_BYTES {
        return Err(SimError::DimensionLimit {
            actual: input.len(),
            max: MAX_INPUT_BYTES,
        });
    }
    Ok(())
}

/// Parses and validates a scenario JSON string.
fn parse_scenario(input: &str) -> Result<Scenario, SimError> {
    check_size(input)?;
    let scenario: Scenario = serde_json::from_str(input)?;
    scenario.validate()?;
    Ok(scenario)
}

fn parse_manifest(input: &str) -> Result<ReplayManifest, SimError> {
    check_size(input)?;
    Ok(serde_json::from_str(input)?)
}

/// Result of a SIGINT run.
#[pyclass(module = "spectra_sim", frozen)]
struct SigintResult {
    /// Complex64 IQ samples (`i + jq`, from the f32 capture values).
    #[pyo3(get)]
    iq: Py<PyArray1<Complex32>>,
    /// Ground truth as a JSON string.
    #[pyo3(get)]
    truth_json: String,
    /// Replay manifest as a JSON string.
    #[pyo3(get)]
    manifest_json: String,
    #[pyo3(get)]
    sample_rate_hz: f64,
    #[pyo3(get)]
    center_frequency_hz: f64,
}

#[pymethods]
impl SigintResult {
    fn __repr__(&self, py: Python<'_>) -> String {
        format!(
            "SigintResult(samples={}, sample_rate_hz={}, center_frequency_hz={})",
            <Bound<'_, PyArray1<Complex32>> as numpy::PyUntypedArrayMethods>::len(self.iq.bind(py)),
            self.sample_rate_hz,
            self.center_frequency_hz
        )
    }
}

/// Runs the scenario's SIGINT job.
#[pyfunction]
fn run_sigint(py: Python<'_>, scenario_json: &str) -> PyResult<SigintResult> {
    let run = py
        .detach(|| {
            let scenario = parse_scenario(scenario_json)?;
            spectra_sim::run_sigint(&scenario)
        })
        .map_err(to_py_err)?;
    let capture = &run.simulation.capture;
    let samples: Vec<Complex32> = capture
        .samples
        .iter()
        .map(|sample| Complex32::new(sample.i, sample.q))
        .collect();
    let truth_json = serde_json::to_string(&run.simulation.truth)
        .map_err(|error| to_py_err(SimError::Json(error)))?;
    let manifest_json =
        serde_json::to_string(&run.manifest).map_err(|error| to_py_err(SimError::Json(error)))?;
    Ok(SigintResult {
        iq: PyArray1::from_vec(py, samples).unbind(),
        truth_json,
        manifest_json,
        sample_rate_hz: capture.sample_rate_hz,
        center_frequency_hz: capture.center_frequency_hz,
    })
}

/// Runs the scenario's C-UAS job and returns the run as a JSON string.
#[pyfunction]
fn run_cuas(py: Python<'_>, scenario_json: &str) -> PyResult<String> {
    py.detach(|| {
        let scenario = parse_scenario(scenario_json)?;
        let run = spectra_sim::run_cuas(&scenario)?;
        Ok(serde_json::to_string(&run)?)
    })
    .map_err(to_py_err)
}

/// Validates a scenario JSON string; raises `ValueError` when it is invalid.
#[pyfunction]
fn validate(scenario_json: &str) -> PyResult<()> {
    parse_scenario(scenario_json).map(|_| ()).map_err(to_py_err)
}

/// Re-runs the scenario against a replay manifest and returns the verdict:
/// `"match"`, `"scenario_mismatch"`, `"version_mismatch"`, `"schema_mismatch"`
/// or `"output_mismatch"`. `kind` is `"sigint"` or `"cuas"`.
#[pyfunction]
fn verify_replay(
    py: Python<'_>,
    scenario_json: &str,
    manifest_json: &str,
    kind: &str,
) -> PyResult<&'static str> {
    let kind = match kind {
        "sigint" => RunKind::Sigint,
        "cuas" => RunKind::Cuas,
        other => {
            return Err(PyValueError::new_err(format!(
                "invalid kind {other:?}: expected \"sigint\" or \"cuas\""
            )))
        }
    };
    let verdict = py
        .detach(|| {
            let scenario = parse_scenario(scenario_json)?;
            let manifest = parse_manifest(manifest_json)?;
            spectra_sim::verify_replay(&scenario, &manifest, kind)
        })
        .map_err(to_py_err)?;
    Ok(match verdict {
        ReplayVerdict::Match => "match",
        ReplayVerdict::ScenarioMismatch { .. } => "scenario_mismatch",
        ReplayVerdict::VersionMismatch { .. } => "version_mismatch",
        ReplayVerdict::SchemaMismatch { .. } => "schema_mismatch",
        ReplayVerdict::OutputMismatch { .. } => "output_mismatch",
        // `ReplayVerdict` is non-exhaustive.
        _ => "unknown",
    })
}

#[pymodule]
fn _spectra_sim(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add_class::<SigintResult>()?;
    m.add_function(wrap_pyfunction!(run_sigint, m)?)?;
    m.add_function(wrap_pyfunction!(run_cuas, m)?)?;
    m.add_function(wrap_pyfunction!(validate, m)?)?;
    m.add_function(wrap_pyfunction!(verify_replay, m)?)?;
    Ok(())
}
