//! Regression tests for defects and integration gaps found during the v0.4.0 audit.

use spectra_sim::{
    effective_directional_gain_dbi, fresnel_coefficients, run_cuas, run_sigint, simulate_iq,
    Antenna, AntennaPatternModel, Complex64, KinematicState, MaterialProperties, OcmMessage,
    OemMessage, Polarization, Scenario, ValidityLedger, Vec3,
};

fn medium(relative_permittivity_re: f64) -> MaterialProperties {
    MaterialProperties {
        relative_permittivity: Complex64::new(relative_permittivity_re, 0.0),
        relative_permeability: 1.0,
        conductivity_s_per_m: 0.0,
        validity: ValidityLedger::default(),
    }
}

#[test]
fn fresnel_te_and_tm_are_not_swapped_at_oblique_incidence() {
    let a = medium(1.0);
    let b = medium(2.25);
    let theta = std::f64::consts::FRAC_PI_4;
    let te = fresnel_coefficients(1.0e9, &a, &b, theta, Polarization::Te).unwrap();
    let tm = fresnel_coefficients(1.0e9, &a, &b, theta, Polarization::Tm).unwrap();
    assert!((te.reflection.re + 0.303_337).abs() < 1e-5, "r_TE={}", te.reflection.re);
    assert!((tm.reflection.re + 0.092_013).abs() < 1e-5, "r_TM={}", tm.reflection.re);
    assert!(te.reflection.abs() > tm.reflection.abs());
}

#[test]
fn fresnel_tm_transmission_uses_the_angle_ratio() {
    let a = medium(1.0);
    let b = medium(2.25);
    let tm = fresnel_coefficients(
        1.0e9,
        &a,
        &b,
        std::f64::consts::FRAC_PI_4,
        Polarization::Tm,
    )
    .unwrap();
    assert!((tm.transmission.re - 0.728_009).abs() < 1e-5, "tau_TM={}", tm.transmission.re);
}

#[test]
fn fresnel_conserves_energy_across_the_incidence_range() {
    let a = medium(1.0);
    let b = medium(2.25);
    let n1 = 1.0_f64;
    let n2 = 1.5_f64;
    for degrees in [0.0_f64, 10.0, 20.0, 30.0, 45.0, 60.0, 75.0, 85.0] {
        let theta = degrees.to_radians();
        let cos_i = theta.cos();
        let sin_t = n1 / n2 * theta.sin();
        let cos_t = (1.0 - sin_t * sin_t).sqrt();
        let te_ratio = (n2 * cos_t) / (n1 * cos_i);
        let te = fresnel_coefficients(1.0e9, &a, &b, theta, Polarization::Te).unwrap();
        let tm = fresnel_coefficients(1.0e9, &a, &b, theta, Polarization::Tm).unwrap();
        assert!((te.reflection.abs().powi(2) + te_ratio * te.transmission.abs().powi(2) - 1.0).abs() < 1e-9);
        assert!((tm.reflection.abs().powi(2) + te_ratio * tm.transmission.abs().powi(2) - 1.0).abs() < 1e-9);
    }
}

#[test]
fn fresnel_tm_reflection_vanishes_at_the_brewster_angle() {
    let a = medium(1.0);
    let b = medium(2.25);
    let brewster = 1.5_f64.atan();
    let tm = fresnel_coefficients(1.0e9, &a, &b, brewster, Polarization::Tm).unwrap();
    assert!(tm.reflection.abs() < 1e-9, "|r_TM|={}", tm.reflection.abs());
}

#[test]
fn a_gain_medium_is_rejected() {
    let mut bad = medium(4.0);
    bad.relative_permittivity = Complex64::new(4.0, 0.5);
    assert!(bad.validate().is_err());
}

#[test]
fn exported_snr_matches_generated_rms_for_a_pulsed_waveform() {
    let scenario: Scenario = serde_json::from_str(include_str!("../examples/scenarios/sigint_urban.json")).unwrap();
    let run = run_sigint(&scenario).unwrap();
    let samples = &run.simulation.capture.samples;
    let total: f64 = samples
        .iter()
        .map(|s| f64::from(s.i).mul_add(f64::from(s.i), f64::from(s.q) * f64::from(s.q)))
        .sum();
    let measured_rms = (total / samples.len() as f64).sqrt();
    let receiver = scenario.receiver("sensor-1").unwrap();
    let error_db = 20.0 * (measured_rms / receiver.agc_target_rms).log10();
    assert!(error_db.abs() < 1.0, "measured_rms={measured_rms}, error={error_db} dB");
    assert!(run.simulation.truth.clipped_sample_fraction < 0.01);
    assert_eq!(run.simulation.capture.full_scale_v, receiver.full_scale_v);
    assert_eq!(run.simulation.capture.adc_bits, receiver.adc_bits);
}

