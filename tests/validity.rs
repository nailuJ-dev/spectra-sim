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
    fn scenario_ledger_states_its_scope() {
        let mut scenario = scenario();
        for propagation in [
            PropagationModel::FreeSpace,
            PropagationModel::TwoRay,
            PropagationModel::Rician {
                k_factor_db: 6.0,
                shadowing_std_db: 2.0,
            },
        ] {
            scenario.propagation = propagation;
            let ledger = scenario_validity_ledger(&scenario);
            let scope: Vec<_> = ledger
                .issues()
                .iter()
                .filter(|issue| issue.component == "scope")
                .collect();
            assert_eq!(scope.len(), 1, "{:?}", ledger.issues());
            assert!(
                scope[0]
                    .message
                    .contains("models are catalogued from the scenario configuration"),
                "{}",
                scope[0].message
            );
        }
    }

    fn names(ledger: &ValidityLedger) -> Vec<&str> {
        ledger.models().iter().map(|m| m.name.as_str()).collect()
    }

    fn maturity(ledger: &ValidityLedger, name: &str) -> ModelMaturity {
        ledger
            .models()
            .iter()
            .find(|m| m.name == name)
            .unwrap_or_else(|| panic!("model {name} missing: {:?}", names(ledger)))
            .maturity
    }

    #[test]
    fn sigint_ledger_catalogues_waveform_receiver_and_antennas() {
        let ledger = scenario_validity_ledger(&scenario());
        let names = names(&ledger);
        for expected in ["thermal_noise_ktb", "receiver_impairments"] {
            assert!(names.contains(&expected), "{names:?}");
        }
        assert!(
            names.iter().any(|n| n.starts_with("waveform_")),
            "{names:?}"
        );
        assert!(names.iter().any(|n| n.starts_with("antenna_")), "{names:?}");
        assert_eq!(
            maturity(&ledger, "receiver_impairments"),
            ModelMaturity::ValidatedWithWarnings
        );
        assert!(ledger.issues().iter().any(|i| i.component == "receiver"));
        // No CUAS job, so no radar models.
        assert!(!names.contains(&"monostatic_radar_equation"), "{names:?}");
    }

    #[test]
    fn cuas_ledger_catalogues_radar_isac_and_flags_micro_doppler() {
        let text = std::fs::read_to_string("examples/scenarios/cuas_drone_isac.json").unwrap();
        let scenario: Scenario = serde_json::from_str(&text).unwrap();
        let ledger = scenario_validity_ledger(&scenario);
        let names = names(&ledger);
        for expected in [
            "monostatic_radar_equation",
            "rcs_constant",
            "array_angle_resolution",
            "micro_doppler_index",
            "ofdm_isac_resolution",
        ] {
            assert!(names.contains(&expected), "{names:?}");
        }
        assert_eq!(
            maturity(&ledger, "micro_doppler_index"),
            ModelMaturity::ExperimentalResearch
        );
        assert_eq!(ledger.quality(), PhysicsQuality::Experimental);
        // Receiver chain belongs to SIGINT jobs only.
        assert!(!names.contains(&"receiver_impairments"), "{names:?}");
    }

    #[test]
    fn cuas_ledger_without_rotors_has_no_micro_doppler_model() {
        let text = std::fs::read_to_string("examples/scenarios/cuas_bird.json").unwrap();
        let scenario: Scenario = serde_json::from_str(&text).unwrap();
        let ledger = scenario_validity_ledger(&scenario);
        assert!(!names(&ledger).contains(&"micro_doppler_index"));
        assert_ne!(ledger.quality(), PhysicsQuality::Experimental);
    }

    #[test]
    fn models_are_catalogued_once() {
        let text = std::fs::read_to_string("examples/scenarios/cuas_drone_isac.json").unwrap();
        let scenario: Scenario = serde_json::from_str(&text).unwrap();
        let ledger = scenario_validity_ledger(&scenario);
        let mut all = names(&ledger);
        let before = all.len();
        all.sort_unstable();
        all.dedup();
        assert_eq!(all.len(), before, "{all:?}");
    }

    #[test]
    fn report_flattens_newlines_in_issue_lines() {
        let mut ledger = ValidityLedger::default();
        ledger.push(ValidityIssue::warning(
            "comp\nonent",
            "line one\r\nline two\nthree",
        ));
        let report = validity_report_markdown(&ledger);
        let items: Vec<_> = report.lines().filter(|l| l.starts_with("- ")).collect();
        assert_eq!(items.len(), 1, "{report}");
        assert_eq!(
            items[0], "- warning — comp onent: line one  line two three",
            "{report}"
        );
        assert!(!report.contains('\r'), "{report:?}");
    }

    #[test]
    fn scenario_ledger_free_space_vs_two_ray() {
        let mut scenario = scenario();
        scenario.propagation = PropagationModel::FreeSpace;
        let free = scenario_validity_ledger(&scenario);
        assert_eq!(free.models()[0].name, "free_space");
        assert_eq!(
            maturity(&free, "free_space"),
            ModelMaturity::ValidatedReference
        );
        assert!(!free.issues().iter().any(|i| i.component == "propagation"));

        scenario.propagation = PropagationModel::TwoRay;
        let two_ray = scenario_validity_ledger(&scenario);
        assert_eq!(two_ray.models()[0].name, "two_ray");
        assert_eq!(
            maturity(&two_ray, "two_ray"),
            ModelMaturity::ValidatedWithWarnings
        );
        assert!(two_ray
            .issues()
            .iter()
            .any(|i| i.component == "propagation"));
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
