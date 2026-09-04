use spectra_sim::{enu_to_geo, Origin, Vec3};

#[test]
fn zero_enu_offset_returns_the_origin() {
    let origin = Origin {
        latitude_deg: 48.1173,
        longitude_deg: -1.6778,
        altitude_m: 42.0,
    };
    let geo = enu_to_geo(
        &origin,
        Vec3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
    )
    .unwrap();
    assert!((geo.latitude_deg - origin.latitude_deg).abs() < 1e-10);
    assert!((geo.longitude_deg - origin.longitude_deg).abs() < 1e-10);
    assert!((geo.altitude_m - origin.altitude_m).abs() < 1e-6);
}

#[test]
fn ecef_round_trip_grade_conversion_remains_valid_for_large_local_offsets() {
    let origin = Origin {
        latitude_deg: 60.0,
        longitude_deg: 10.0,
        altitude_m: 100.0,
    };
    let geo = enu_to_geo(
        &origin,
        Vec3 {
            x: 250_000.0,
            y: 300_000.0,
            z: 10_000.0,
        },
    )
    .unwrap();
    assert!((-90.0..=90.0).contains(&geo.latitude_deg));
    assert!((-180.0..=180.0).contains(&geo.longitude_deg));
    assert!(geo.altitude_m.is_finite());
}
