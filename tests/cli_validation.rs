use std::process::Command;

#[test]
fn malformed_seed_is_rejected_instead_of_falling_back_to_scenario_seed() {
    let output = Command::new(env!("CARGO_BIN_EXE_spectra-sim"))
        .args([
            "randomize",
            "examples/scenarios/cuas_drone_isac.json",
            "examples/scenarios/randomization.json",
            "--seed",
            "900l",
            "--out",
            "/tmp/spectra-invalid-seed.json",
        ])
        .output()
        .expect("spectra-sim binary starts");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--seed must be an unsigned integer: 900l"),
        "stderr={stderr}"
    );
}
