use serde::{Deserialize, Serialize};

use crate::{Result, SimError};

use super::{AbsoluteEpoch, ReferenceFrame};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AbsoluteOrbitState {
    pub epoch: AbsoluteEpoch,
    pub frame: ReferenceFrame,
    pub position_m: [f64; 3],
    pub velocity_m_per_s: [f64; 3],
}

impl AbsoluteOrbitState {
    pub fn new(
        epoch: AbsoluteEpoch,
        frame: ReferenceFrame,
        position_m: [f64; 3],
        velocity_m_per_s: [f64; 3],
    ) -> Result<Self> {
        if position_m
            .iter()
            .chain(velocity_m_per_s.iter())
            .any(|v| !v.is_finite())
        {
            return Err(SimError::NonFinite);
        }
        Ok(Self {
            epoch,
            frame,
            position_m,
            velocity_m_per_s,
        })
    }
}