#[test]
fn iq_phase_imbalance_creates_cross_coupling() {
    let scenario: Scenario = serde_json::from_str(include_str!("../examples/scenarios/sigint_urban.json")).unwrap();
    let emitter = scenario.emitter("emitter-alpha").unwrap();
    let mut clean = scenario.receiver("sensor-1").unwrap().clone();
    clean.iq_gain_imbalance_db = 0.0;
    clean.iq_phase_imbalance_deg = 0.0;
    clean.phase_noise_rad_std = 0.0;
    let mut skewed = clean.clone();
    skewed.iq_phase_imbalance_deg = 20.0;
    let a = simulate_iq(emitter, &clean, scenario.environment, scenario.propagation, 0, 512, 7).unwrap();
    let b = simulate_iq(emitter, &skewed, scenario.environment, scenario.propagation, 0, 512, 7).unwrap();
    assert!(a.capture.samples.iter().zip(&b.capture.samples).any(|(x, y)| (x.i - y.i).abs() > 1e-6));
}

#[test]
fn oem_kvn_accepts_optional_acceleration_columns() {
    let text = "CCSDS_OEM_VERS = 3.0\nMETA_START\nOBJECT_NAME = SAT-A\nREF_FRAME = GCRF\nTIME_SYSTEM = UTC\nMETA_STOP\n2026-09-02T00:00:00 7000 0 0 0 7.5 0 0.001 0.002 0.003\n2026-09-02T00:01:00 6990 450 0 -0.48 7.49 0 0.001 0.002 0.003\n";
    let oem = OemMessage::from_kvn(text).unwrap();
    assert_eq!(oem.segments[0].states.len(), 2);
    assert_eq!(oem.segments[0].states[0].position_m[0], 7_000_000.0);
    assert_eq!(
        oem.segments[0].states[0].acceleration_m_per_s2,
        Some([1.0, 2.0, 3.0])
    );
}

#[test]
fn oem_kvn_rejects_a_malformed_ephemeris_line() {
    let text = "CCSDS_OEM_VERS = 3.0\nMETA_START\nREF_FRAME = GCRF\nTIME_SYSTEM = UTC\nMETA_STOP\n2026-09-02T00:00:00 7000 0 0 0 7.5\n2026-09-02T00:01:00 6990 450 0 -0.48 7.49 0\n";
    assert!(OemMessage::from_kvn(text).is_err());
}

#[test]
fn ocm_kvn_honours_a_bracketed_traj_units_list() {
    let text = "CCSDS_OCM_VERS = 3.0\nMETA_START\nOBJECT_NAME = SAT-A\nTIME_SYSTEM = UTC\nMETA_STOP\nTRAJ_START\nTRAJ_ID = primary\nTRAJ_REF_FRAME = GCRF\nTRAJ_TYPE = CARTPV\nTRAJ_UNITS = [m,m,m,m/s,m/s,m/s]\n2026-09-02T00:00:00 7000000 0 0 0 7500 0\n2026-09-02T00:01:00 6990000 450000 0 -480 7490 0\nTRAJ_STOP\n";
    let ocm = OcmMessage::from_kvn(text).unwrap();
    assert_eq!(ocm.trajectory_states[0].state.position_m[0], 7_000_000.0);
}

#[test]
fn duplicate_entity_ids_are_rejected() {
    let mut scenario: Scenario = serde_json::from_str(include_str!("../examples/scenarios/sigint_urban.json")).unwrap();
    let duplicate = scenario.receivers[0].clone();
    scenario.receivers.push(duplicate);
    assert!(scenario.validate().is_err());
}

#[test]
fn directional_antenna_uses_platform_attitude() {
    let antenna = Antenna {
        gain_dbi: 20.0,
        polarization_loss_db: 0.0,
        cable_loss_db: 0.0,
        pattern: AntennaPatternModel::EllipticalGaussian {
            horizontal_half_power_beamwidth_deg: 20.0,
            vertical_half_power_beamwidth_deg: 10.0,
            floor_gain_dbi: -30.0,
        },
    };
    let state_north = KinematicState {
        position_enu_m: Vec3 { x: 0.0, y: 0.0, z: 0.0 },
        velocity_enu_mps: Vec3 { x: 0.0, y: 0.0, z: 0.0 },
        roll_deg: 0.0,
        pitch_deg: 0.0,
        yaw_deg: 0.0,
    };
    let mut state_east = state_north;
    state_east.yaw_deg = 90.0;
    let target_north = Vec3 { x: 0.0, y: 1_000.0, z: 0.0 };
    let aligned = effective_directional_gain_dbi(&antenna, state_north, target_north, 10.0e9).unwrap();
    let misaligned = effective_directional_gain_dbi(&antenna, state_east, target_north, 10.0e9).unwrap();
    assert!(aligned > misaligned + 20.0);
}

#[test]
fn cuas_path_consumes_scenario_propagation() {
    let base: Scenario = serde_json::from_str(include_str!("../examples/scenarios/cuas_drone_isac.json")).unwrap();
    let mut free = base.clone();
    free.propagation = spectra_sim::PropagationModel::FreeSpace;
    let two_ray = base;
    let a = run_cuas(&free).unwrap();
    let b = run_cuas(&two_ray).unwrap();
    assert!((a.radar_measurement.received_power_dbm - b.radar_measurement.received_power_dbm).abs() > 1e-6);
}


#[test]
fn legacy_scenarios_without_pattern_fields_remain_valid() {
    let scenario: Scenario =
        serde_json::from_str(include_str!("../examples/scenarios/cuas_drone_isac.json")).unwrap();
    assert!(scenario.validate().is_ok());
    assert_eq!(
        scenario.receivers[0].antenna.pattern,
        AntennaPatternModel::Isotropic
    );
}
