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

mod verify {
    use super::load;
    use spectra_sim::{run_cuas, run_sigint, verify_replay, ReplayVerdict, RunKind};

    fn flip_first_hex(hash: &str) -> String {
        let mut chars: Vec<char> = hash.chars().collect();
        chars[0] = if chars[0] == '0' { '1' } else { '0' };
        chars.into_iter().collect()
    }

    #[test]
    fn verify_matches_fresh_manifest() {
        let scenario = load("examples/scenarios/sigint_urban.json");
        let run = run_sigint(&scenario).unwrap();
        assert_eq!(
            verify_replay(&scenario, &run.manifest, RunKind::Sigint).unwrap(),
            ReplayVerdict::Match
        );
    }

    #[test]
    fn verify_matches_fresh_cuas_manifest() {
        let scenario = load("examples/scenarios/cuas_drone_isac.json");
        let run = run_cuas(&scenario).unwrap();
        assert_eq!(
            verify_replay(&scenario, &run.manifest, RunKind::Cuas).unwrap(),
            ReplayVerdict::Match
        );
    }

    #[test]
    fn verify_matches_legacy_manifest_without_output_schema_version() {
        let scenario = load("examples/scenarios/sigint_urban.json");
        let run = run_sigint(&scenario).unwrap();
        let mut value = serde_json::to_value(&run.manifest).unwrap();
        value
            .as_object_mut()
            .unwrap()
            .remove("output_schema_version");
        let legacy: spectra_sim::ReplayManifest = serde_json::from_value(value).unwrap();
        assert_eq!(legacy.output_schema_version, "unversioned");
        assert_eq!(
            verify_replay(&scenario, &legacy, RunKind::Sigint).unwrap(),
            ReplayVerdict::Match
        );
    }

    #[test]
    fn verify_detects_scenario_change() {
        let mut scenario = load("examples/scenarios/sigint_urban.json");
        let manifest = run_sigint(&scenario).unwrap().manifest;
        scenario.seed += 1;
        let verdict = verify_replay(&scenario, &manifest, RunKind::Sigint).unwrap();
        assert!(
            matches!(verdict, ReplayVerdict::ScenarioMismatch { ref expected, .. } if *expected == manifest.scenario_sha256),
            "{verdict:?}"
        );
    }

    #[test]
    fn verify_detects_tampered_output_hash() {
        let scenario = load("examples/scenarios/sigint_urban.json");
        let mut manifest = run_sigint(&scenario).unwrap().manifest;
        let real = manifest.output_sha256.clone();
        manifest.output_sha256 = flip_first_hex(&real);
        let verdict = verify_replay(&scenario, &manifest, RunKind::Sigint).unwrap();
        assert_eq!(
            verdict,
            ReplayVerdict::OutputMismatch {
                expected: manifest.output_sha256.clone(),
                actual: real
            }
        );
    }

    #[test]
    fn verify_reports_version_mismatch() {
        let scenario = load("examples/scenarios/sigint_urban.json");
        let mut manifest = run_sigint(&scenario).unwrap().manifest;
        manifest.simulator_version = "0.0.0".into();
        manifest.output_sha256 = flip_first_hex(&manifest.output_sha256);
        let verdict = verify_replay(&scenario, &manifest, RunKind::Sigint).unwrap();
        assert_eq!(
            verdict,
            ReplayVerdict::VersionMismatch {
                manifest: "0.0.0".into(),
                current: env!("CARGO_PKG_VERSION").into()
            }
        );
    }

    #[test]
    fn verify_reports_schema_mismatch_for_tampered_legacy_manifest() {
        let scenario = load("examples/scenarios/sigint_urban.json");
        let run = run_sigint(&scenario).unwrap();
        let mut value = serde_json::to_value(&run.manifest).unwrap();
        value
            .as_object_mut()
            .unwrap()
            .remove("output_schema_version");
        let mut legacy: spectra_sim::ReplayManifest = serde_json::from_value(value).unwrap();
        legacy.output_sha256 = flip_first_hex(&legacy.output_sha256);
        let verdict = verify_replay(&scenario, &legacy, RunKind::Sigint).unwrap();
        assert_eq!(
            verdict,
            ReplayVerdict::SchemaMismatch {
                manifest: "unversioned".into(),
                current: spectra_sim::OUTPUT_SCHEMA_VERSION.into()
            }
        );
    }

    #[test]
    fn verify_with_wrong_job_kind_is_an_error() {
        let scenario = load("examples/scenarios/sigint_urban.json");
        let manifest = run_sigint(&scenario).unwrap().manifest;
        assert!(verify_replay(&scenario, &manifest, RunKind::Cuas).is_err());
    }
}
