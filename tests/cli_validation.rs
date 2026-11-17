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
    for name in [
        "sigint",
        "cuas",
        "randomize",
        "validate",
        "replay",
        "examples",
    ] {
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

fn temp_path(test: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("spectra-{test}-{}.json", std::process::id()))
}

#[test]
fn validate_accepts_all_example_scenarios() {
    let mut checked = 0;
    for entry in std::fs::read_dir("examples/scenarios").expect("examples dir") {
        let path = entry.expect("dir entry").path();
        let is_json = path.extension().and_then(|e| e.to_str()) == Some("json");
        let is_randomization =
            path.file_name().and_then(|n| n.to_str()) == Some("randomization.json");
        if !is_json || is_randomization {
            continue;
        }
        let output = run_cli(&["validate", path.to_str().expect("utf-8 path")]);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{}: stderr={stderr}",
            path.display()
        );
        assert!(
            stdout.starts_with("ok:"),
            "{}: stdout={stdout}",
            path.display()
        );
        checked += 1;
    }
    assert!(checked > 0, "no example scenarios found");
}

#[test]
fn validate_rejects_unknown_field() {
    let source = std::fs::read_to_string("examples/scenarios/sigint_ofdm.json").expect("read");
    let mut value: serde_json::Value = serde_json::from_str(&source).expect("json");
    value["bogus_field"] = serde_json::json!(1);
    let path = temp_path("validate-unknown-field");
    std::fs::write(&path, value.to_string()).expect("write temp file");
    let output = run_cli(&["validate", path.to_str().expect("utf-8 path")]);
    let _ = std::fs::remove_file(&path);
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unknown field"), "stderr={stderr}");
    assert_eq!(stderr.trim().lines().count(), 1, "stderr={stderr}");
}

#[test]
fn validate_rejects_duplicate_ids() {
    let source = std::fs::read_to_string("examples/scenarios/sigint_ofdm.json").expect("read");
    let mut value: serde_json::Value = serde_json::from_str(&source).expect("json");
    let first = value["emitters"][0].clone();
    let id = first["id"].as_str().expect("emitter id").to_string();
    value["emitters"]
        .as_array_mut()
        .expect("emitters array")
        .push(first);
    let path = temp_path("validate-duplicate-ids");
    std::fs::write(&path, value.to_string()).expect("write temp file");
    let output = run_cli(&["validate", path.to_str().expect("utf-8 path")]);
    let _ = std::fs::remove_file(&path);
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains(&id), "stderr={stderr}");
    assert!(stderr.contains("duplicate"), "stderr={stderr}");
}

#[test]
fn validate_json_flag() {
    let output = run_cli(&[
        "validate",
        "examples/scenarios/cuas_drone_isac.json",
        "--json",
    ]);
    assert_eq!(output.status.code(), Some(0));
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("stdout is JSON");
    assert_eq!(value["valid"], serde_json::json!(true));
    assert_eq!(value["jobs"], serde_json::json!(["cuas"]));
}

#[test]
fn validate_prints_exact_summary_line() {
    let output = run_cli(&["validate", "examples/scenarios/sigint_urban.json"]);
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        stdout,
        "ok: 1 emitters, 1 receivers, 0 targets, jobs: [sigint]\n"
    );
}

#[test]
fn validate_empty_file_exits_2_with_one_line() {
    let path = temp_path("validate-empty");
    std::fs::write(&path, "").expect("write temp file");
    let output = run_cli(&["validate", path.to_str().expect("utf-8 path")]);
    let _ = std::fs::remove_file(&path);
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(stderr.trim().lines().count(), 1, "stderr={stderr}");
}

#[test]
fn validate_oversized_file_exits_2() {
    let path = temp_path("validate-oversized");
    std::fs::write(&path, vec![b' '; 32 * 1024 * 1024 + 1]).expect("write temp file");
    let output = run_cli(&["validate", path.to_str().expect("utf-8 path")]);
    let _ = std::fs::remove_file(&path);
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(stderr.trim().lines().count(), 1, "stderr={stderr}");
}

#[test]
fn randomize_negative_seed_keeps_legacy_message() {
    let output = run_cli(&[
        "randomize",
        "examples/scenarios/cuas_drone_isac.json",
        "examples/scenarios/randomization.json",
        "--seed",
        "-5",
    ]);
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--seed must be an unsigned integer: -5"),
        "stderr={stderr}"
    );
}

#[test]
fn examples_list_has_eight_entries() {
    let output = run_cli(&["examples", "list"]);
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 8, "stdout={stdout}");
    assert!(lines.contains(&"sigint_urban"), "stdout={stdout}");
    let mut sorted = lines.clone();
    sorted.sort_unstable();
    assert_eq!(lines, sorted, "list must be sorted");
}

#[test]
fn examples_show_round_trips() {
    let output = run_cli(&["examples", "show", "sigint_urban"]);
    assert_eq!(output.status.code(), Some(0));
    let expected = std::fs::read(format!(
        "{}/examples/scenarios/sigint_urban.json",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("read example");
    assert_eq!(output.stdout, expected);
}

#[test]
fn examples_show_unknown_exits_2() {
    let output = run_cli(&["examples", "show", "no_such_example"]);
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("no_such_example"), "stderr={stderr}");
}

#[test]
fn replay_cli_exit_codes() {
    let dir = std::env::temp_dir().join(format!("spectra-replay-cli-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let scenario = "examples/scenarios/sigint_urban.json";
    let out = run_cli(&["sigint", scenario, "--out", dir.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0));
    let manifest = dir.join("replay_manifest.json");
    let manifest_arg = manifest.to_str().unwrap();

    let ok = run_cli(&["replay", scenario, manifest_arg, "--kind", "sigint"]);
    assert_eq!(ok.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&ok.stdout).contains("MATCH"));

    let text = std::fs::read_to_string(&manifest).unwrap();
    let value: serde_json::Value = serde_json::from_str(&text).unwrap();
    let hash = value["output_sha256"].as_str().unwrap().to_string();
    let flipped = if hash.starts_with('0') { "1" } else { "0" };
    let tampered = text.replace(&hash, &format!("{flipped}{}", &hash[1..]));
    let bad = dir.join("tampered.json");
    std::fs::write(&bad, tampered).unwrap();
    let out = run_cli(&[
        "replay",
        scenario,
        bad.to_str().unwrap(),
        "--kind",
        "sigint",
    ]);
    assert_eq!(out.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("output mismatch"), "stderr={stderr}");

    let missing = run_cli(&[
        "replay",
        scenario,
        "/nonexistent-manifest.json",
        "--kind",
        "sigint",
    ]);
    assert_eq!(missing.status.code(), Some(3));

    let no_kind = run_cli(&["replay", scenario, manifest_arg]);
    assert_eq!(no_kind.status.code(), Some(2));
    std::fs::remove_dir_all(&dir).ok();
}
