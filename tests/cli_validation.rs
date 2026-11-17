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
        "sweep",
        "plot",
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
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(path.to_str().expect("utf-8 path")),
        "stderr={stderr}"
    );
    assert_eq!(stderr.trim().lines().count(), 1, "stderr={stderr}");
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

/// Unique scratch directory that is removed on drop, even when a test fails.
struct TempDir(std::path::PathBuf);

impl TempDir {
    fn new(test: &str) -> Self {
        let path = std::env::temp_dir().join(format!("spectra-{test}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        Self(path)
    }

    fn path(&self) -> &std::path::Path {
        &self.0
    }

    fn arg(&self) -> &str {
        self.0.to_str().expect("utf-8 path")
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn sorted_file_names(dir: &std::path::Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .expect("read dir")
        .map(|entry| {
            entry
                .expect("dir entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    names
}

const SIGINT_SCENARIO: &str = "examples/scenarios/sigint_urban.json";

#[test]
fn sweep_writes_one_dir_per_seed() {
    let dir = TempDir::new("sweep-dirs");
    let output = run_cli(&[
        "sweep",
        SIGINT_SCENARIO,
        "--kind",
        "sigint",
        "--seeds",
        "0..3",
        "--out",
        dir.arg(),
    ]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(
        sorted_file_names(dir.path()),
        ["seed_0", "seed_1", "seed_2", "sweep_manifest.json"]
    );

    let manifest: serde_json::Value = serde_json::from_slice(
        &std::fs::read(dir.path().join("sweep_manifest.json")).expect("read sweep manifest"),
    )
    .expect("sweep manifest is JSON");
    assert_eq!(manifest["schema_version"], "spectra-sim-sweep-v1");
    assert_eq!(manifest["kind"], "sigint");
    let entries = manifest["entries"].as_array().expect("entries array");
    let seeds: Vec<u64> = entries
        .iter()
        .map(|entry| entry["seed"].as_u64().expect("seed"))
        .collect();
    assert_eq!(seeds, [0, 1, 2]);

    // Each entry hash is the output hash from that seed directory's replay manifest.
    for entry in entries {
        let seed = entry["seed"].as_u64().expect("seed");
        let replay: serde_json::Value = serde_json::from_slice(
            &std::fs::read(dir.path().join(format!("seed_{seed}/replay_manifest.json")))
                .expect("read replay manifest"),
        )
        .expect("replay manifest is JSON");
        assert_eq!(replay["output_sha256"], entry["output_sha256"]);
        assert_eq!(replay["seed"], serde_json::json!(seed));
    }
}

#[test]
fn sweep_sigint_seed_dir_has_exact_file_set() {
    let dir = TempDir::new("sweep-sigint-files");
    let output = run_cli(&[
        "sweep",
        SIGINT_SCENARIO,
        "--kind",
        "sigint",
        "--seeds",
        "4..5",
        "--out",
        dir.arg(),
    ]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(
        sorted_file_names(&dir.path().join("seed_4")),
        [
            "iq_capture.json",
            "replay_manifest.json",
            "sigint_scenario.json",
            "truth.json"
        ]
    );
}

#[test]
fn sweep_cuas_seed_dir_has_exact_file_set() {
    let dir = TempDir::new("sweep-cuas-files");
    let output = run_cli(&[
        "sweep",
        "examples/scenarios/cuas_drone_isac.json",
        "--kind",
        "cuas",
        "--seeds",
        "0..2",
        "--out",
        dir.arg(),
    ]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(
        sorted_file_names(&dir.path().join("seed_1")),
        [
            "cuas_isac_scenario.json",
            "cuas_scenario.json",
            "isac_truth.json",
            "radar_truth.json",
            "replay_manifest.json"
        ]
    );
}

#[test]
fn sweep_is_independent_of_job_count() {
    let serial = TempDir::new("sweep-jobs1");
    let parallel = TempDir::new("sweep-jobs4");
    for (dir, jobs) in [(&serial, "1"), (&parallel, "4")] {
        let output = run_cli(&[
            "sweep",
            SIGINT_SCENARIO,
            "--kind",
            "sigint",
            "--seeds",
            "0..4",
            "--randomization",
            "examples/scenarios/randomization.json",
            "--jobs",
            jobs,
            "--out",
            dir.arg(),
        ]);
        assert_eq!(output.status.code(), Some(0), "{output:?}");
    }
    let a = std::fs::read(serial.path().join("sweep_manifest.json")).expect("read");
    let b = std::fs::read(parallel.path().join("sweep_manifest.json")).expect("read");
    assert_eq!(a, b);
}

#[test]
fn sweep_refuses_non_empty_out_without_force() {
    let dir = TempDir::new("sweep-force");
    std::fs::create_dir_all(dir.path()).expect("create dir");
    std::fs::write(dir.path().join("keep.txt"), "x").expect("write file");
    let args = |force: bool| {
        let mut args = vec![
            "sweep",
            SIGINT_SCENARIO,
            "--kind",
            "sigint",
            "--seeds",
            "0..1",
            "--out",
            dir.arg(),
        ];
        if force {
            args.push("--force");
        }
        run_cli(&args)
    };
    let refused = args(false);
    assert_eq!(refused.status.code(), Some(3), "{refused:?}");
    assert!(
        String::from_utf8_lossy(&refused.stderr).contains("not empty"),
        "{refused:?}"
    );
    assert!(!dir.path().join("sweep_manifest.json").exists());

    let forced = args(true);
    assert_eq!(forced.status.code(), Some(0), "{forced:?}");
    assert!(dir.path().join("sweep_manifest.json").exists());
    assert!(dir.path().join("keep.txt").exists());
}

#[test]
fn sweep_unwritable_out_exits_3() {
    let dir = TempDir::new("sweep-unwritable");
    std::fs::create_dir_all(dir.path()).expect("create dir");
    let blocker = dir.path().join("file");
    std::fs::write(&blocker, "x").expect("write file");
    let nested = blocker.join("out");
    let output = run_cli(&[
        "sweep",
        SIGINT_SCENARIO,
        "--kind",
        "sigint",
        "--seeds",
        "0..1",
        "--out",
        nested.to_str().expect("utf-8 path"),
    ]);
    assert_eq!(output.status.code(), Some(3), "{output:?}");
}

#[test]
fn sweep_rejects_bad_seeds_and_jobs() {
    for seeds in ["5..5", "10..2", "a..b", "0..1000000000", "3"] {
        let dir = TempDir::new("sweep-bad-seeds");
        let output = run_cli(&[
            "sweep",
            SIGINT_SCENARIO,
            "--kind",
            "sigint",
            "--seeds",
            seeds,
            "--out",
            dir.arg(),
        ]);
        assert_eq!(output.status.code(), Some(2), "seeds={seeds}: {output:?}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("--seeds"), "seeds={seeds}: {stderr}");
        assert!(!dir.path().exists(), "no output for bad seeds={seeds}");
    }
    let dir = TempDir::new("sweep-bad-jobs");
    let output = run_cli(&[
        "sweep",
        SIGINT_SCENARIO,
        "--kind",
        "sigint",
        "--seeds",
        "0..2",
        "--jobs",
        "0",
        "--out",
        dir.arg(),
    ]);
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(String::from_utf8_lossy(&output.stderr).contains("--jobs"));
}

#[test]
fn sweep_with_randomization_changes_hashes() {
    let dir = TempDir::new("sweep-randomization");
    let output = run_cli(&[
        "sweep",
        SIGINT_SCENARIO,
        "--kind",
        "sigint",
        "--seeds",
        "0..3",
        "--randomization",
        "examples/scenarios/randomization.json",
        "--out",
        dir.arg(),
    ]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let manifest: serde_json::Value = serde_json::from_slice(
        &std::fs::read(dir.path().join("sweep_manifest.json")).expect("read sweep manifest"),
    )
    .expect("sweep manifest is JSON");
    let mut hashes: Vec<&str> = manifest["entries"]
        .as_array()
        .expect("entries array")
        .iter()
        .map(|entry| entry["output_sha256"].as_str().expect("hash"))
        .collect();
    assert_eq!(hashes.len(), 3);
    hashes.sort_unstable();
    hashes.dedup();
    assert_eq!(hashes.len(), 3, "hashes must differ across seeds");
}

#[test]
fn sweep_help_lists_flags() {
    let output = run_cli(&["sweep", "--help"]);
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    for flag in [
        "--kind",
        "--seeds",
        "--randomization",
        "--jobs",
        "--out",
        "--force",
    ] {
        assert!(stdout.contains(flag), "missing {flag} in {stdout}");
    }
}

#[test]
fn replay_reports_version_mismatch() {
    let dir = TempDir::new("replay-version");
    let out = run_cli(&["sigint", SIGINT_SCENARIO, "--out", dir.arg()]);
    assert_eq!(out.status.code(), Some(0));
    let text = std::fs::read_to_string(dir.path().join("replay_manifest.json")).expect("read");
    let mut value: serde_json::Value = serde_json::from_str(&text).expect("json");
    value["simulator_version"] = serde_json::json!("0.0.0");
    let hash = value["output_sha256"].as_str().expect("hash").to_string();
    let flipped = if hash.starts_with('0') { "1" } else { "0" };
    value["output_sha256"] = serde_json::json!(format!("{flipped}{}", &hash[1..]));
    let manifest = dir.path().join("old_version_manifest.json");
    std::fs::write(&manifest, value.to_string()).expect("write manifest");

    let out = run_cli(&[
        "replay",
        SIGINT_SCENARIO,
        manifest.to_str().expect("utf-8 path"),
        "--kind",
        "sigint",
    ]);
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("version mismatch"), "stderr={stderr}");
    assert!(stderr.contains("0.0.0"), "stderr={stderr}");
}

#[test]
fn replay_reports_scenario_mismatch() {
    let dir = TempDir::new("replay-scenario");
    let out = run_cli(&["sigint", SIGINT_SCENARIO, "--out", dir.arg()]);
    assert_eq!(out.status.code(), Some(0));
    let source = std::fs::read_to_string(SIGINT_SCENARIO).expect("read scenario");
    let mut value: serde_json::Value = serde_json::from_str(&source).expect("json");
    let seed = value["seed"].as_u64().expect("scenario seed");
    value["seed"] = serde_json::json!(seed + 1);
    let changed = dir.path().join("changed_scenario.json");
    std::fs::write(&changed, value.to_string()).expect("write scenario");

    let out = run_cli(&[
        "replay",
        changed.to_str().expect("utf-8 path"),
        dir.path()
            .join("replay_manifest.json")
            .to_str()
            .expect("utf-8 path"),
        "--kind",
        "sigint",
    ]);
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("scenario mismatch"), "stderr={stderr}");
}

#[test]
fn replay_bad_manifest_json_names_the_file() {
    let dir = TempDir::new("replay-bad-manifest");
    std::fs::create_dir_all(dir.path()).expect("create dir");
    let manifest = dir.path().join("broken_manifest.json");
    std::fs::write(&manifest, "{").expect("write manifest");
    let manifest_arg = manifest.to_str().expect("utf-8 path");
    let out = run_cli(&["replay", SIGINT_SCENARIO, manifest_arg, "--kind", "sigint"]);
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains(manifest_arg), "stderr={stderr}");
    assert_eq!(stderr.trim().lines().count(), 1, "stderr={stderr}");
}

#[test]
fn replay_oversized_manifest_exits_2() {
    let dir = TempDir::new("replay-oversized");
    std::fs::create_dir_all(dir.path()).expect("create dir");
    let manifest = dir.path().join("huge_manifest.json");
    std::fs::write(&manifest, vec![b' '; 32 * 1024 * 1024 + 1]).expect("write manifest");
    let out = run_cli(&[
        "replay",
        SIGINT_SCENARIO,
        manifest.to_str().expect("utf-8 path"),
        "--kind",
        "sigint",
    ]);
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("dimension limit"), "stderr={stderr}");
    assert_eq!(stderr.trim().lines().count(), 1, "stderr={stderr}");
}

#[test]
fn replay_reports_schema_mismatch_for_legacy_manifest() {
    let dir = TempDir::new("replay-schema");
    let out = run_cli(&["sigint", SIGINT_SCENARIO, "--out", dir.arg()]);
    assert_eq!(out.status.code(), Some(0));
    let text = std::fs::read_to_string(dir.path().join("replay_manifest.json")).expect("read");
    let mut value: serde_json::Value = serde_json::from_str(&text).expect("json");
    value
        .as_object_mut()
        .expect("object")
        .remove("output_schema_version");
    let hash = value["output_sha256"].as_str().expect("hash").to_string();
    let flipped = if hash.starts_with('0') { "1" } else { "0" };
    value["output_sha256"] = serde_json::json!(format!("{flipped}{}", &hash[1..]));
    let manifest = dir.path().join("legacy_manifest.json");
    std::fs::write(&manifest, value.to_string()).expect("write manifest");

    let out = run_cli(&[
        "replay",
        SIGINT_SCENARIO,
        manifest.to_str().expect("utf-8 path"),
        "--kind",
        "sigint",
    ]);
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("schema mismatch"), "stderr={stderr}");
    assert!(stderr.contains("unversioned"), "stderr={stderr}");
    assert!(!stderr.contains("output mismatch"), "stderr={stderr}");
}

#[test]
fn replay_scenario_mismatch_mentions_differing_versions() {
    let dir = TempDir::new("replay-scenario-version");
    let out = run_cli(&["sigint", SIGINT_SCENARIO, "--out", dir.arg()]);
    assert_eq!(out.status.code(), Some(0));
    let text = std::fs::read_to_string(dir.path().join("replay_manifest.json")).expect("read");
    let mut value: serde_json::Value = serde_json::from_str(&text).expect("json");
    value["simulator_version"] = serde_json::json!("0.0.0");
    let hash = value["scenario_sha256"].as_str().expect("hash").to_string();
    let flipped = if hash.starts_with('0') { "1" } else { "0" };
    value["scenario_sha256"] = serde_json::json!(format!("{flipped}{}", &hash[1..]));
    let manifest = dir.path().join("old_manifest.json");
    std::fs::write(&manifest, value.to_string()).expect("write manifest");
    let out = run_cli(&[
        "replay",
        SIGINT_SCENARIO,
        manifest.to_str().expect("utf-8 path"),
        "--kind",
        "sigint",
    ]);
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("scenario mismatch"), "stderr={stderr}");
    assert!(
        stderr.contains("manifest from simulator 0.0.0"),
        "stderr={stderr}"
    );
}

#[test]
fn sweep_rejects_excessive_jobs_before_creating_output() {
    let dir = TempDir::new("sweep-huge-jobs");
    let output = run_cli(&[
        "sweep",
        SIGINT_SCENARIO,
        "--kind",
        "sigint",
        "--seeds",
        "0..2",
        "--jobs",
        "1000000",
        "--out",
        dir.arg(),
    ]);
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--jobs"), "stderr={stderr}");
    assert!(stderr.contains("1024"), "stderr={stderr}");
    assert!(!dir.path().exists(), "no output dir for rejected --jobs");
}

#[test]
fn sweep_accepts_the_maximum_job_count() {
    let dir = TempDir::new("sweep-max-jobs");
    let output = run_cli(&[
        "sweep",
        SIGINT_SCENARIO,
        "--kind",
        "sigint",
        "--seeds",
        "0..1",
        "--jobs",
        "1024",
        "--out",
        dir.arg(),
    ]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(dir.path().join("sweep_manifest.json").exists());
}

#[test]
fn failing_sweep_names_seed_and_leaves_no_manifest() {
    let dir = TempDir::new("sweep-failing");
    std::fs::create_dir_all(dir.path()).expect("create dir");
    // A stale manifest from an earlier sweep must not survive a failed forced run.
    let manifest = dir.path().join("sweep_manifest.json");
    std::fs::write(&manifest, "{\"bogus\":true}").expect("write stale manifest");
    let output = run_cli(&[
        "sweep",
        SIGINT_SCENARIO,
        "--kind",
        "cuas",
        "--seeds",
        "0..3",
        "--jobs",
        "1",
        "--force",
        "--out",
        dir.arg(),
    ]);
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("sweep aborted at seed 0"),
        "stderr={stderr}"
    );
    assert_eq!(stderr.trim().lines().count(), 1, "stderr={stderr}");
    assert!(!manifest.exists(), "stale sweep manifest must be removed");

    let fresh = TempDir::new("sweep-failing-fresh");
    let output = run_cli(&[
        "sweep",
        SIGINT_SCENARIO,
        "--kind",
        "cuas",
        "--seeds",
        "0..3",
        "--out",
        fresh.arg(),
    ]);
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(!fresh.path().join("sweep_manifest.json").exists());
}

#[test]
fn sweep_force_help_documents_manifest_index() {
    let output = run_cli(&["sweep", "--help"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("sweep_manifest.json"), "stdout={stdout}");
}

#[test]
fn single_run_output_path_that_is_a_file_exits_3_naming_it() {
    let dir = TempDir::new("single-out-file");
    std::fs::create_dir_all(dir.path()).expect("create dir");
    let blocker = dir.path().join("regular_file");
    std::fs::write(&blocker, "x").expect("write file");
    let blocker_arg = blocker.to_str().expect("utf-8 path");
    let output = run_cli(&["sigint", SIGINT_SCENARIO, "--out", blocker_arg]);
    assert_eq!(output.status.code(), Some(3), "{output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains(blocker_arg), "stderr={stderr}");
}

#[test]
fn single_run_unwritable_output_exits_3_naming_it() {
    let dir = TempDir::new("single-out-unwritable");
    std::fs::create_dir_all(dir.path()).expect("create dir");
    let blocker = dir.path().join("regular_file");
    std::fs::write(&blocker, "x").expect("write file");
    let nested = blocker.join("out");
    let nested_arg = nested.to_str().expect("utf-8 path");
    let output = run_cli(&["sigint", SIGINT_SCENARIO, "--out", nested_arg]);
    assert_eq!(output.status.code(), Some(3), "{output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains(nested_arg), "stderr={stderr}");
}

fn assert_validity_report(path: &std::path::Path) {
    let report = std::fs::read_to_string(path).expect("validity report exists");
    assert!(report.contains("# Physics validity report"), "{report}");
    assert!(report.contains("synthetic data"), "{report}");
}

#[test]
fn cli_writes_validity_report() {
    let sigint = TempDir::new("validity-report-sigint");
    let output = run_cli(&["sigint", SIGINT_SCENARIO, "--out", sigint.arg()]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_validity_report(&sigint.path().join("validity_report.md"));

    let cuas = TempDir::new("validity-report-cuas");
    let output = run_cli(&[
        "cuas",
        "examples/scenarios/cuas_drone_isac.json",
        "--out",
        cuas.arg(),
    ]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_validity_report(&cuas.path().join("validity_report.md"));
}

#[test]
fn sweep_does_not_write_validity_report() {
    let dir = TempDir::new("validity-report-sweep");
    let output = run_cli(&[
        "sweep",
        SIGINT_SCENARIO,
        "--kind",
        "sigint",
        "--seeds",
        "0..1",
        "--out",
        dir.arg(),
    ]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(!dir.path().join("seed_0/validity_report.md").exists());
}
