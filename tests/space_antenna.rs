use spectra_sim::{
    ground_station_az_el_range, AntennaPattern, GaussianAntennaPattern, GeodeticPosition,
    GroundStation, ParabolicDish,
};

#[test]
fn dish_gain_increases_with_frequency() {
    let dish = ParabolicDish {
        diameter_m: 1.2,
        aperture_efficiency: 0.65,
    };
    assert!(dish.boresight_gain_dbi(12e9).unwrap() > dish.boresight_gain_dbi(6e9).unwrap());
}

#[test]
fn gaussian_pattern_loses_gain_off_axis() {
    let p = GaussianAntennaPattern {
        boresight_gain_dbi: 40.0,
        half_power_beamwidth_rad: 0.02,
        floor_gain_dbi: -10.0,
    };
    assert!(p.gain_dbi(0.02, 12e9).unwrap() < p.gain_dbi(0.0, 12e9).unwrap());
}

#[test]
fn station_geometry_marks_below_horizon_target_invisible() {
    let station = GroundStation::new(
        "eq",
        GeodeticPosition {
            latitude_rad: 0.0,
            longitude_rad: 0.0,
            height_m: 0.0,
        },
        0.0,
    )
    .unwrap();
    let below = [-7_000_000.0, 0.0, 0.0];
    let geometry = ground_station_az_el_range(&station, below).unwrap();
    assert!(!geometry.visible);
    assert!(geometry.elevation_rad < 0.0);
}
