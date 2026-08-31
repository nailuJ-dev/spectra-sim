use spectra_sim::{
    fresnel_coefficients, propagate_stack, water_properties, MaterialProperties, MediumLayer,
    MediumStack, Polarization, ValidityLedger, WaterPermittivityModel, WaterState,
};

use spectra_sim::materials::Complex64;

fn air() -> MaterialProperties {
    MaterialProperties {
        relative_permittivity: Complex64::new(1.0006, 0.0),
        relative_permeability: 1.0,
        conductivity_s_per_m: 0.0,
        validity: ValidityLedger::default(),
    }
}

#[test]
fn identical_media_have_negligible_reflection() {
    let f = fresnel_coefficients(1.0e9, &air(), &air(), 0.0, Polarization::Te).unwrap();
    assert!(f.reflection.abs() < 1e-9);
    assert!((f.transmission.abs() - 1.0).abs() < 1e-9);
}

#[test]
fn a_seawater_layer_reduces_complex_amplitude() {
    let sea = water_properties(
        100_000.0,
        &WaterState::new(15.0, 35.0).unwrap(),
        WaterPermittivityModel::ConductiveLowFrequency,
    )
    .unwrap();
    let stack = MediumStack::new(vec![
        MediumLayer::new(Some(0.0), air()).unwrap(),
        MediumLayer::new(Some(5.0), sea).unwrap(),
    ])
    .unwrap();
    let out = propagate_stack(&stack, 100_000.0, 0.0, Polarization::Te).unwrap();
    assert!(out.complex_gain.abs() < 1.0);
    assert!(out.attenuation_db > 0.0);
}
