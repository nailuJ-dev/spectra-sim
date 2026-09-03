use serde::{Deserialize, Serialize};

use crate::{physics::free_space_path_loss_db, Result, SimError};

pub const BOLTZMANN_DBW_PER_K_PER_HZ: f64 = -228.599_167_173_217_67;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LinkBudgetInput {
    pub frequency_hz: f64,
    pub range_m: f64,
    pub tx_power_dbw: f64,
    pub tx_feeder_loss_db: f64,
    pub tx_gain_dbi: f64,
    pub rx_gain_dbi: f64,
    pub rx_feeder_loss_db: f64,
    pub propagation_loss_db: f64,
    pub polarization_loss_db: f64,
    pub pointing_loss_db: f64,
    pub other_loss_db: f64,
    pub system_noise_temperature_k: f64,
    pub bandwidth_hz: f64,
    pub bit_rate_bps: Option<f64>,
    pub required_eb_n0_db: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LinkBudgetResult {
    pub free_space_loss_db: f64,
    pub eirp_dbw: f64,
    pub received_power_dbw: f64,
    pub g_over_t_db_per_k: f64,
    pub c_n0_db_hz: f64,
    pub c_n_db: f64,
    pub eb_n0_db: Option<f64>,
    pub link_margin_db: Option<f64>,
}

pub fn compute_link_budget(input: LinkBudgetInput) -> Result<LinkBudgetResult> {
    let values = [
        input.frequency_hz,
        input.range_m,
        input.tx_power_dbw,
        input.tx_feeder_loss_db,
        input.tx_gain_dbi,
        input.rx_gain_dbi,
        input.rx_feeder_loss_db,
        input.propagation_loss_db,
        input.polarization_loss_db,
        input.pointing_loss_db,
        input.other_loss_db,
        input.system_noise_temperature_k,
        input.bandwidth_hz,
    ];
    if values.iter().any(|v| !v.is_finite())
        || input.bit_rate_bps.is_some_and(|v| !v.is_finite())
        || input.required_eb_n0_db.is_some_and(|v| !v.is_finite())
    {
        return Err(SimError::NonFinite);
    }
    if input.system_noise_temperature_k <= 0.0
        || input.bandwidth_hz <= 0.0
        || input.bit_rate_bps.is_some_and(|v| v <= 0.0)
    {
        return Err(SimError::InvalidArgument(
            "invalid link-budget noise/rate input".into(),
        ));
    }
    for (label, loss) in [
        ("tx feeder loss", input.tx_feeder_loss_db),
        ("rx feeder loss", input.rx_feeder_loss_db),
        ("propagation loss", input.propagation_loss_db),
        ("polarization loss", input.polarization_loss_db),
        ("pointing loss", input.pointing_loss_db),
        ("other loss", input.other_loss_db),
    ] {
        if loss < 0.0 {
            return Err(SimError::InvalidArgument(format!(
                "{label} must be non-negative"
            )));
        }
    }
    let fspl = free_space_path_loss_db(input.frequency_hz, input.range_m)?;
    let eirp = input.tx_power_dbw - input.tx_feeder_loss_db + input.tx_gain_dbi;
    let received = eirp
        - fspl
        - input.propagation_loss_db
        - input.polarization_loss_db
        - input.pointing_loss_db
        - input.other_loss_db
        + input.rx_gain_dbi
        - input.rx_feeder_loss_db;
    let g_over_t = input.rx_gain_dbi
        - input.rx_feeder_loss_db
        - 10.0 * input.system_noise_temperature_k.log10();
    let c_n0 = eirp
        - fspl
        - input.propagation_loss_db
        - input.polarization_loss_db
        - input.pointing_loss_db
        - input.other_loss_db
        + g_over_t
        - BOLTZMANN_DBW_PER_K_PER_HZ;
    let c_n = c_n0 - 10.0 * input.bandwidth_hz.log10();
    let eb_n0 = input.bit_rate_bps.map(|rate| c_n0 - 10.0 * rate.log10());
    let margin = match (eb_n0, input.required_eb_n0_db) {
        (Some(actual), Some(required)) => Some(actual - required),
        _ => None,
    };
    Ok(LinkBudgetResult {
        free_space_loss_db: fspl,
        eirp_dbw: eirp,
        received_power_dbw: received,
        g_over_t_db_per_k: g_over_t,
        c_n0_db_hz: c_n0,
        c_n_db: c_n,
        eb_n0_db: eb_n0,
        link_margin_db: margin,
    })
}
