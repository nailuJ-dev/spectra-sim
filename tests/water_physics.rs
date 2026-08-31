use spectra_sim::{propagation_constant, water_properties, WaterPermittivityModel, WaterState};

#[test]
fn meissner_wentz_reproduces_pure_water_reference_point() {
    let state = WaterState::new(25.0, 0.0).unwrap();
    let props = water_properties(
        1.7e9,
        &state,
        WaterPermittivityModel::MeissnerWentzDoubleDebye,
    )
    .unwrap();
    assert!((props.relative_permittivity.re - 77.83).abs() < 0.6);
    assert!((props.relative_permittivity.im.abs() - 6.42).abs() < 0.6);
}

#[test]
fn seawater_is_more_attenuating_than_fresh_water_at_vlf() {
    let fresh = water_properties(
        30_000.0,
        &WaterState::new(15.0, 0.0).unwrap(),
        WaterPermittivityModel::ConductiveLowFrequency,
    )
    .unwrap();
    let sea = water_properties(
        30_000.0,
        &WaterState::new(15.0, 35.0).unwrap(),
        WaterPermittivityModel::ConductiveLowFrequency,
    )
    .unwrap();
    let gamma_fresh = propagation_constant(30_000.0, &fresh).unwrap();
    let gamma_sea = propagation_constant(30_000.0, &sea).unwrap();
    assert!(gamma_sea.alpha_np_per_m > gamma_fresh.alpha_np_per_m);
    assert!(gamma_sea.skin_depth_m < gamma_fresh.skin_depth_m);
}

#[test]
fn seawater_model_marks_extrapolated_inputs() {
    let state = WaterState::new(40.0, 35.0).unwrap();
    let props = water_properties(
        100.0e9,
        &state,
        WaterPermittivityModel::MeissnerWentzDoubleDebye,
    )
    .unwrap();
    assert!(!props.validity.issues().is_empty());
}
