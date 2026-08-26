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
