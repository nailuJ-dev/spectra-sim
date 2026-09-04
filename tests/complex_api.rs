use spectra_sim::{Complex64, MaterialComplex64};

#[test]
fn material_and_rf_complex_types_are_the_same_public_type() {
    let material_value: MaterialComplex64 = Complex64::new(1.0, -0.5);
    let rf_value: Complex64 = material_value;
    assert_eq!(rf_value, Complex64::new(1.0, -0.5));
}
