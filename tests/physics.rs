use spectra_sim::{free_space_path_loss_db, thermal_noise_dbm, one_way_doppler_hz, SPEED_OF_LIGHT_MPS};

#[test]
fn fspl_at_one_meter_matches_closed_form() {
    let loss = free_space_path_loss_db(1.0e9, 1.0).unwrap_or(f64::NAN);
    assert!((loss - 32.4478).abs() < 0.02, "loss={loss}");
}

#[test]
fn thermal_noise_at_290k_one_mhz_is_about_minus_114_dbm() {
    let noise = thermal_noise_dbm(1.0e6, 290.0, 0.0).unwrap_or(f64::NAN);
    assert!((noise + 113.98).abs() < 0.2, "noise={noise}");
}

#[test]
fn one_way_doppler_obeys_velocity_over_wavelength() {
    let velocity = 30.0;
    let f = 3.0e9;
    let expected = -velocity / (SPEED_OF_LIGHT_MPS / f);
    let actual = one_way_doppler_hz(f, velocity).unwrap_or(f64::NAN);
    assert!((actual - expected).abs() < 1e-9);
}
