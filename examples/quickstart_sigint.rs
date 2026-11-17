//! Runs the urban SIGINT example scenario and prints the link SNR.
//!
//! Run with `cargo run --example quickstart_sigint`.

use spectra_sim::{run_sigint, Scenario};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/examples/scenarios/sigint_urban.json"
    );
    let scenario: Scenario = serde_json::from_str(&std::fs::read_to_string(path)?)?;
    let run = run_sigint(&scenario)?;
    let truth = &run.simulation.truth;
    println!(
        "emitter {} -> receiver {}",
        truth.emitter_id, truth.receiver_id
    );
    println!("distance_m: {:.1}", truth.distance_m);
    println!("snr_db: {:.2}", truth.snr_db);
    println!("samples: {}", run.simulation.capture.samples.len());
    Ok(())
}
