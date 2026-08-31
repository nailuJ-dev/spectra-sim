use crate::{Epoch, Result, SimError};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReferenceFrame {
    Icrf,
    Gcrf,
    Eme2000,
    Teme,
    Itrf,
    Enu,
    Ned,
    SpacecraftBody,
    Antenna,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct OrbitState {
    pub epoch: Epoch,
    pub frame: ReferenceFrame,
    pub position_m: [f64; 3],
    pub velocity_m_per_s: [f64; 3],
}

impl OrbitState {
    pub fn new(
        epoch: Epoch,
        frame: ReferenceFrame,
        position_m: [f64; 3],
        velocity_m_per_s: [f64; 3],
    ) -> Result<Self> {
        if position_m
            .iter()
            .chain(velocity_m_per_s.iter())
            .any(|value| !value.is_finite())
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
