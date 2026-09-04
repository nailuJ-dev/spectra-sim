use spectra_sim::{
    solve_absolute_one_way_link, AbsoluteEphemeris, AbsoluteEpoch, AbsoluteOrbitState,
    ReferenceFrame, TimeScale,
};

fn state(second: f64, x: f64, vx: f64) -> AbsoluteOrbitState {
    let base = AbsoluteEpoch::from_calendar(2026, 9, 2, 0, 0, 0.0, TimeScale::Tai).unwrap();
    AbsoluteOrbitState::new(
        base.shift_si_seconds(second, None).unwrap(),
        ReferenceFrame::Gcrf,
        [x, 0.0, 0.0],
        [vx, 0.0, 0.0],
    )
    .unwrap()
}

#[test]
fn absolute_hermite_preserves_constant_velocity() {
    let eph = AbsoluteEphemeris::new(vec![
        state(0.0, 7_000_000.0, 1_000.0),
        state(10.0, 7_010_000.0, 1_000.0),
    ])
    .unwrap();
    let epoch = eph.states()[0].epoch.shift_si_seconds(5.0, None).unwrap();
    let mid = eph.state_at(epoch, None).unwrap();
    assert!((mid.position_m[0] - 7_005_000.0).abs() < 1e-6);
    assert!((mid.velocity_m_per_s[0] - 1_000.0).abs() < 1e-6);
}

#[test]
fn absolute_link_keeps_signed_doppler() {
    let tx = AbsoluteEphemeris::new(vec![
        state(-1.0, 999_900.0, 100.0),
        state(2.0, 1_000_200.0, 100.0),
    ])
    .unwrap();
    let rx = AbsoluteEphemeris::new(vec![state(-1.0, 0.0, 0.0), state(2.0, 0.0, 0.0)]).unwrap();
    let receive_epoch = tx.states()[0].epoch.shift_si_seconds(2.0, None).unwrap();
    let link =
        solve_absolute_one_way_link(&tx, &rx, receive_epoch, 1.5e9, 12, 1e-12, None).unwrap();
    assert!(link.range_rate_m_per_s > 0.0);
    assert!(link.doppler_hz < 0.0);
}

#[test]
fn absolute_quintic_hermite_preserves_constant_acceleration() {
    let base = AbsoluteEpoch::from_calendar(2026, 9, 2, 0, 0, 0.0, TimeScale::Tai).unwrap();
    let make = |second: f64| {
        let x0 = 7_000_000.0;
        let v0 = 1_000.0;
        let acceleration = 2.0;
        let position = x0 + v0 * second + 0.5 * acceleration * second * second;
        let velocity = v0 + acceleration * second;
        AbsoluteOrbitState::new_with_acceleration(
            base.shift_si_seconds(second, None).unwrap(),
            ReferenceFrame::Gcrf,
            [position, 0.0, 0.0],
            [velocity, 0.0, 0.0],
            Some([acceleration, 0.0, 0.0]),
        )
        .unwrap()
    };

    let eph = AbsoluteEphemeris::new(vec![make(0.0), make(10.0)]).unwrap();
    let epoch = base.shift_si_seconds(5.0, None).unwrap();
    let mid = eph.state_at(epoch, None).unwrap();
    assert!((mid.position_m[0] - 7_005_025.0).abs() < 1e-6);
    assert!((mid.velocity_m_per_s[0] - 1_010.0).abs() < 1e-6);
    assert_eq!(mid.acceleration_m_per_s2, Some([2.0, 0.0, 0.0]));
}
