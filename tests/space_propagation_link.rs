use spectra_sim::{
    compute_link_budget, earth_space_corrections, gaseous_specific_attenuation_p676_13,
    ionospheric_first_order, AtmosphericLayer, EarthSpaceEnvironment, LinkBudgetInput,
};

#[test]
fn ionosphere_obeys_inverse_frequency_squared() {
    let a = ionospheric_first_order(1.0e9, 10.0, None).unwrap();
    let b = ionospheric_first_order(2.0e9, 10.0, None).unwrap();
    assert!((a.group_delay_m / b.group_delay_m - 4.0).abs() < 1e-12);
}

#[test]
fn p676_line_by_line_is_finite_and_positive() {
    let attenuation = gaseous_specific_attenuation_p676_13(60.0, 1013.25, 7.5, 288.15).unwrap();
    assert!(attenuation.dry_air_db_per_km.is_finite());
    assert!(attenuation.water_vapour_db_per_km.is_finite());
    assert!(attenuation.total_db_per_km() > 0.0);
}

#[test]
fn earth_space_and_link_budget_compose_without_hidden_defaults() {
    let environment = EarthSpaceEnvironment {
        atmospheric_layers: vec![AtmosphericLayer {
            path_length_km: 5.0,
            pressure_hpa: 850.0,
            water_vapour_density_g_m3: 5.0,
            temperature_k: 280.0,
        }],
        rain_attenuation_db: Some(2.0),
        cloud_attenuation_db: Some(0.5),
        tropospheric_scintillation_db: Some(0.2),
        slant_tec_tecu: Some(20.0),
        faraday_rotation_rad: None,
    };
    let corrections = earth_space_corrections(12.0e9, &environment).unwrap();
    let result = compute_link_budget(LinkBudgetInput {
        frequency_hz: 12.0e9,
        range_m: 36_000_000.0,
        tx_power_dbw: 10.0,
        tx_feeder_loss_db: 1.0,
        tx_gain_dbi: 42.0,
        rx_gain_dbi: 45.0,
        rx_feeder_loss_db: 1.0,
        propagation_loss_db: corrections.total_attenuation_db(),
        polarization_loss_db: 0.5,
        pointing_loss_db: 0.5,
        other_loss_db: 1.0,
        system_noise_temperature_k: 300.0,
        bandwidth_hz: 1.0e6,
        bit_rate_bps: Some(500_000.0),
        required_eb_n0_db: Some(5.0),
    })
    .unwrap();
    assert!(result.c_n0_db_hz.is_finite());
    assert!(result.eb_n0_db.unwrap().is_finite());
}

#[test]
fn p840_9_cloud_coefficient_and_slant_attenuation_are_positive() {
    use spectra_sim::{
        cloud_attenuation_p840_9_db, cloud_liquid_mass_absorption_coefficient_p840_9,
    };
    let coefficient = cloud_liquid_mass_absorption_coefficient_p840_9(40.0).unwrap();
    assert!(coefficient.is_finite());
    assert!(coefficient > 0.0);
    let attenuation = cloud_attenuation_p840_9_db(40.0, 0.5, 30_f64.to_radians()).unwrap();
    assert!(attenuation > 0.0);
    assert!((attenuation - coefficient).abs() < 1e-12);
}

#[test]
fn one_way_link_doppler_sign_distinguishes_receding_and_approaching_motion() {
    use spectra_sim::{
        solve_one_way_link, Ephemeris, Epoch, OrbitState, ReferenceFrame, TimeScale,
    };

    fn state(t: f64, x: f64, vx: f64) -> OrbitState {
        OrbitState::new(
            Epoch::relative_seconds(t, TimeScale::Tai).unwrap(),
            ReferenceFrame::Gcrf,
            [x, 0.0, 0.0],
            [vx, 0.0, 0.0],
        )
        .unwrap()
    }

    let rx = Ephemeris::new(vec![state(-1.0, 0.0, 0.0), state(2.0, 0.0, 0.0)]).unwrap();
    let receding = Ephemeris::new(vec![
        state(-1.0, 999_900.0, 100.0),
        state(2.0, 1_000_200.0, 100.0),
    ])
    .unwrap();
    let approaching = Ephemeris::new(vec![
        state(-1.0, 1_000_100.0, -100.0),
        state(2.0, 999_800.0, -100.0),
    ])
    .unwrap();
    let epoch = Epoch::relative_seconds(1.0, TimeScale::Tai).unwrap();
    let away = solve_one_way_link(&receding, &rx, epoch, 1.5e9, 12, 1e-12).unwrap();
    let toward = solve_one_way_link(&approaching, &rx, epoch, 1.5e9, 12, 1e-12).unwrap();
    assert!(away.range_rate_m_per_s > 0.0);
    assert!(away.doppler_hz < 0.0);
    assert!(toward.range_rate_m_per_s < 0.0);
    assert!(toward.doppler_hz > 0.0);
}


#[test]
fn p676_rejects_frequencies_outside_the_recommendation_domain() {
    assert!(gaseous_specific_attenuation_p676_13(0.999, 1013.25, 7.5, 288.15).is_err());
    assert!(gaseous_specific_attenuation_p676_13(1_000.001, 1013.25, 7.5, 288.15).is_err());
}
