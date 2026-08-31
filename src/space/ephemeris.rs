use crate::{Epoch, OrbitState, Result, SimError};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ephemeris {
    states: Vec<OrbitState>,
}

impl Ephemeris {
    pub fn new(mut states: Vec<OrbitState>) -> Result<Self> {
        if states.len() < 2 {
            return Err(SimError::InvalidArgument(
                "ephemeris requires at least two states".into(),
            ));
        }
        states.sort_by(|a, b| a.epoch.seconds.total_cmp(&b.epoch.seconds));
        let frame = states[0].frame;
        let scale = states[0].epoch.scale;
        if states
            .iter()
            .any(|state| state.frame != frame || state.epoch.scale != scale)
        {
            return Err(SimError::InvalidArgument(
                "ephemeris states must use a common frame and time scale".into(),
            ));
        }
        if states
            .windows(2)
            .any(|pair| pair[0].epoch.seconds >= pair[1].epoch.seconds)
        {
            return Err(SimError::InvalidArgument(
                "ephemeris epochs must be strictly increasing".into(),
            ));
        }
        Ok(Self { states })
    }

    pub fn states(&self) -> &[OrbitState] {
        &self.states
    }

    pub fn state_at(&self, epoch: Epoch) -> Result<OrbitState> {
        if epoch.scale != self.states[0].epoch.scale {
            return Err(SimError::InvalidArgument(
                "requested epoch uses a different time scale".into(),
            ));
        }
        let first = self
            .states
            .first()
            .ok_or_else(|| SimError::InvalidArgument("empty ephemeris".into()))?;
        let last = self
            .states
            .last()
            .ok_or_else(|| SimError::InvalidArgument("empty ephemeris".into()))?;
        if epoch.seconds < first.epoch.seconds || epoch.seconds > last.epoch.seconds {
            return Err(SimError::InvalidArgument(
                "requested epoch is outside ephemeris bounds".into(),
            ));
        }
        let upper = self
            .states
            .partition_point(|state| state.epoch.seconds < epoch.seconds);
        if upper < self.states.len()
            && (self.states[upper].epoch.seconds - epoch.seconds).abs() <= f64::EPSILON
        {
            return Ok(self.states[upper]);
        }
        let b_index = upper.min(self.states.len() - 1);
        let a_index = b_index.saturating_sub(1);
        hermite(self.states[a_index], self.states[b_index], epoch)
    }
}

fn hermite(a: OrbitState, b: OrbitState, epoch: Epoch) -> Result<OrbitState> {
    let h = b.epoch.seconds - a.epoch.seconds;
    if h <= 0.0 {
        return Err(SimError::InvalidArgument(
            "ephemeris interval must be positive".into(),
        ));
    }
    let u = (epoch.seconds - a.epoch.seconds) / h;
    let h00 = 2.0 * u.powi(3) - 3.0 * u.powi(2) + 1.0;
    let h10 = u.powi(3) - 2.0 * u.powi(2) + u;
    let h01 = -2.0 * u.powi(3) + 3.0 * u.powi(2);
    let h11 = u.powi(3) - u.powi(2);
    let dh00 = (6.0 * u.powi(2) - 6.0 * u) / h;
    let dh10 = 3.0 * u.powi(2) - 4.0 * u + 1.0;
    let dh01 = (-6.0 * u.powi(2) + 6.0 * u) / h;
    let dh11 = 3.0 * u.powi(2) - 2.0 * u;
    let mut position = [0.0; 3];
    let mut velocity = [0.0; 3];
    for axis in 0..3 {
        position[axis] = h00 * a.position_m[axis]
            + h10 * h * a.velocity_m_per_s[axis]
            + h01 * b.position_m[axis]
            + h11 * h * b.velocity_m_per_s[axis];
        velocity[axis] = dh00 * a.position_m[axis]
            + dh10 * a.velocity_m_per_s[axis]
            + dh01 * b.position_m[axis]
            + dh11 * b.velocity_m_per_s[axis];
    }
    OrbitState::new(epoch, a.frame, position, velocity)
}
