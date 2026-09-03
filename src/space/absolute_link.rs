use serde::{Deserialize, Serialize};

use crate::{Result, SimError};

use super::{AbsoluteEphemeris, AbsoluteEpoch, EopSample, SPEED_OF_LIGHT_M_PER_S};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AbsoluteSpaceLinkGeometry {
    pub transmit_epoch: AbsoluteEpoch,
    pub receive_epoch: AbsoluteEpoch,
    pub light_time_s: f64,
    pub slant_range_m: f64,
    pub range_rate_m_per_s: f64,
    pub doppler_hz: f64,
    pub iterations: usize,
}

pub fn solve_absolute_one_way_link(
    tx: &AbsoluteEphemeris,
    rx: &AbsoluteEphemeris,
    receive_epoch: AbsoluteEpoch,
    carrier_frequency_hz: f64,
    max_iterations: usize,
    tolerance_s: f64,
    eop: Option<&EopSample>,
) -> Result<AbsoluteSpaceLinkGeometry> {
    if tx.frame() != rx.frame() {
        return Err(SimError::InvalidArgument(
            "reference frame mismatch in absolute one-way link solver".into(),
        ));
    }
    if tx.time_scale() != rx.time_scale() || receive_epoch.scale() != tx.time_scale() {
        return Err(SimError::InvalidArgument(
            "time-scale mismatch in absolute one-way link solver".into(),
        ));
    }
    if !carrier_frequency_hz.is_finite()
        || carrier_frequency_hz <= 0.0
        || max_iterations == 0
        || !tolerance_s.is_finite()
        || tolerance_s <= 0.0
    {
        return Err(SimError::InvalidArgument(
            "invalid absolute one-way link solver configuration".into(),
        ));
    }
    let rx_state = rx.state_at(receive_epoch, eop)?;
    let mut transmit_epoch = receive_epoch;
    let mut iterations = 0usize;
    for iteration in 0..max_iterations {
        iterations = iteration + 1;
        let tx_state = tx.state_at(transmit_epoch, eop)?;
        let delta = sub(rx_state.position_m, tx_state.position_m);
        let range = norm(delta);
        let next = receive_epoch.shift_si_seconds(-range / SPEED_OF_LIGHT_M_PER_S, eop)?;
        let correction = next.seconds_since(&transmit_epoch, eop)?.abs();
        transmit_epoch = next;
        if correction <= tolerance_s {
            break;
        }
        if iteration + 1 == max_iterations {
            return Err(SimError::InvalidArgument(
                "absolute light-time iteration did not converge".into(),
            ));
        }
    }
    let tx_state = tx.state_at(transmit_epoch, eop)?;
    let delta = sub(rx_state.position_m, tx_state.position_m);
    let range = norm(delta);
    if range <= f64::EPSILON {
        return Err(SimError::InvalidArgument(
            "transmitter and receiver positions coincide".into(),
        ));
    }
    let los = scale(delta, 1.0 / range);
    let relative_velocity = sub(rx_state.velocity_m_per_s, tx_state.velocity_m_per_s);
    let range_rate = dot(relative_velocity, los);
    let doppler = -range_rate * carrier_frequency_hz / SPEED_OF_LIGHT_M_PER_S;
    Ok(AbsoluteSpaceLinkGeometry {
        transmit_epoch,
        receive_epoch,
        light_time_s: receive_epoch.seconds_since(&transmit_epoch, eop)?,
        slant_range_m: range,
        range_rate_m_per_s: range_rate,
        doppler_hz: doppler,
        iterations,
    })
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn scale(a: [f64; 3], value: f64) -> [f64; 3] {
    [a[0] * value, a[1] * value, a[2] * value]
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0].mul_add(b[0], a[1].mul_add(b[1], a[2] * b[2]))
}
fn norm(a: [f64; 3]) -> f64 {
    dot(a, a).sqrt()
}
