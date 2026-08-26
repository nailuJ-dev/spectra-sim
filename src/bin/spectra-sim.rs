use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use spectra_sim::{run_cuas, run_sigint, sigint_golden_scenario, DomainRandomization, Result, Scenario, SimError};

fn main() {
    if let Err(error) = run() {
        eprintln!("spectra-sim: {error}");
        std::process::exit(2);
    }
}

fn run() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 { return Err(SimError::InvalidArgument(usage().into())); }
    match args[1].as_str() {
        "sigint" => {
            let scenario = load_scenario(&args[2])?;
            let output_dir = option_value(&args, "--out").unwrap_or_else(|| "artifacts/sigint".into());
            let run = run_sigint(&scenario)?;
            fs::create_dir_all(&output_dir)?;
            write_json(Path::new(&output_dir).join("iq_capture.json"), &run.simulation.capture)?;
            let sdk_scenario = sigint_golden_scenario(&scenario, &run.simulation.capture)?;
            write_json(Path::new(&output_dir).join("sigint_scenario.json"), &sdk_scenario)?;
            write_json(Path::new(&output_dir).join("truth.json"), &run.simulation.truth)?;
            write_json(Path::new(&output_dir).join("replay_manifest.json"), &run.manifest)?;
        }
        "cuas" => {
            let scenario = load_scenario(&args[2])?;
            let output_dir = option_value(&args, "--out").unwrap_or_else(|| "artifacts/cuas".into());
            let run = run_cuas(&scenario)?;
            fs::create_dir_all(&output_dir)?;
            write_json(Path::new(&output_dir).join("radar_truth.json"), &run.radar_measurement)?;
            write_json(Path::new(&output_dir).join("cuas_scenario.json"), &run.recorded_scenario)?;
            if let Some(value) = &run.isac_measurement { write_json(Path::new(&output_dir).join("isac_truth.json"), value)?; }
            if let Some(value) = &run.isac_scenario { write_json(Path::new(&output_dir).join("cuas_isac_scenario.json"), value)?; }
            write_json(Path::new(&output_dir).join("replay_manifest.json"), &run.manifest)?;
        }
        "randomize" => {
            if args.len() < 4 { return Err(SimError::InvalidArgument("randomize requires <scenario.json> <randomization.json>".into())); }
            let scenario = load_scenario(&args[2])?;
            let config: DomainRandomization = serde_json::from_slice(&fs::read(&args[3])?)?;
            let seed = option_value(&args, "--seed").and_then(|v| v.parse().ok()).unwrap_or(scenario.seed);
            let out_path = option_value(&args, "--out").unwrap_or_else(|| "artifacts/randomized_scenario.json".into());
            let randomized = config.apply(&scenario, seed)?;
            if let Some(parent) = Path::new(&out_path).parent() { fs::create_dir_all(parent)?; }
            write_json(PathBuf::from(out_path), &randomized)?;
        }
        _ => return Err(SimError::InvalidArgument(usage().into())),
    }
    Ok(())
}

fn load_scenario(path: &str) -> Result<Scenario> {
    let bytes = fs::read(path)?;
    if bytes.len() > 32 * 1024 * 1024 { return Err(SimError::DimensionLimit { actual: bytes.len(), max: 32 * 1024 * 1024 }); }
    let scenario: Scenario = serde_json::from_slice(&bytes)?;
    scenario.validate()?;
    Ok(scenario)
}

fn option_value(args: &[String], name: &str) -> Option<String> {
    args.windows(2).find(|pair| pair[0] == name).map(|pair| pair[1].clone())
}

fn write_json(path: PathBuf, value: &impl serde::Serialize) -> Result<()> {
    fs::write(path, serde_json::to_vec_pretty(value)?)?;
    Ok(())
}

fn usage() -> &'static str {
    "usage: spectra-sim sigint <scenario.json> [--out DIR] | cuas <scenario.json> [--out DIR] | randomize <scenario.json> <randomization.json> [--seed N] [--out FILE]"
}
