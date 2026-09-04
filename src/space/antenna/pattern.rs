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

/// Serializable antenna-pattern model used by terrestrial scenario entities.
///
/// The legacy scalar `gain_dbi` remains the boresight gain. Pattern values are
/// applied relative to that gain, except `floor_gain_dbi`, which is an absolute
/// lower gain bound.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AntennaPatternModel {
    #[default]
    Isotropic,
    Gaussian {
        half_power_beamwidth_deg: f64,
        floor_gain_dbi: f64,
    },
    EllipticalGaussian {
        horizontal_half_power_beamwidth_deg: f64,
        vertical_half_power_beamwidth_deg: f64,
        floor_gain_dbi: f64,
    },
    /// `(off_axis_deg, relative_gain_db)` samples. Relative gain must be <= 0.
    TabulatedRelative {
        samples: Vec<(f64, f64)>,
    },
}

impl AntennaPatternModel {
    pub fn validate(&self, boresight_gain_dbi: f64) -> Result<()> {
        if !boresight_gain_dbi.is_finite() {
            return Err(SimError::NonFinite);
        }
        match self {
            Self::Isotropic => Ok(()),
            Self::Gaussian {
                half_power_beamwidth_deg,
                floor_gain_dbi,
            } => validate_gaussian(*half_power_beamwidth_deg, *floor_gain_dbi, boresight_gain_dbi),
            Self::EllipticalGaussian {
                horizontal_half_power_beamwidth_deg,
                vertical_half_power_beamwidth_deg,
                floor_gain_dbi,
            } => {
                validate_gaussian(
                    *horizontal_half_power_beamwidth_deg,
                    *floor_gain_dbi,
                    boresight_gain_dbi,
                )?;
                validate_gaussian(
                    *vertical_half_power_beamwidth_deg,
                    *floor_gain_dbi,
                    boresight_gain_dbi,
                )
            }
            Self::TabulatedRelative { samples } => {
                if samples.len() < 2
                    || samples[0].0 < 0.0
                    || samples.windows(2).any(|pair| pair[0].0 >= pair[1].0)
                    || samples
                        .iter()
                        .any(|(angle, gain)| !angle.is_finite() || !gain.is_finite() || *gain > 0.0)
                {
                    return Err(SimError::InvalidArgument(
                        "tabulated_relative antenna pattern must contain >=2 strictly sorted finite samples with relative gain <= 0 dB".into(),
                    ));
                }
                Ok(())
            }
        }
    }

    pub fn gain_dbi(
        &self,
        boresight_gain_dbi: f64,
        off_axis_rad: f64,
        horizontal_angle_rad: f64,
        vertical_angle_rad: f64,
        frequency_hz: f64,
    ) -> Result<f64> {
        self.validate(boresight_gain_dbi)?;
        if [off_axis_rad, horizontal_angle_rad, vertical_angle_rad, frequency_hz]
            .iter()
            .any(|value| !value.is_finite())
            || frequency_hz <= 0.0
            || off_axis_rad < 0.0
        {
            return Err(SimError::InvalidArgument(
                "invalid directional antenna evaluation input".into(),
            ));
        }

        match self {
            Self::Isotropic => Ok(boresight_gain_dbi),
            Self::Gaussian {
                half_power_beamwidth_deg,
                floor_gain_dbi,
            } => {
                let hpbw = half_power_beamwidth_deg.to_radians();
                let loss = 3.0 * (2.0 * off_axis_rad / hpbw).powi(2);
                Ok((boresight_gain_dbi - loss).max(*floor_gain_dbi))
            }
            Self::EllipticalGaussian {
                horizontal_half_power_beamwidth_deg,
                vertical_half_power_beamwidth_deg,
                floor_gain_dbi,
            } => {
                let h = horizontal_half_power_beamwidth_deg.to_radians();
                let v = vertical_half_power_beamwidth_deg.to_radians();
                let loss = 3.0
                    * ((2.0 * horizontal_angle_rad / h).powi(2)
                        + (2.0 * vertical_angle_rad / v).powi(2));
                Ok((boresight_gain_dbi - loss).max(*floor_gain_dbi))
            }
            Self::TabulatedRelative { samples } => {
                let angle_deg = off_axis_rad.to_degrees();
                let relative = interpolate_relative_gain(samples, angle_deg);
                Ok(boresight_gain_dbi + relative)
            }
        }
    }
}

fn validate_gaussian(hpbw_deg: f64, floor_gain_dbi: f64, boresight_gain_dbi: f64) -> Result<()> {
    if !hpbw_deg.is_finite() || !floor_gain_dbi.is_finite() {
        return Err(SimError::NonFinite);
    }
    if hpbw_deg <= 0.0 || hpbw_deg > 360.0 || floor_gain_dbi > boresight_gain_dbi {
        return Err(SimError::InvalidArgument(
            "invalid Gaussian antenna-pattern parameters".into(),
        ));
    }
    Ok(())
}

fn interpolate_relative_gain(samples: &[(f64, f64)], angle_deg: f64) -> f64 {
    if angle_deg <= samples[0].0 {
        return samples[0].1;
    }
    if angle_deg >= samples[samples.len() - 1].0 {
        return samples[samples.len() - 1].1;
    }
    let upper = samples.partition_point(|sample| sample.0 < angle_deg);
    let a = samples[upper - 1];
    let b = samples[upper];
    let t = (angle_deg - a.0) / (b.0 - a.0);
    a.1 + (b.1 - a.1) * t
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
