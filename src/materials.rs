use serde::{Deserialize, Serialize};

use crate::{Result, SimError, ValidityLedger};

pub const VACUUM_PERMITTIVITY_F_PER_M: f64 = 8.854_187_812_8e-12;
pub const VACUUM_PERMEABILITY_H_PER_M: f64 = 1.256_637_062_12e-6;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Complex64 {
    pub re: f64,
    pub im: f64,
}

impl Complex64 {
    pub const fn new(re: f64, im: f64) -> Self { Self { re, im } }
    pub fn abs(self) -> f64 { self.re.hypot(self.im) }
    pub fn arg(self) -> f64 { self.im.atan2(self.re) }
    pub fn conj(self) -> Self { Self::new(self.re, -self.im) }
    pub fn add(self, rhs: Self) -> Self { Self::new(self.re + rhs.re, self.im + rhs.im) }
    pub fn sub(self, rhs: Self) -> Self { Self::new(self.re - rhs.re, self.im - rhs.im) }
    pub fn mul(self, rhs: Self) -> Self {
        Self::new(self.re.mul_add(rhs.re, -(self.im * rhs.im)), self.re.mul_add(rhs.im, self.im * rhs.re))
    }
    pub fn scale(self, value: f64) -> Self { Self::new(self.re * value, self.im * value) }
    pub fn div(self, rhs: Self) -> Self {
        let denominator = rhs.re.mul_add(rhs.re, rhs.im * rhs.im);
        Self::new(
            (self.re.mul_add(rhs.re, self.im * rhs.im)) / denominator,
            (self.im.mul_add(rhs.re, -(self.re * rhs.im))) / denominator,
        )
    }
    pub fn sqrt(self) -> Self {
        let magnitude = self.abs();
        let re = ((magnitude + self.re) * 0.5).max(0.0).sqrt();
        let im_mag = ((magnitude - self.re) * 0.5).max(0.0).sqrt();
        Self::new(re, if self.im < 0.0 { -im_mag } else { im_mag })
    }
    pub fn exp(self) -> Self {
        let amplitude = self.re.exp();
        Self::new(amplitude * self.im.cos(), amplitude * self.im.sin())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaterialProperties {
    pub relative_permittivity: Complex64,
    pub relative_permeability: f64,
    pub conductivity_s_per_m: f64,
    pub validity: ValidityLedger,
}

impl MaterialProperties {
    pub fn validate(&self) -> Result<()> {
        if !self.relative_permittivity.re.is_finite()
            || !self.relative_permittivity.im.is_finite()
            || !self.relative_permeability.is_finite()
            || !self.conductivity_s_per_m.is_finite()
            || self.relative_permittivity.re <= 0.0
            || self.relative_permeability <= 0.0
            || self.conductivity_s_per_m < 0.0
        {
            return Err(SimError::InvalidArgument("material properties must be finite and physical".into()));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PropagationConstant {
    pub alpha_np_per_m: f64,
    pub beta_rad_per_m: f64,
    pub wavelength_m: f64,
    pub phase_velocity_m_per_s: f64,
    pub skin_depth_m: f64,
    pub intrinsic_impedance_ohm: Complex64,
}

pub fn propagation_constant(frequency_hz: f64, material: &MaterialProperties) -> Result<PropagationConstant> {
    material.validate()?;
    if !frequency_hz.is_finite() || frequency_hz <= 0.0 {
        return Err(SimError::InvalidArgument("frequency_hz must be finite and positive".into()));
    }
    let omega = std::f64::consts::TAU * frequency_hz;
    let mu = VACUUM_PERMEABILITY_H_PER_M * material.relative_permeability;
    let epsilon = material.relative_permittivity.scale(VACUUM_PERMITTIVITY_F_PER_M);
    let sigma_plus_jwe = Complex64::new(material.conductivity_s_per_m - omega * epsilon.im, omega * epsilon.re);
    let jwm = Complex64::new(0.0, omega * mu);
    let gamma = jwm.mul(sigma_plus_jwe).sqrt();
    let alpha = gamma.re.abs();
    let beta = gamma.im.abs();
    let wavelength = if beta > f64::EPSILON { std::f64::consts::TAU / beta } else { f64::INFINITY };
    let phase_velocity = if beta > f64::EPSILON { omega / beta } else { f64::INFINITY };
    let skin_depth = if alpha > f64::EPSILON { 1.0 / alpha } else { f64::INFINITY };
    let impedance = jwm.div(sigma_plus_jwe).sqrt();
    Ok(PropagationConstant {
        alpha_np_per_m: alpha,
        beta_rad_per_m: beta,
        wavelength_m: wavelength,
        phase_velocity_m_per_s: phase_velocity,
        skin_depth_m: skin_depth,
        intrinsic_impedance_ohm: impedance,
    })
}
