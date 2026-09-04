use serde::{Deserialize, Serialize};

use crate::materials::Complex64;
use crate::{propagation_constant, MaterialProperties, Result, SimError};

/// Field polarization relative to the plane of incidence.
///
/// * [`Polarization::Te`] (transverse electric, s/perpendicular): **E** is
///   perpendicular to the plane of incidence.
/// * [`Polarization::Tm`] (transverse magnetic, p/parallel): **H** is
///   perpendicular to the plane of incidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Polarization {
    Te,
    Tm,
}

/// Fresnel electric-field coefficients at a planar interface.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FresnelCoefficients {
    pub reflection: Complex64,
    pub transmission: Complex64,
    pub transmitted_angle_rad: Complex64,
}

pub fn fresnel_coefficients(
    frequency_hz: f64,
    medium1: &MaterialProperties,
    medium2: &MaterialProperties,
    incidence_rad: f64,
    polarization: Polarization,
) -> Result<FresnelCoefficients> {
    if !incidence_rad.is_finite() || incidence_rad.abs() >= std::f64::consts::FRAC_PI_2 {
        return Err(SimError::InvalidArgument(
            "incidence angle must be finite and within (-pi/2, pi/2)".into(),
        ));
    }

    let eta1 = propagation_constant(frequency_hz, medium1)?.intrinsic_impedance_ohm;
    let eta2 = propagation_constant(frequency_hz, medium2)?.intrinsic_impedance_ohm;
    let n1 = medium1
        .relative_permittivity
        .scale(medium1.relative_permeability)
        .sqrt();
    let n2 = medium2
        .relative_permittivity
        .scale(medium2.relative_permeability)
        .sqrt();

    let sin_i = incidence_rad.sin();
    let sin_t = (n1 / n2).scale(sin_i);
    let cos_t = (Complex64::new(1.0, 0.0) - sin_t * sin_t).sqrt();
    let cos_i = incidence_rad.cos();

    let (reflection, transmission) = match polarization {
        Polarization::Te => {
            // TE: Gamma = (eta2 cos(i) - eta1 cos(t)) /
            //             (eta2 cos(i) + eta1 cos(t))
            let numerator = eta2.scale(cos_i) - eta1 * cos_t;
            let denominator = eta2.scale(cos_i) + eta1 * cos_t;
            if denominator.abs() <= f64::MIN_POSITIVE {
                return Err(SimError::InvalidArgument(
                    "degenerate TE Fresnel denominator".into(),
                ));
            }
            let reflection = numerator / denominator;
            let transmission = Complex64::new(1.0, 0.0) + reflection;
            (reflection, transmission)
        }
        Polarization::Tm => {
            // TM: Gamma = (eta2 cos(t) - eta1 cos(i)) /
            //             (eta2 cos(t) + eta1 cos(i))
            let numerator = eta2 * cos_t - eta1.scale(cos_i);
            let denominator = eta2 * cos_t + eta1.scale(cos_i);
            if denominator.abs() <= f64::MIN_POSITIVE || cos_t.abs() <= f64::MIN_POSITIVE {
                return Err(SimError::InvalidArgument(
                    "degenerate TM Fresnel geometry".into(),
                ));
            }
            let reflection = numerator / denominator;
            // Tangential-E continuity for the total E-field magnitude.
            let transmission = (Complex64::new(1.0, 0.0) + reflection).scale(cos_i) / cos_t;
            (reflection, transmission)
        }
    };

    Ok(FresnelCoefficients {
        reflection,
        transmission,
        transmitted_angle_rad: complex_asin(sin_t)?,
    })
}

fn complex_asin(z: Complex64) -> Result<Complex64> {
    // asin(z) = -i ln(iz + sqrt(1-z^2))
    let i_z = Complex64::new(-z.im, z.re);
    let root = (Complex64::new(1.0, 0.0) - z * z).sqrt();
    let value = i_z + root;
    let magnitude = value.abs();
    if !magnitude.is_finite() || magnitude <= f64::MIN_POSITIVE {
        return Err(SimError::InvalidArgument(
            "complex asin reached a logarithmic singularity".into(),
        ));
    }
    let ln = Complex64::new(magnitude.ln(), value.arg());
    Ok(Complex64::new(ln.im, -ln.re))
}
