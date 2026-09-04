use std::ops::{Add, Div, Mul, Sub};

use serde::{Deserialize, Serialize};

use crate::{Result, SimError};

pub const SPEED_OF_LIGHT_MPS: f64 = 299_792_458.0;
pub const BOLTZMANN_J_PER_K: f64 = 1.380_649e-23;
pub const REFERENCE_IMPEDANCE_OHM: f64 = 50.0;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vec3 {
    pub fn new(x: f64, y: f64, z: f64) -> Result<Self> {
        let value = Self { x, y, z };
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> Result<()> {
        if self.x.is_finite() && self.y.is_finite() && self.z.is_finite() {
            Ok(())
        } else {
            Err(SimError::NonFinite)
        }
    }

    pub fn add_vec(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
        }
    }

    pub fn sub_vec(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
        }
    }

    pub fn scale(self, scalar: f64) -> Self {
        Self {
            x: self.x * scalar,
            y: self.y * scalar,
            z: self.z * scalar,
        }
    }

    pub fn dot(self, other: Self) -> f64 {
        self.x
            .mul_add(other.x, self.y.mul_add(other.y, self.z * other.z))
    }

    pub fn cross(self, other: Self) -> Self {
        Self {
            x: self.y.mul_add(other.z, -(self.z * other.y)),
            y: self.z.mul_add(other.x, -(self.x * other.z)),
            z: self.x.mul_add(other.y, -(self.y * other.x)),
        }
    }

    pub fn norm(self) -> f64 {
        self.dot(self).max(0.0).sqrt()
    }

    pub fn distance(self, other: Self) -> f64 {
        self.sub_vec(other).norm()
    }

    pub fn normalized(self) -> Result<Self> {
        let norm = self.norm();
        if norm <= 1e-12 {
            return Err(SimError::InvalidArgument(
                "cannot normalize a near-zero vector".into(),
            ));
        }
        Ok(self.scale(1.0 / norm))
    }
}

/// Shared complex-number type used by RF, propagation, interfaces and materials.
///
/// Keeping one type across the crate avoids API-incompatible complex values in
/// otherwise composable public structures.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Complex64 {
    pub re: f64,
    pub im: f64,
}

impl Complex64 {
    pub const ZERO: Self = Self { re: 0.0, im: 0.0 };

    pub const fn new(re: f64, im: f64) -> Self {
        Self { re, im }
    }

    pub fn from_polar(amplitude: f64, phase_rad: f64) -> Self {
        let (sin, cos) = phase_rad.sin_cos();
        Self {
            re: amplitude * cos,
            im: amplitude * sin,
        }
    }

    pub fn abs(self) -> f64 {
        self.re.hypot(self.im)
    }

    pub fn arg(self) -> f64 {
        self.im.atan2(self.re)
    }

    pub fn conj(self) -> Self {
        Self::new(self.re, -self.im)
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

    pub fn add_complex(self, other: Self) -> Self {
        self + other
    }

    pub fn mul_complex(self, other: Self) -> Self {
        self * other
    }

    pub fn scale(self, scalar: f64) -> Self {
        Self {
            re: self.re * scalar,
            im: self.im * scalar,
        }
    }

    pub fn magnitude_squared(self) -> f64 {
        self.re.mul_add(self.re, self.im * self.im)
    }

    pub fn phase(self) -> f64 {
        self.arg()
    }
}

impl Add for Complex64 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.re + rhs.re, self.im + rhs.im)
    }
}

impl Sub for Complex64 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.re - rhs.re, self.im - rhs.im)
    }
}

impl Mul for Complex64 {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self::new(
            self.re.mul_add(rhs.re, -(self.im * rhs.im)),
            self.re.mul_add(rhs.im, self.im * rhs.re),
        )
    }
}

impl Div for Complex64 {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        let denominator = rhs.re.mul_add(rhs.re, rhs.im * rhs.im);
        Self::new(
            self.re.mul_add(rhs.re, self.im * rhs.im) / denominator,
            self.im.mul_add(rhs.re, -(self.re * rhs.im)) / denominator,
        )
    }
}

pub fn db_to_linear(db: f64) -> f64 {
    10.0_f64.powf(db / 10.0)
}

pub fn linear_to_db(value: f64) -> f64 {
    10.0 * value.max(1e-300).log10()
}

pub fn dbm_to_watts(dbm: f64) -> f64 {
    10.0_f64.powf((dbm - 30.0) / 10.0)
}

pub fn watts_to_dbm(watts: f64) -> f64 {
    10.0 * watts.max(1e-300).log10() + 30.0
}

pub fn wavelength_m(frequency_hz: f64) -> Result<f64> {
    if !frequency_hz.is_finite() || frequency_hz <= 0.0 {
        return Err(SimError::InvalidArgument(
            "frequency_hz must be finite and positive".into(),
        ));
    }
    Ok(SPEED_OF_LIGHT_MPS / frequency_hz)
}
