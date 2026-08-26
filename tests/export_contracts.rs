use spectra_sim::{run_cuas, run_sigint, GoldenCuasSourceOut, Scenario};

fn fixture(path: &str) -> Scenario {
    let bytes = std::fs::read(path).unwrap_or_default();
    serde_json::from_slice(&bytes).unwrap_or_else(|_| panic!("valid fixture"))
}

#[test]
fn sigint_export_matches_public_iq_contract() {
    let run = run_sigint(&fixture("examples/scenarios/sigint_urban.json"))
        .unwrap_or_else(|_| panic!("sigint simulation"));
    let value = serde_json::to_value(&run.simulation.capture).unwrap_or_default();
    assert!(value.get("sample_rate_hz").is_some());
    assert!(value.get("center_frequency_hz").is_some());
    assert!(value
        .get("samples")
        .and_then(|v| v.as_array())
        .is_some_and(|v| !v.is_empty()));
}

#[test]
fn cuas_export_has_thirteen_isac_features() {
    let run = run_cuas(&fixture("examples/scenarios/cuas_drone_isac.json"))
        .unwrap_or_else(|_| panic!("cuas simulation"));
    let scenario = run.isac_scenario.unwrap_or_else(|| panic!("isac scenario"));
    match scenario.source {
        GoldenCuasSourceOut::IsacRecorded { input } => assert_eq!(input.features.len(), 13),
        _ => panic!("expected isac source"),
    }
}
