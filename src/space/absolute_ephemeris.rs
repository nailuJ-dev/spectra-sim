use serde::{Deserialize, Serialize};

use crate::{Result, SimError};

use super::{AbsoluteEpoch, AbsoluteOrbitState, EopSample, OemSegment, ReferenceFrame, TimeScale};

/// Ephemeris defined on an absolute astronomical time axis.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AbsoluteEphemeris {
    states: Vec<AbsoluteOrbitState>,
}

impl AbsoluteEphemeris {
    pub fn new(mut states: Vec<AbsoluteOrbitState>) -> Result<Self> {
        if states.len() < 2 {
            return Err(SimError::InvalidArgument(
                "absolute ephemeris requires at least two states".into(),
            ));
        }
        states.sort_by(|a, b| a.epoch.julian_date().total_cmp(&b.epoch.julian_date()));
        let frame = states[0].frame;
        let scale = states[0].epoch.scale();
        if states
            .iter()
            .any(|state| state.frame != frame || state.epoch.scale() != scale)
        {
            return Err(SimError::InvalidArgument(
                "absolute ephemeris states must use a common frame and time scale".into(),
            ));
        }
        if states
            .windows(2)
            .any(|pair| pair[0].epoch.julian_date() >= pair[1].epoch.julian_date())
        {
            return Err(SimError::InvalidArgument(
                "absolute ephemeris epochs must be strictly increasing".into(),
            ));
        }
        Ok(Self { states })
    }

    pub fn from_oem_segment(segment: &OemSegment) -> Result<Self> {
        Self::new(segment.states.clone())
    }

    pub fn states(&self) -> &[AbsoluteOrbitState] {
        &self.states
    }

    pub fn frame(&self) -> ReferenceFrame {
        self.states[0].frame
    }

    pub fn time_scale(&self) -> TimeScale {
        self.states[0].epoch.scale()
    }

    pub fn state_at(
        &self,
        epoch: AbsoluteEpoch,
        eop: Option<&EopSample>,
    ) -> Result<AbsoluteOrbitState> {
        if epoch.scale() != self.time_scale() {
            return Err(SimError::InvalidArgument(
                "requested absolute epoch uses a different time scale".into(),
            ));
        }
        let first = &self.states[0];
        let last = &self.states[self.states.len() - 1];
        if epoch.julian_date() < first.epoch.julian_date()
            || epoch.julian_date() > last.epoch.julian_date()
        {
            return Err(SimError::InvalidArgument(
                "requested absolute epoch is outside ephemeris bounds".into(),
            ));
        }
        let upper = self
            .states
            .partition_point(|state| state.epoch.julian_date() < epoch.julian_date());
        if upper < self.states.len()
            && self.states[upper]
                .epoch
                .julian_date()
                .total_cmp(&epoch.julian_date())
                == std::cmp::Ordering::Equal
        {
            return Ok(self.states[upper].clone());
        }
        let b_index = upper.min(self.states.len() - 1);
        let a_index = b_index.saturating_sub(1);
        hermite_absolute(&self.states[a_index], &self.states[b_index], epoch, eop)
    }
}

fn hermite_absolute(
    a: &AbsoluteOrbitState,
    b: &AbsoluteOrbitState,
    epoch: AbsoluteEpoch,
    eop: Option<&EopSample>,
) -> Result<AbsoluteOrbitState> {
    let h = b.epoch.seconds_since(&a.epoch, eop)?;
    if h <= 0.0 {
        return Err(SimError::InvalidArgument(
            "absolute ephemeris interval must be positive".into(),
        ));
    }
    let dt = epoch.seconds_since(&a.epoch, eop)?;
    let u = dt / h;
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
    AbsoluteOrbitState::new(epoch, a.frame, position, velocity)
}
