use spectra_sim::{ModelMaturity, PhysicsQuality, ValidityIssue, ValidityLedger};

#[test]
fn quality_escalates_from_warnings_to_extrapolated() {
    let mut ledger = ValidityLedger::default();
    assert_eq!(ledger.quality(), PhysicsQuality::Validated);
    ledger.push(ValidityIssue::warning("eop", "stale EOP"));
    assert_eq!(ledger.quality(), PhysicsQuality::ValidatedWithWarnings);
    ledger.push(ValidityIssue::extrapolation(
        "water",
        "frequency outside reference range",
    ));
    assert_eq!(ledger.quality(), PhysicsQuality::Extrapolated);
}

#[test]
fn experimental_model_marks_simulation_experimental() {
    let mut ledger = ValidityLedger::default();
    ledger.record_model(
        "surface-wave",
        "0.1",
        "arXiv:2402.17787",
        ModelMaturity::ExperimentalResearch,
    );
    assert_eq!(ledger.quality(), PhysicsQuality::Experimental);
}

#[test]
fn serialization_is_deterministic_for_identical_ledgers() {
    let mut a = ValidityLedger::default();
    a.push(ValidityIssue::warning("orbit", "missing covariance"));
    let b = a.clone();
    assert_eq!(
        serde_json::to_vec(&a).unwrap(),
        serde_json::to_vec(&b).unwrap()
    );
}
