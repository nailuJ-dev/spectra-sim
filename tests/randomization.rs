use spectra_sim::{DomainRandomization, Scenario};

#[test]
fn domain_randomization_is_reproducible() {
    let scenario: Scenario = serde_json::from_slice(
        &std::fs::read("examples/scenarios/cuas_drone_isac.json").unwrap_or_default(),
    )
    .unwrap_or_else(|_| panic!("scenario"));
    let cfg: DomainRandomization = serde_json::from_slice(
        &std::fs::read("examples/scenarios/randomization.json").unwrap_or_default(),
    )
    .unwrap_or_else(|_| panic!("config"));
    let a = cfg.apply(&scenario, 1234).unwrap_or_else(|_| panic!("a"));
    let b = cfg.apply(&scenario, 1234).unwrap_or_else(|_| panic!("b"));
    assert_eq!(a, b);
    assert_ne!(a, scenario);
}
