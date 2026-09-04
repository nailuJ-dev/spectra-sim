use spectra_sim::{
    transform_absolute_state, AbsoluteEpoch, AbsoluteOrbitState, EopSample, EopTable,
    ReferenceFrame, TimeScale,
};

#[test]
fn c04_interpolation_is_bounded_and_linear() {
    let text = "2026 09 01 0 60919.0 0.10 0.20 0.050 0.001 -0.002 0 0 0.001\n2026 09 02 0 60920.0 0.12 0.24 0.070 0.003 -0.004 0 0 0.003\n";
    let table = EopTable::from_c04_20u24(text).unwrap();
    let mid = table.sample_at_mjd(60919.5).unwrap();
    assert!((mid.ut1_minus_utc_s - 0.060).abs() < 1e-12);
    assert!((mid.lod_s - 0.002).abs() < 1e-12);
    assert!(table.sample_at_mjd(60918.0).is_err());
}

#[test]
fn gcrf_itrf_round_trip_preserves_state() {
    let epoch = AbsoluteEpoch::from_calendar(2026, 9, 2, 12, 0, 0.0, TimeScale::Utc).unwrap();
    let eop = EopSample {
        mjd_utc: epoch.modified_julian_date(),
        polar_motion_x_rad: 0.1_f64.to_radians() / 3600.0,
        polar_motion_y_rad: 0.2_f64.to_radians() / 3600.0,
        ut1_minus_utc_s: 0.05,
        lod_s: 0.001,
        dx_rad: 0.0,
        dy_rad: 0.0,
    };
    let original = AbsoluteOrbitState::new(
        epoch,
        ReferenceFrame::Gcrf,
        [7_000_000.0, 1_000.0, -2_000.0],
        [100.0, 7_400.0, 20.0],
    )
    .unwrap();
    let itrf = transform_absolute_state(&original, ReferenceFrame::Itrf, &eop).unwrap();
    let back = transform_absolute_state(&itrf, ReferenceFrame::Gcrf, &eop).unwrap();
    for i in 0..3 {
        assert!((back.position_m[i] - original.position_m[i]).abs() < 1e-5);
        assert!((back.velocity_m_per_s[i] - original.velocity_m_per_s[i]).abs() < 1e-8);
    }
}

#[test]
fn teme_itrf_round_trip_preserves_state() {
    let epoch = AbsoluteEpoch::from_calendar(2026, 9, 2, 12, 0, 0.0, TimeScale::Utc).unwrap();
    let eop = EopSample {
        mjd_utc: epoch.modified_julian_date(),
        polar_motion_x_rad: 1e-6,
        polar_motion_y_rad: -2e-6,
        ut1_minus_utc_s: 0.05,
        lod_s: 0.001,
        dx_rad: 0.0,
        dy_rad: 0.0,
    };
    let original = AbsoluteOrbitState::new(
        epoch,
        ReferenceFrame::Teme,
        [7_000_000.0, 1_000.0, -2_000.0],
        [100.0, 7_400.0, 20.0],
    )
    .unwrap();
    let itrf = transform_absolute_state(&original, ReferenceFrame::Itrf, &eop).unwrap();
    let back = transform_absolute_state(&itrf, ReferenceFrame::Teme, &eop).unwrap();
    for i in 0..3 {
        assert!((back.position_m[i] - original.position_m[i]).abs() < 1e-5);
        assert!((back.velocity_m_per_s[i] - original.velocity_m_per_s[i]).abs() < 1e-8);
    }
}

fn finals_line(
    mjd: f64,
    x: f64,
    y: f64,
    dut1: f64,
    lod_ms: f64,
    dx_mas: f64,
    dy_mas: f64,
) -> String {
    let mut bytes = vec![b' '; 130];
    fn put(bytes: &mut [u8], start: usize, end: usize, value: String) {
        let width = end - start;
        let formatted = format!("{:>width$}", value, width = width);
        bytes[start..end].copy_from_slice(formatted.as_bytes());
    }
    put(&mut bytes, 7, 15, format!("{mjd:.2}"));
    put(&mut bytes, 18, 27, format!("{x:.6}"));
    put(&mut bytes, 37, 46, format!("{y:.6}"));
    put(&mut bytes, 58, 68, format!("{dut1:.7}"));
    put(&mut bytes, 79, 86, format!("{lod_ms:.4}"));
    put(&mut bytes, 97, 106, format!("{dx_mas:.4}"));
    put(&mut bytes, 116, 125, format!("{dy_mas:.4}"));
    String::from_utf8(bytes).unwrap()
}

