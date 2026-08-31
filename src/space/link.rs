use serde::{Deserialize, Serialize};
use crate::{Ephemeris, Epoch, Result, SimError};

pub const SPEED_OF_LIGHT_M_PER_S: f64 = 299_792_458.0;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SpaceLinkGeometry {
    pub transmit_epoch: Epoch,
    pub receive_epoch: Epoch,
    pub light_time_s: f64,
    pub slant_range_m: f64,
    pub range_rate_m_per_s: f64,
    pub doppler_hz: f64,
    pub iterations: usize,
}

pub fn solve_one_way_link(
    tx: &Ephemeris,
    rx: &Ephemeris,
    receive_epoch: Epoch,
    carrier_frequency_hz: f64,
    max_iterations: usize,
    tolerance_s: f64,
) -> Result<SpaceLinkGeometry> {
    if !carrier_frequency_hz.is_finite() || carrier_frequency_hz <= 0.0 || max_iterations == 0 || !tolerance_s.is_finite() || tolerance_s <= 0.0 {
        return Err(SimError::InvalidArgument("invalid one-way link solver configuration".into()));
    }
    let rx_state = rx.state_at(receive_epoch)?;
    let mut transmit_seconds = receive_epoch.seconds;
    let mut iterations = 0usize;
    for iteration in 0..max_iterations {
        iterations = iteration + 1;
        let tx_epoch = Epoch::new(transmit_seconds, receive_epoch.scale)?;
        let tx_state = tx.state_at(tx_epoch)?;
        let delta = sub(rx_state.position_m, tx_state.position_m);
        let range = norm(delta);
        let next = receive_epoch.seconds - range / SPEED_OF_LIGHT_M_PER_S;
        if (next - transmit_seconds).abs() <= tolerance_s {
            transmit_seconds = next;
            break;
        }
        transmit_seconds = next;
        if iteration + 1 == max_iterations {
            return Err(SimError::InvalidArgument("light-time iteration did not converge".into()));
        }
    }
    let transmit_epoch = Epoch::new(transmit_seconds, receive_epoch.scale)?;
    let tx_state = tx.state_at(transmit_epoch)?;
    let delta = sub(rx_state.position_m, tx_state.position_m);
    let range = norm(delta);
    if range <= f64::EPSILON { return Err(SimError::InvalidArgument("transmitter and receiver positions coincide".into())); }
    let los = scale(delta, 1.0 / range);
    let relative_velocity = sub(rx_state.velocity_m_per_s, tx_state.velocity_m_per_s);
    let range_rate = dot(relative_velocity, los);
    let doppler = -range_rate * carrier_frequency_hz / SPEED_OF_LIGHT_M_PER_S;
    Ok(SpaceLinkGeometry {
        transmit_epoch,
        receive_epoch,
        light_time_s: receive_epoch.seconds - transmit_seconds,
        slant_range_m: range,
        range_rate_m_per_s: range_rate,
        doppler_hz: doppler,
        iterations,
    })
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] { [a[0] - b[0], a[1] - b[1], a[2] - b[2]] }
fn scale(a: [f64; 3], s: f64) -> [f64; 3] { [a[0] * s, a[1] * s, a[2] * s] }
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 { a[0].mul_add(b[0], a[1].mul_add(b[1], a[2] * b[2])) }
fn norm(a: [f64; 3]) -> f64 { dot(a, a).sqrt() }
