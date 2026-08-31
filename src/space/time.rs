use serde::{Deserialize, Serialize};
use crate::{Result, SimError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimeScale { Utc, Tai, Tt, Ut1, Gps }

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Epoch {
    pub seconds: f64,
    pub scale: TimeScale,
}

impl Epoch {
    pub fn new(seconds: f64, scale: TimeScale) -> Result<Self> {
        if !seconds.is_finite() { return Err(SimError::NonFinite); }
        Ok(Self { seconds, scale })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EarthOrientation {
    pub ut1_minus_utc_s: f64,
    pub polar_motion_x_rad: f64,
    pub polar_motion_y_rad: f64,
}

impl EarthOrientation {
    pub fn validate(&self) -> Result<()> {
        if !self.ut1_minus_utc_s.is_finite() || !self.polar_motion_x_rad.is_finite() || !self.polar_motion_y_rad.is_finite() {
            return Err(SimError::NonFinite);
        }
        Ok(())
    }
}