#[test]
fn finals2000a_fixed_width_parser_uses_documented_columns() {
    let text = format!(
        "{}\n{}\n",
        finals_line(60919.0, 0.10, 0.20, 0.050, 1.0, 2.0, -3.0),
        finals_line(60920.0, 0.12, 0.24, 0.070, 3.0, 4.0, -5.0),
    );
    let table = EopTable::from_finals2000a(&text).unwrap();
    let first = table.samples()[0];
    assert!((first.mjd_utc - 60919.0).abs() < 1e-12);
    assert!((first.lod_s - 0.001).abs() < 1e-12);
    assert!((first.dx_rad - 2.0 * spectra_sim::MAS_TO_RAD).abs() < 1e-18);
    assert!((first.dy_rad + 3.0 * spectra_sim::MAS_TO_RAD).abs() < 1e-18);
}

#[test]
fn finals2000a_rejects_missing_precision_fields_instead_of_substituting_zero() {
    let mut line = finals_line(60919.0, 0.10, 0.20, 0.050, 1.0, 2.0, -3.0).into_bytes();
    for byte in &mut line[97..106] {
        *byte = b' ';
    }
    let second = finals_line(60920.0, 0.12, 0.24, 0.070, 3.0, 4.0, -5.0);
    let text = format!("{}\n{}\n", String::from_utf8(line).unwrap(), second);
    assert!(EopTable::from_finals2000a(&text).is_err());
}

#[test]
fn celestial_pole_offsets_materially_change_the_gcrf_itrf_transform() {
    let epoch = AbsoluteEpoch::from_calendar(2026, 9, 2, 12, 0, 0.0, TimeScale::Utc).unwrap();
    let base = EopSample {
        mjd_utc: epoch.modified_julian_date(),
        polar_motion_x_rad: 1e-6,
        polar_motion_y_rad: -2e-6,
        ut1_minus_utc_s: 0.05,
        lod_s: 0.001,
        dx_rad: 0.0,
        dy_rad: 0.0,
    };
    let corrected = EopSample {
        dx_rad: 2.0e-7,
        dy_rad: -3.0e-7,
        ..base
    };
    let state = AbsoluteOrbitState::new(
        epoch,
        ReferenceFrame::Gcrf,
        [7_000_000.0, 1_000.0, -2_000.0],
        [100.0, 7_400.0, 20.0],
    )
    .unwrap();
    let a = transform_absolute_state(&state, ReferenceFrame::Itrf, &base).unwrap();
    let b = transform_absolute_state(&state, ReferenceFrame::Itrf, &corrected).unwrap();
    let separation = ((a.position_m[0] - b.position_m[0]).powi(2)
        + (a.position_m[1] - b.position_m[1]).powi(2)
        + (a.position_m[2] - b.position_m[2]).powi(2))
    .sqrt();
    assert!(separation > 0.1);
}

#[test]
fn utc_leap_second_preserves_si_second_spacing() {
    let before = AbsoluteEpoch::from_calendar(2016, 12, 31, 23, 59, 59.0, TimeScale::Utc).unwrap();
    let leap = AbsoluteEpoch::from_calendar(2016, 12, 31, 23, 59, 60.0, TimeScale::Utc).unwrap();
    let after = AbsoluteEpoch::from_calendar(2017, 1, 1, 0, 0, 0.0, TimeScale::Utc).unwrap();

    assert!((leap.seconds_since(&before, None).unwrap() - 1.0).abs() < 1e-6);
    assert!((after.seconds_since(&leap, None).unwrap() - 1.0).abs() < 1e-6);
    assert!((after.seconds_since(&before, None).unwrap() - 2.0).abs() < 1e-6);
}

#[test]
fn finals2000a_skips_incomplete_prediction_tail_rows() {
    let first = finals_line(60919.0, 0.10, 0.20, 0.050, 1.0, 2.0, -3.0);
    let second = finals_line(60920.0, 0.12, 0.24, 0.070, 3.0, 4.0, -5.0);
    let mut incomplete = finals_line(60921.0, 0.14, 0.26, 0.080, 4.0, 5.0, -6.0).into_bytes();
    for byte in &mut incomplete[79..86] {
        *byte = b' ';
    }
    let text = format!(
        "{first}\n{second}\n{}\n",
        String::from_utf8(incomplete).unwrap()
    );
    let table = EopTable::from_finals2000a(&text).unwrap();
    assert_eq!(table.samples().len(), 2);
}
