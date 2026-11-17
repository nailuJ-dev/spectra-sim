use spectra_sim::{
    doppler_rate_hz_per_s, solve_absolute_one_way_link, AbsoluteEphemeris, AbsoluteEpoch,
    AbsoluteOrbitState, ReferenceFrame, TimeScale, SPEED_OF_LIGHT_M_PER_S,
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

const FLYBY_SPEED_M_PER_S: f64 = 7_500.0;
const FLYBY_MISS_DISTANCE_M: f64 = 600_000.0;
const FLYBY_CARRIER_HZ: f64 = 2.2e9;

fn flyby_epoch() -> AbsoluteEpoch {
    AbsoluteEpoch::from_calendar(2026, 9, 2, 0, 0, 0.0, TimeScale::Tai).unwrap()
}

/// Fixed transmitter at the origin; receiver on a straight line at constant
/// speed with closest approach (distance `FLYBY_MISS_DISTANCE_M`) at the
/// returned epoch. Both ephemerides are tabulated in the same frame.
fn flyby_pair() -> (AbsoluteEphemeris, AbsoluteEphemeris, AbsoluteEpoch) {
    let base = flyby_epoch();
    let make = |second: f64, position: [f64; 3], velocity: [f64; 3]| {
        AbsoluteOrbitState::new(
            base.shift_si_seconds(second, None).unwrap(),
            ReferenceFrame::Gcrf,
            position,
            velocity,
        )
        .unwrap()
    };
    let times: Vec<f64> = (-40..=40).map(|k| f64::from(k) * 10.0).collect();
    let tx = AbsoluteEphemeris::new(times.iter().map(|&t| make(t, [0.0; 3], [0.0; 3])).collect())
        .unwrap();
    let rx = AbsoluteEphemeris::new(
        times
            .iter()
            .map(|&t| {
                make(
                    t,
                    [FLYBY_MISS_DISTANCE_M, FLYBY_SPEED_M_PER_S * t, 0.0],
                    [0.0, FLYBY_SPEED_M_PER_S, 0.0],
                )
            })
            .collect(),
    )
    .unwrap();
    (tx, rx, base)
}

fn flyby_rate(
    tx: &AbsoluteEphemeris,
    rx: &AbsoluteEphemeris,
    epoch: AbsoluteEpoch,
    step_s: f64,
) -> f64 {
    doppler_rate_hz_per_s(tx, rx, epoch, FLYBY_CARRIER_HZ, 12, 1e-12, None, step_s).unwrap()
}

#[test]
fn doppler_rate_matches_closed_form_at_closest_approach() {
    let (tx, rx, closest) = flyby_pair();
    let expected = -(FLYBY_CARRIER_HZ / SPEED_OF_LIGHT_M_PER_S) * FLYBY_SPEED_M_PER_S.powi(2)
        / FLYBY_MISS_DISTANCE_M;
    let actual = flyby_rate(&tx, &rx, closest, 0.5);
    let relative_error = ((actual - expected) / expected).abs();
    assert!(
        relative_error < 1e-3,
        "expected {expected}, got {actual}, relative error {relative_error}"
    );
}

#[test]
fn doppler_rate_sign_and_magnitude_before_and_after_closest_approach() {
    let (tx, rx, closest) = flyby_pair();
    let before = flyby_rate(
        &tx,
        &rx,
        closest.shift_si_seconds(-100.0, None).unwrap(),
        0.5,
    );
    let at = flyby_rate(&tx, &rx, closest, 0.5);
    let after = flyby_rate(
        &tx,
        &rx,
        closest.shift_si_seconds(100.0, None).unwrap(),
        0.5,
    );
    assert!(before < 0.0, "before = {before}");
    assert!(at < 0.0, "at = {at}");
    assert!(after < 0.0, "after = {after}");
    assert!(at.abs() > before.abs(), "at = {at}, before = {before}");
    assert!(at.abs() > after.abs(), "at = {at}, after = {after}");
    // The geometry is symmetric about closest approach.
    assert!(((before - after) / at).abs() < 1e-6);
}

#[test]
fn doppler_rate_rejects_bad_step() {
    let (tx, rx, closest) = flyby_pair();
    for step in [0.0, f64::NAN, -1.0, f64::INFINITY] {
        let result =
            doppler_rate_hz_per_s(&tx, &rx, closest, FLYBY_CARRIER_HZ, 12, 1e-12, None, step);
        assert!(result.is_err(), "step {step} should be rejected");
    }
}
