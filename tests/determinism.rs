use spectra_sim::{run_cuas, run_sigint, Scenario};

fn load(path: &str) -> Scenario {
    serde_json::from_slice(&std::fs::read(path).unwrap_or_default())
        .unwrap_or_else(|_| panic!("fixture must deserialize"))
}

#[test]
fn sigint_replay_is_byte_stable_for_same_seed() {
    let scenario = load("examples/scenarios/sigint_urban.json");
    let left = run_sigint(&scenario).unwrap_or_else(|_| panic!("first run"));
    let right = run_sigint(&scenario).unwrap_or_else(|_| panic!("second run"));
    assert_eq!(left.manifest, right.manifest);
    assert_eq!(left.simulation.capture, right.simulation.capture);
}

#[test]
fn cuas_replay_is_byte_stable_for_same_seed() {
    let scenario = load("examples/scenarios/cuas_drone_isac.json");
    let left = run_cuas(&scenario).unwrap_or_else(|_| panic!("first run"));
    let right = run_cuas(&scenario).unwrap_or_else(|_| panic!("second run"));
    assert_eq!(left.manifest, right.manifest);
    assert_eq!(left.recorded_scenario, right.recorded_scenario);
}
