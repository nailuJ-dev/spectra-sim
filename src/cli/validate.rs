//! `validate` subcommand: parse and check a scenario without simulating it.

use std::path::Path;

use spectra_sim::{Result, Scenario};

use super::io::load_scenario;

/// Names of the jobs present in the scenario, in a stable order.
fn job_names(scenario: &Scenario) -> Vec<&'static str> {
    let mut jobs = Vec::new();
    if scenario.sigint_job.is_some() {
        jobs.push("sigint");
    }
    if scenario.cuas_job.is_some() {
        jobs.push("cuas");
    }
    jobs
}

/// Loads and validates `path`, printing a one-line summary on success.
pub fn run(path: &Path, json: bool) -> Result<()> {
    let scenario = load_scenario(path)?;
    let jobs = job_names(&scenario);
    if json {
        println!("{}", serde_json::json!({ "valid": true, "jobs": jobs }));
    } else {
        println!(
            "ok: {} emitters, {} receivers, {} targets, jobs: [{}]",
            scenario.emitters.len(),
            scenario.receivers.len(),
            scenario.targets.len(),
            jobs.join(", ")
        );
    }
    Ok(())
}
