use serde::{Deserialize, Serialize};

use crate::{
    simulate_monostatic_radar_with_propagation, wavelength_m, DeterministicRng, Environment,
    IsacConfig, KinematicState, PropagationModel, RadarConfig, RadarMeasurement, Result, Target,
    SPEED_OF_LIGHT_MPS,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IsacMeasurement {
    pub radar: RadarMeasurement,
    pub subcarrier_spacing_hz: f64,
    pub symbols: usize,
    pub nominal_unambiguous_range_m: f64,
    pub coherent_time_s: f64,
    pub range_resolution_m: f64,
    pub radial_velocity_resolution_mps: f64,
    pub angle_resolution_deg: f64,
}

pub fn simulate_isac(
    sensor: KinematicState,
    target: &Target,
    config: &IsacConfig,
    environment: Environment,
) -> Result<IsacMeasurement> {
    let mut rng = DeterministicRng::new(0);
    simulate_isac_with_propagation(
        sensor,
        target,
        config,
        environment,
        PropagationModel::FreeSpace,
        &mut rng,
    )
}

pub fn simulate_isac_with_propagation(
    sensor: KinematicState,
    target: &Target,
    config: &IsacConfig,
    environment: Environment,
    propagation: PropagationModel,
    rng: &mut DeterministicRng,
) -> Result<IsacMeasurement> {
    config.validate()?;
    propagation.validate()?;
    let symbol_duration = 1.0 / config.subcarrier_spacing_hz;
    let coherent_time = symbol_duration * config.symbols as f64;
    let radar_config = RadarConfig {
        carrier_frequency_hz: config.carrier_frequency_hz,
        bandwidth_hz: config.bandwidth_hz,
        tx_power_dbm: config.tx_power_dbm,
        tx_gain_dbi: config.tx_gain_dbi,
        rx_gain_dbi: config.rx_gain_dbi,
        tx_pattern: config.tx_pattern.clone(),
        rx_pattern: config.rx_pattern.clone(),
        noise_figure_db: config.noise_figure_db,
        system_loss_db: config.system_loss_db,
        coherent_time_s: coherent_time,
        array_elements: config.array_elements,
        element_spacing_lambda: config.element_spacing_lambda,
    };
    let radar = simulate_monostatic_radar_with_propagation(
        sensor,
        target,
        &radar_config,
        environment,
        propagation,
        rng,
    )?;
    let lambda = wavelength_m(config.carrier_frequency_hz)?;
    let nominal_unambiguous_range = SPEED_OF_LIGHT_MPS / (2.0 * config.subcarrier_spacing_hz);
    let velocity_resolution = lambda / (2.0 * coherent_time);
    Ok(IsacMeasurement {
        range_resolution_m: SPEED_OF_LIGHT_MPS / (2.0 * config.bandwidth_hz),
        radial_velocity_resolution_mps: velocity_resolution,
        angle_resolution_deg: radar.angle_resolution_deg,
        radar,
        subcarrier_spacing_hz: config.subcarrier_spacing_hz,
        symbols: config.symbols,
        nominal_unambiguous_range_m: nominal_unambiguous_range,
        coherent_time_s: coherent_time,
    })
}

pub fn cuas_reference_features(
    measurement: &RadarMeasurement,
    rf_energy: f32,
    burstiness: f32,
    angular_rate_dps: f32,
) -> Vec<f32> {
    let az = measurement.azimuth_deg.to_radians();
    let el = measurement.elevation_deg.to_radians();
    let range_norm = (measurement.range_m.ln_1p() / 100_000.0_f64.ln_1p()).clamp(0.0, 1.0) as f32;
    let velocity_norm = (measurement.radial_velocity_mps / 200.0).clamp(-2.0, 2.0) as f32;
    let snr_norm = (measurement.snr_db / 40.0).clamp(-2.0, 2.0);
    let doppler_norm = (f64::from(measurement.doppler_spread_hz).ln_1p() / 1_000.0_f64.ln_1p())
        .clamp(0.0, 1.0) as f32;
    let rcs_norm = (measurement.rcs_m2.ln_1p() / 100.0_f64.ln_1p()).clamp(0.0, 1.0) as f32;
    let angular_rate_norm = (angular_rate_dps / 180.0).clamp(-2.0, 2.0);
    vec![
        range_norm,
        velocity_norm,
        az.sin() as f32,
        az.cos() as f32,
        el.sin() as f32,
        el.cos() as f32,
        snr_norm,
        doppler_norm,
        measurement.micro_doppler_index,
        rcs_norm,
        rf_energy.clamp(0.0, 1.0),
        burstiness.clamp(0.0, 1.0),
        angular_rate_norm,
    ]
}
