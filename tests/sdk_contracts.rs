use spectra_sim::{run_sigint, sigint_golden_scenario, Scenario};

#[test]
fn sigint_export_matches_golden_path_wrapper_contract() {
    let scenario: Scenario =
        serde_json::from_str(include_str!("../examples/scenarios/sigint_urban.json"))
            .expect("fixture must deserialize");
    let run = run_sigint(&scenario).expect("fixture must simulate");
    let golden = sigint_golden_scenario(&scenario, &run.simulation.capture)
        .expect("wrapper must be produced");
    assert_eq!(golden.expected_label.as_deref(), Some("pulsed_carrier"));
    assert_eq!(golden.capture.id, run.simulation.capture.id);
}

#[test]
fn ofdm_export_does_not_claim_an_untrained_reference_label() {
    let scenario: Scenario =
        serde_json::from_str(include_str!("../examples/scenarios/sigint_ofdm.json"))
            .expect("fixture must deserialize");
    let run = run_sigint(&scenario).expect("fixture must simulate");
    let golden = sigint_golden_scenario(&scenario, &run.simulation.capture)
        .expect("wrapper must be produced");
    assert_eq!(golden.expected_label, None);
}

fn urban_run() -> spectra_sim::SigintRun {
    let scenario: Scenario =
        serde_json::from_str(include_str!("../examples/scenarios/sigint_urban.json"))
            .expect("fixture must deserialize");
    run_sigint(&scenario).expect("fixture must simulate")
}

#[test]
fn manifest_carries_output_schema_version() {
    let run = urban_run();
    assert_eq!(run.manifest.output_schema_version, "spectra-sim-output-v1");
}

#[test]
fn legacy_manifest_without_schema_field_still_parses() {
    let run = urban_run();
    let mut value = serde_json::to_value(&run.manifest).expect("manifest must serialize");
    value
        .as_object_mut()
        .expect("manifest is an object")
        .remove("output_schema_version");
    let legacy: spectra_sim::ReplayManifest =
        serde_json::from_value(value).expect("legacy manifest must parse");
    assert_eq!(legacy.output_schema_version, "unversioned");
}

#[test]
fn output_hash_is_unchanged() {
    let run = urban_run();
    assert_eq!(
        run.manifest.output_sha256,
        "a40a02e982b9d8223132993f27274fc26cc86516b0cff5c507b94a34f6766cc2"
    );
}
