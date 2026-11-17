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

fn run_cli(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_spectra-sim"))
        .args(args)
        .output()
        .expect("spectra-sim binary starts")
}

#[test]
fn help_lists_subcommands() {
    let output = run_cli(&["--help"]);
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    for name in ["sigint", "cuas", "randomize"] {
        assert!(stdout.contains(name), "missing {name} in stdout={stdout}");
    }
}

#[test]
fn version_flag() {
    let output = run_cli(&["--version"]);
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(env!("CARGO_PKG_VERSION")),
        "stdout={stdout}"
    );
}

#[test]
fn missing_scenario_file_exits_3() {
    let output = run_cli(&["sigint", "/nonexistent.json"]);
    assert_eq!(output.status.code(), Some(3));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("/nonexistent.json"), "stderr={stderr}");
}

#[test]
fn bad_json_exits_2() {
    let path = std::env::temp_dir().join(format!("spectra-bad-json-{}.json", std::process::id()));
    std::fs::write(&path, "{").expect("write temp file");
    let output = run_cli(&["sigint", path.to_str().expect("utf-8 path")]);
    let _ = std::fs::remove_file(&path);
    assert_eq!(output.status.code(), Some(2));
}
