use spectra_sim::{
    solve_one_way_link, Ephemeris, Epoch, OrbitState, ReferenceFrame, SimError, TimeScale,
};

fn state_in_frame(t: f64, x: f64, vx: f64, frame: ReferenceFrame) -> OrbitState {
    OrbitState::new(
        Epoch::relative_seconds(t, TimeScale::Tai).unwrap(),
        frame,
        [x, 0.0, 0.0],
        [vx, 0.0, 0.0],
    )
    .unwrap()
}

fn state(t: f64, x: f64, vx: f64) -> OrbitState {
    state_in_frame(t, x, vx, ReferenceFrame::Gcrf)
}

#[test]
fn hermite_ephemeris_preserves_constant_velocity_motion() {
    let eph = Ephemeris::new(vec![
        state(0.0, 7_000_000.0, 1_000.0),
        state(10.0, 7_010_000.0, 1_000.0),
    ])
    .unwrap();
    let mid = eph
        .state_at(Epoch::relative_seconds(5.0, TimeScale::Tai).unwrap())
        .unwrap();
    assert!((mid.position_m[0] - 7_005_000.0).abs() < 1e-6);
    assert!((mid.velocity_m_per_s[0] - 1_000.0).abs() < 1e-6);
}

#[test]
fn one_way_link_iterates_light_time_and_computes_doppler() {
    let tx = Ephemeris::new(vec![
        state(-1.0, 1_000_000.0, 100.0),
        state(2.0, 1_000_300.0, 100.0),
    ])
    .unwrap();
    let rx = Ephemeris::new(vec![state(-1.0, 0.0, 0.0), state(2.0, 0.0, 0.0)]).unwrap();
    let link = solve_one_way_link(
        &tx,
        &rx,
        Epoch::relative_seconds(1.0, TimeScale::Tai).unwrap(),
        1.5e9,
        12,
        1e-12,
    )
    .unwrap();
    assert!(link.light_time_s > 0.003 && link.light_time_s < 0.004);
    assert!(link.slant_range_m > 1_000_000.0);
    assert!(link.doppler_hz < 0.0);
}

#[test]
fn one_way_link_rejects_mixed_reference_frames_before_vector_subtraction() {
    let tx = Ephemeris::new(vec![
        state_in_frame(-1.0, 1_000_000.0, 0.0, ReferenceFrame::Gcrf),
        state_in_frame(2.0, 1_000_000.0, 0.0, ReferenceFrame::Gcrf),
    ])
    .unwrap();
    let rx = Ephemeris::new(vec![
        state_in_frame(-1.0, 0.0, 0.0, ReferenceFrame::Itrf),
        state_in_frame(2.0, 0.0, 0.0, ReferenceFrame::Itrf),
    ])
    .unwrap();
    let error = solve_one_way_link(
        &tx,
        &rx,
        Epoch::relative_seconds(1.0, TimeScale::Tai).unwrap(),
        1.5e9,
        12,
        1e-12,
    )
    .expect_err("mixed reference frames must fail");
    match error {
        SimError::InvalidArgument(message) => {
            assert!(
                message.contains("reference frame mismatch"),
                "message={message}"
            );
        }
        other => panic!("unexpected error: {other}"),
    }
}
