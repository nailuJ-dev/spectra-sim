//! Runs the drone ISAC counter-UAS example scenario and prints the radar measurement.
//!
//! Run with `cargo run --example quickstart_cuas`.

use spectra_sim::{run_cuas, Scenario};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/examples/scenarios/cuas_drone_isac.json"
    );
    let scenario: Scenario = serde_json::from_str(&std::fs::read_to_string(path)?)?;
    let run = run_cuas(&scenario)?;
    let radar = &run.radar_measurement;
    println!("range_m: {:.1}", radar.range_m);
    println!("radial_velocity_mps: {:.2}", radar.radial_velocity_mps);
    println!("snr_db: {:.2}", radar.snr_db);
    Ok(())
}
