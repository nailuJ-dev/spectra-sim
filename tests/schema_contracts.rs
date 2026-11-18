#![cfg(feature = "schema")]

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use serde_json::Value;
use spectra_sim::{
    cuas_run_schema, pass_config_schema, pass_result_schema, scenario_schema, sigint_run_schema,
};

fn repo_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
}

fn read_json(relative: &str) -> Value {
    let text = fs::read_to_string(repo_path(relative)).expect("file is readable");
    serde_json::from_str(&text).expect("file is valid JSON")
}

#[test]
fn committed_schemas_are_current() {
    assert_eq!(
        scenario_schema(),
        read_json("schemas/scenario.schema.json"),
        "regenerate with: cargo run -q -- schema scenario > schemas/scenario.schema.json"
    );
    assert_eq!(
        sigint_run_schema(),
        read_json("schemas/sigint_run.schema.json"),
        "regenerate with: cargo run -q -- schema sigint-run > schemas/sigint_run.schema.json"
    );
    assert_eq!(
        cuas_run_schema(),
        read_json("schemas/cuas_run.schema.json"),
        "regenerate with: cargo run -q -- schema cuas-run > schemas/cuas_run.schema.json"
    );
    assert_eq!(
        pass_config_schema(),
        read_json("schemas/pass_config.schema.json"),
        "regenerate with: cargo run -q -- schema pass-config > schemas/pass_config.schema.json"
    );
    assert_eq!(
        pass_result_schema(),
        read_json("schemas/pass_result.schema.json"),
        "regenerate with: cargo run -q -- schema pass-result > schemas/pass_result.schema.json"
    );
}

#[test]
fn example_pass_config_matches_schema() {
    let schema = pass_config_schema();
    let config = read_json("examples/space/vanguard1_pass.json");
    let properties = schema["properties"].as_object().expect("properties");
    for key in config.as_object().expect("object").keys() {
        assert!(properties.contains_key(key), "undeclared key {key}");
    }
    for required in schema["required"].as_array().expect("required") {
        let key = required.as_str().expect("string");
        assert!(config.get(key).is_some(), "missing required key {key}");
    }
    assert_eq!(schema["additionalProperties"], Value::Bool(false));
}

#[test]
fn schemas_are_deterministic() {
    assert_eq!(scenario_schema(), scenario_schema());
    assert_eq!(
        serde_json::to_string(&cuas_run_schema()).expect("serializes"),
        serde_json::to_string(&cuas_run_schema()).expect("serializes")
    );
}

#[test]
fn every_example_scenario_matches_schema() {
    let schema = scenario_schema();
    let properties = schema["properties"]
        .as_object()
        .expect("scenario schema declares properties");
    let required: Vec<&str> = schema["required"]
        .as_array()
        .expect("scenario schema declares required keys")
        .iter()
        .map(|value| value.as_str().expect("required entries are strings"))
        .collect();

    let dir = repo_path("examples/scenarios");
    let mut checked = 0;
    for entry in fs::read_dir(dir).expect("examples dir is readable") {
        let path = entry.expect("dir entry").path();
        let name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_string();
        // randomization.json is a DomainRandomization config, not a scenario.
        if path.extension().and_then(|value| value.to_str()) != Some("json")
            || name == "randomization.json"
        {
            continue;
        }
        let value: Value =
            serde_json::from_str(&fs::read_to_string(&path).expect("example is readable"))
                .expect("example is valid JSON");
        let object = value.as_object().expect("scenario is an object");
        for key in object.keys() {
            assert!(
                properties.contains_key(key),
                "{name}: key {key} is not declared in the scenario schema"
            );
        }
        for key in &required {
            assert!(object.contains_key(*key), "{name}: missing required {key}");
        }
        checked += 1;
    }
    assert!(
        checked >= 6,
        "expected the example scenarios, got {checked}"
    );
}

#[test]
fn schema_cli_prints_json() {
    for kind in ["scenario", "sigint-run", "cuas-run"] {
        let output = Command::new(env!("CARGO_BIN_EXE_spectra-sim"))
            .args(["schema", kind])
            .output()
            .expect("spectra-sim binary starts");
        assert_eq!(output.status.code(), Some(0), "schema {kind}");
        let value: Value = serde_json::from_slice(&output.stdout).expect("stdout is JSON");
        assert!(
            value.get("$schema").is_some(),
            "schema {kind} lacks $schema"
        );
    }
}

#[test]
fn schema_cli_rejects_unknown_kind() {
    let output = Command::new(env!("CARGO_BIN_EXE_spectra-sim"))
        .args(["schema", "bogus"])
        .output()
        .expect("spectra-sim binary starts");
    assert_eq!(output.status.code(), Some(2));
}
