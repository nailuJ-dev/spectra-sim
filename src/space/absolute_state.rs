use serde::{Deserialize, Serialize};

use crate::{Result, SimError};

use super::{AbsoluteEpoch, ReferenceFrame};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AbsoluteOrbitState {
    pub epoch: AbsoluteEpoch,
    pub frame: ReferenceFrame,
    pub position_m: [f64; 3],
    pub velocity_m_per_s: [f64; 3],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acceleration_m_per_s2: Option<[f64; 3]>,
}

impl AbsoluteOrbitState {
    pub fn new(
        epoch: AbsoluteEpoch,
        frame: ReferenceFrame,
        position_m: [f64; 3],
        velocity_m_per_s: [f64; 3],
    ) -> Result<Self> {
        Self::new_with_acceleration(epoch, frame, position_m, velocity_m_per_s, None)
    }

    pub fn new_with_acceleration(
        epoch: AbsoluteEpoch,
        frame: ReferenceFrame,
        position_m: [f64; 3],
        velocity_m_per_s: [f64; 3],
        acceleration_m_per_s2: Option<[f64; 3]>,
    ) -> Result<Self> {
        if position_m
            .iter()
            .chain(velocity_m_per_s.iter())
            .chain(acceleration_m_per_s2.iter().flatten())
            .any(|v| !v.is_finite())
        {
            return Err(SimError::NonFinite);
        }
        Ok(Self {
            epoch,
            frame,
            position_m,
            velocity_m_per_s,
            acceleration_m_per_s2,
        })
    }
}
