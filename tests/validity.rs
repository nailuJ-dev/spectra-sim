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

mod report_tests {
    use spectra_sim::report::{scenario_validity_ledger, validity_report_markdown};
    use spectra_sim::{
        ModelMaturity, PhysicsQuality, PropagationModel, Scenario, ValidityIssue, ValidityLedger,
    };

    fn scenario() -> Scenario {
        let text = std::fs::read_to_string("examples/scenarios/sigint_urban.json")
            .expect("read example scenario");
        serde_json::from_str(&text).expect("parse example scenario")
    }

    #[test]
    fn report_lists_models_and_quality() {
        let mut ledger = ValidityLedger::default();
        ledger.record_model(
            "free_space",
            "1",
            "Friis equation",
            ModelMaturity::ValidatedReference,
        );
        ledger.push(ValidityIssue::warning("dataset", "synthetic data only"));
        let report = validity_report_markdown(&ledger);
        assert!(report.starts_with("# Physics validity report"), "{report}");
        assert!(report.contains("free_space"), "{report}");
        assert!(
            report.contains("Overall quality: VALIDATED_WITH_WARNINGS"),
            "{report}"
        );
        assert!(
            report.contains("| Model | Maturity | Reference | Validity range |"),
            "{report}"
        );
        assert!(
            report.contains("- warning — dataset: synthetic data only"),
            "{report}"
        );
        assert!(!report.contains("No issues recorded."), "{report}");
    }

    #[test]
    fn empty_ledger_report() {
        let report = validity_report_markdown(&ValidityLedger::default());
        assert!(report.contains("Overall quality: VALIDATED"), "{report}");
        assert!(report.contains("No issues recorded."), "{report}");
    }

    #[test]
    fn report_escapes_pipes_in_cells() {
        let mut ledger = ValidityLedger::default();
        ledger.record_model(
            "a|b",
            "1",
            "ref | with pipe",
            ModelMaturity::ValidatedReference,
        );
        let report = validity_report_markdown(&ledger);
        assert!(report.contains("a\\|b"), "{report}");
        assert!(report.contains("ref \\| with pipe"), "{report}");
    }

    #[test]
    fn scenario_ledger_free_space_vs_two_ray() {
        let mut scenario = scenario();
        scenario.propagation = PropagationModel::FreeSpace;
        let free = scenario_validity_ledger(&scenario);
        assert_eq!(free.models().len(), 1);
        assert_eq!(free.models()[0].name, "free_space");
        assert_eq!(free.models()[0].maturity, ModelMaturity::ValidatedReference);
        // Only the always-on synthetic-data warning applies.
        assert_eq!(free.issues().len(), 1);
        assert_eq!(free.quality(), PhysicsQuality::ValidatedWithWarnings);

        scenario.propagation = PropagationModel::TwoRay;
        let two_ray = scenario_validity_ledger(&scenario);
        assert_eq!(two_ray.models()[0].name, "two_ray");
        assert_eq!(
            two_ray.models()[0].maturity,
            ModelMaturity::ValidatedWithWarnings
        );
        assert_eq!(two_ray.quality(), PhysicsQuality::ValidatedWithWarnings);
        assert!(two_ray.issues().len() > free.issues().len());

        scenario.propagation = PropagationModel::Rician {
            k_factor_db: 6.0,
            shadowing_std_db: 2.0,
        };
        let rician = scenario_validity_ledger(&scenario);
        assert_eq!(rician.models()[0].name, "rician");
        assert!(rician.issues().len() > free.issues().len());
    }
}
