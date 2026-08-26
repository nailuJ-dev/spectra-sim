use serde::{Deserialize, Serialize};

use crate::{
    azimuth_elevation_deg, db_to_linear, line_of_sight_unit, monostatic_doppler_hz,
    radial_velocity_mps, thermal_noise_dbm, wavelength_m, Environment, KinematicState, RadarConfig,
    Result, SimError, Target,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RadarMeasurement {
    pub range_m: f64,
    pub radial_velocity_mps: f64,
    pub azimuth_deg: f64,
    pub elevation_deg: f64,
    pub received_power_dbm: f64,
    pub snr_db: f32,
    pub doppler_hz: f64,
    pub doppler_spread_hz: f32,
    pub micro_doppler_index: f32,
    pub rcs_m2: f64,
    pub range_resolution_m: f64,
    pub velocity_resolution_mps: f64,
    pub angle_resolution_deg: f64,
}

pub fn simulate_monostatic_radar(
    sensor: KinematicState,
    target: &Target,
    config: &RadarConfig,
    environment: Environment,
) -> Result<RadarMeasurement> {
    target.validate()?;
    config.validate()?;
    environment.validate()?;
    sensor.validate()?;
    let range = sensor
        .position_enu_m
        .distance(target.state.position_enu_m)
        .max(0.01);
    let radial = radial_velocity_mps(sensor, target.state)?;
    let (az, el) = azimuth_elevation_deg(sensor.position_enu_m, target.state.position_enu_m)?;
    let lambda = wavelength_m(config.carrier_frequency_hz)?;
    let tx_watts = 10.0_f64.powf((config.tx_power_dbm - 30.0) / 10.0);
    let gt = db_to_linear(config.tx_gain_dbi);
    let gr = db_to_linear(config.rx_gain_dbi);
    let losses = db_to_linear(config.system_loss_db + environment.extra_loss_db);
    let numerator = tx_watts * gt * gr * lambda.powi(2) * target.rcs_m2.max(1e-12);
    let denominator = (4.0 * std::f64::consts::PI).powi(3) * range.powi(4) * losses;
    let received_watts = (numerator / denominator).max(1e-300);
    let received_dbm = 10.0 * (received_watts * 1_000.0).log10();
    let noise_dbm = thermal_noise_dbm(
        config.bandwidth_hz,
        environment.temperature_k,
        config.noise_figure_db,
    )?;
    let snr = received_dbm - noise_dbm;
    let doppler = monostatic_doppler_hz(config.carrier_frequency_hz, radial)?;
    let tip_velocity = std::f64::consts::TAU * target.rotor_radius_m * target.rotor_rpm / 60.0;
    let los = line_of_sight_unit(sensor.position_enu_m, target.state.position_enu_m)?;
    let rotor_aspect = los.x.hypot(los.y).clamp(0.0, 1.0);
    let micro_span = if target.rotor_count > 0 {
        (2.0 * tip_velocity * rotor_aspect / lambda).abs()
    } else {
        0.0
    };
    let micro_index = if micro_span <= 0.0 {
        0.0
    } else {
        (micro_span / (micro_span + 500.0)).clamp(0.0, 1.0) as f32
    };
    let range_resolution = crate::SPEED_OF_LIGHT_MPS / (2.0 * config.bandwidth_hz);
    let velocity_resolution = lambda / (2.0 * config.coherent_time_s);
    let aperture_lambda =
        ((config.array_elements.saturating_sub(1)) as f64 * config.element_spacing_lambda).max(0.5);
    let angle_resolution = (1.0 / aperture_lambda).min(1.0).asin().to_degrees();
    if !snr.is_finite() {
        return Err(SimError::NonFinite);
    }
    Ok(RadarMeasurement {
        range_m: range,
        radial_velocity_mps: radial,
        azimuth_deg: az,
        elevation_deg: el,
        received_power_dbm: received_dbm,
        snr_db: snr as f32,
        doppler_hz: doppler,
        doppler_spread_hz: micro_span.min(f32::MAX as f64) as f32,
        micro_doppler_index: micro_index,
        rcs_m2: target.rcs_m2,
        range_resolution_m: range_resolution,
        velocity_resolution_mps: velocity_resolution,
        angle_resolution_deg: angle_resolution,
    })
}
