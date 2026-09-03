use serde::{Deserialize, Serialize};

use crate::{Result, SimError};

pub trait AntennaPattern {
    fn gain_dbi(&self, off_axis_rad: f64, frequency_hz: f64) -> Result<f64>;
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GaussianAntennaPattern {
    pub boresight_gain_dbi: f64,
    pub half_power_beamwidth_rad: f64,
    pub floor_gain_dbi: f64,
}

impl AntennaPattern for GaussianAntennaPattern {
    fn gain_dbi(&self, off_axis_rad: f64, frequency_hz: f64) -> Result<f64> {
        if [
            self.boresight_gain_dbi,
            self.half_power_beamwidth_rad,
            self.floor_gain_dbi,
            off_axis_rad,
            frequency_hz,
        ]
        .iter()
        .any(|v| !v.is_finite())
        {
            return Err(SimError::NonFinite);
        }
        if self.half_power_beamwidth_rad <= 0.0 || frequency_hz <= 0.0 || off_axis_rad < 0.0 {
            return Err(SimError::InvalidArgument(
                "invalid Gaussian antenna pattern input".into(),
            ));
        }
        let loss = 3.0 * (2.0 * off_axis_rad / self.half_power_beamwidth_rad).powi(2);
        Ok((self.boresight_gain_dbi - loss).max(self.floor_gain_dbi))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ParabolicDish {
    pub diameter_m: f64,
    pub aperture_efficiency: f64,
}

impl ParabolicDish {
    pub fn boresight_gain_dbi(&self, frequency_hz: f64) -> Result<f64> {
        if !self.diameter_m.is_finite()
            || !self.aperture_efficiency.is_finite()
            || !frequency_hz.is_finite()
        {
            return Err(SimError::NonFinite);
        }
        if self.diameter_m <= 0.0
            || !(0.0..=1.0).contains(&self.aperture_efficiency)
            || self.aperture_efficiency == 0.0
            || frequency_hz <= 0.0
        {
            return Err(SimError::InvalidArgument(
                "invalid parabolic dish input".into(),
            ));
        }
        let wavelength = 299_792_458.0 / frequency_hz;
        let gain_linear = self.aperture_efficiency
            * (std::f64::consts::PI * self.diameter_m / wavelength).powi(2);
        Ok(10.0 * gain_linear.log10())
    }

    pub fn approximate_half_power_beamwidth_rad(&self, frequency_hz: f64) -> Result<f64> {
        if self.diameter_m <= 0.0 || frequency_hz <= 0.0 || !frequency_hz.is_finite() {
            return Err(SimError::InvalidArgument(
                "invalid parabolic dish input".into(),
            ));
        }
        let wavelength = 299_792_458.0 / frequency_hz;
        Ok((70.0 * wavelength / self.diameter_m).to_radians())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TabulatedAntennaPattern {
    /// Sorted `(off_axis_rad, gain_dbi)` samples.
    pub samples: Vec<(f64, f64)>,
}

impl AntennaPattern for TabulatedAntennaPattern {
    fn gain_dbi(&self, off_axis_rad: f64, frequency_hz: f64) -> Result<f64> {
        if !off_axis_rad.is_finite() || !frequency_hz.is_finite() || frequency_hz <= 0.0 {
            return Err(SimError::NonFinite);
        }
        if off_axis_rad < 0.0
            || self.samples.len() < 2
            || self.samples[0].0 < 0.0
            || self.samples.windows(2).any(|p| p[0].0 >= p[1].0)
        {
            return Err(SimError::InvalidArgument(
                "tabulated antenna pattern must be strictly sorted".into(),
            ));
        }
        if self
            .samples
            .iter()
            .flat_map(|p| [p.0, p.1])
            .any(|v| !v.is_finite())
        {
            return Err(SimError::NonFinite);
        }
        if off_axis_rad <= self.samples[0].0 {
            return Ok(self.samples[0].1);
        }
        if off_axis_rad >= self.samples[self.samples.len() - 1].0 {
            return Ok(self.samples[self.samples.len() - 1].1);
        }
        let upper = self.samples.partition_point(|p| p.0 < off_axis_rad);
        let a = self.samples[upper - 1];
        let b = self.samples[upper];
        let t = (off_axis_rad - a.0) / (b.0 - a.0);
        Ok(a.1 + (b.1 - a.1) * t)
    }
}

/// Pointing loss relative to the pattern boresight gain.
pub fn pointing_loss_db<P: AntennaPattern>(
    pattern: &P,
    off_axis_rad: f64,
    frequency_hz: f64,
) -> Result<f64> {
    let boresight = pattern.gain_dbi(0.0, frequency_hz)?;
    let actual = pattern.gain_dbi(off_axis_rad, frequency_hz)?;
    Ok((boresight - actual).max(0.0))
}
