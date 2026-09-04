use crate::{
    db_to_linear, linear_to_db, wavelength_m, Complex64, DeterministicRng, Environment,
    KinematicState, PropagationModel, Result, SimError, Vec3, BOLTZMANN_J_PER_K,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PropagationChannel {
    pub distance_m: f64,
    pub free_space_path_loss_db: f64,
    pub fading_db: f64,
    /// One-way excess loss beyond free-space spreading, including the explicit
    /// environment loss and the selected propagation-model correction.
    pub excess_loss_db: f64,
    pub total_one_way_loss_db: f64,
    pub complex_channel: Complex64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LinkBudget {
    pub distance_m: f64,
    pub path_loss_db: f64,
    pub fading_db: f64,
    pub total_loss_db: f64,
    pub received_power_dbm: f64,
    pub radial_velocity_mps: f64,
    pub doppler_hz: f64,
    pub complex_channel: Complex64,
}

pub fn free_space_path_loss_db(frequency_hz: f64, distance_m: f64) -> Result<f64> {
    if !distance_m.is_finite() || distance_m <= 0.0 {
        return Err(SimError::InvalidArgument(
            "distance_m must be positive".into(),
        ));
    }
    let lambda = wavelength_m(frequency_hz)?;
    Ok(20.0 * (4.0 * std::f64::consts::PI * distance_m / lambda).log10())
}

pub fn thermal_noise_dbm(
    bandwidth_hz: f64,
    temperature_k: f64,
    noise_figure_db: f64,
) -> Result<f64> {
    if !bandwidth_hz.is_finite()
        || bandwidth_hz <= 0.0
        || !temperature_k.is_finite()
        || temperature_k <= 0.0
        || !noise_figure_db.is_finite()
        || noise_figure_db < 0.0
    {
        return Err(SimError::InvalidArgument(
            "invalid thermal-noise parameters".into(),
        ));
    }
    let watts = BOLTZMANN_J_PER_K * temperature_k * bandwidth_hz;
    Ok(10.0 * (watts * 1_000.0).max(1e-300).log10() + noise_figure_db)
}

pub fn line_of_sight_unit(from: Vec3, to: Vec3) -> Result<Vec3> {
    to.sub_vec(from).normalized()
}

pub fn radial_velocity_mps(sensor: KinematicState, target: KinematicState) -> Result<f64> {
    let los = line_of_sight_unit(sensor.position_enu_m, target.position_enu_m)?;
    Ok(target
        .velocity_enu_mps
        .sub_vec(sensor.velocity_enu_mps)
        .dot(los))
}

pub fn one_way_doppler_hz(frequency_hz: f64, radial_velocity_mps: f64) -> Result<f64> {
    Ok(-radial_velocity_mps / wavelength_m(frequency_hz)?)
}

pub fn monostatic_doppler_hz(frequency_hz: f64, radial_velocity_mps: f64) -> Result<f64> {
    Ok(-2.0 * radial_velocity_mps / wavelength_m(frequency_hz)?)
}

pub fn azimuth_elevation_deg(sensor: Vec3, target: Vec3) -> Result<(f64, f64)> {
    let delta = target.sub_vec(sensor);
    let range = delta.norm();
    if range <= 1e-12 {
        return Err(SimError::InvalidArgument(
            "sensor and target positions coincide".into(),
        ));
    }
    let az = delta.x.atan2(delta.y).to_degrees();
    let el = (delta.z / range).clamp(-1.0, 1.0).asin().to_degrees();
    Ok((az, el))
}

/// Computes a deterministic one-way propagation channel. Free-space spreading
/// is separated from the model/environment excess term so monostatic radar can
/// apply the latter twice while retaining the classical R^4 radar equation.
pub fn propagation_channel(
    tx_state: KinematicState,
    rx_state: KinematicState,
    frequency_hz: f64,
    environment: Environment,
    model: PropagationModel,
    rng: &mut DeterministicRng,
) -> Result<PropagationChannel> {
    tx_state.validate()?;
    rx_state.validate()?;
    environment.validate()?;
    model.validate()?;

    let distance = tx_state
        .position_enu_m
        .distance(rx_state.position_enu_m)
        .max(0.01);
    let fspl = free_space_path_loss_db(frequency_hz, distance)?;
    let lambda = wavelength_m(frequency_hz)?;
    let direct_phase = -std::f64::consts::TAU * distance / lambda;
    let mut channel = Complex64::from_polar(1.0, direct_phase);
    let mut model_delta_db = 0.0;
    let mut fading_db = 0.0;

    match model {
        PropagationModel::FreeSpace => {}
        PropagationModel::TwoRay => {
            let horizontal = ((tx_state.position_enu_m.x - rx_state.position_enu_m.x).powi(2)
                + (tx_state.position_enu_m.y - rx_state.position_enu_m.y).powi(2))
            .sqrt();
            let reflected = (horizontal.powi(2)
                + (tx_state.position_enu_m.z + rx_state.position_enu_m.z).powi(2))
            .sqrt()
            .max(0.01);
            let reflected_phase = -std::f64::consts::TAU * reflected / lambda;
            let reflected_amp = environment.ground_reflection_coefficient * distance / reflected;
            channel = channel + Complex64::from_polar(reflected_amp, reflected_phase);
            model_delta_db = -linear_to_db(channel.magnitude_squared().max(1e-12));
        }
        PropagationModel::Rician {
            k_factor_db,
            shadowing_std_db,
        } => {
            let k = db_to_linear(k_factor_db).max(0.0);
            let los_amp = (k / (k + 1.0)).sqrt();
            let scatter_std = (1.0 / (2.0 * (k + 1.0))).sqrt();
            let scatter =
                Complex64::new(rng.normal(0.0, scatter_std)?, rng.normal(0.0, scatter_std)?);
            channel = Complex64::from_polar(los_amp, direct_phase) + scatter;
            fading_db = linear_to_db(channel.magnitude_squared().max(1e-12));
            model_delta_db = -fading_db + rng.normal(0.0, shadowing_std_db)?;
        }
    }

    let excess_loss_db = environment.extra_loss_db + model_delta_db;
    Ok(PropagationChannel {
        distance_m: distance,
        free_space_path_loss_db: fspl,
        fading_db,
        excess_loss_db,
        total_one_way_loss_db: fspl + excess_loss_db,
        complex_channel: channel,
    })
}

#[allow(clippy::too_many_arguments)]
pub fn link_budget(
    tx_power_dbm: f64,
    tx_gain_dbi: f64,
    rx_gain_dbi: f64,
    tx_state: KinematicState,
    rx_state: KinematicState,
    frequency_hz: f64,
    environment: Environment,
    model: PropagationModel,
    rng: &mut DeterministicRng,
) -> Result<LinkBudget> {
    let propagation =
        propagation_channel(tx_state, rx_state, frequency_hz, environment, model, rng)?;
    let radial = radial_velocity_mps(rx_state, tx_state)?;
    let doppler = one_way_doppler_hz(frequency_hz, radial)?;
    let received = tx_power_dbm + tx_gain_dbi + rx_gain_dbi - propagation.total_one_way_loss_db;
    Ok(LinkBudget {
        distance_m: propagation.distance_m,
        path_loss_db: propagation.free_space_path_loss_db,
        fading_db: propagation.fading_db,
        total_loss_db: propagation.total_one_way_loss_db,
        received_power_dbm: received,
        radial_velocity_mps: radial,
        doppler_hz: doppler,
        complex_channel: propagation.complex_channel,
    })
}
