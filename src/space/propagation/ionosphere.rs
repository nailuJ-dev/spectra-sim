use serde::{Deserialize, Serialize};

use crate::{Result, SimError};

const IONO_COEFFICIENT_M_HZ2_PER_ELECTRON_M2: f64 = 40.3;
const SPEED_OF_LIGHT_M_PER_S: f64 = 299_792_458.0;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct IonosphericCorrection {
    pub group_delay_m: f64,
    pub group_delay_s: f64,
    pub phase_advance_m: f64,
    pub faraday_rotation_rad: Option<f64>,
}

/// First-order cold-plasma ionospheric correction using slant TEC in TECU
/// (1 TECU = 1e16 electrons/m²). Group delay scales as f^-2.
pub fn ionospheric_first_order(
    frequency_hz: f64,
    slant_tec_tecu: f64,
    faraday_rotation_rad: Option<f64>,
) -> Result<IonosphericCorrection> {
    if !frequency_hz.is_finite()
        || !slant_tec_tecu.is_finite()
        || faraday_rotation_rad.is_some_and(|v| !v.is_finite())
    {
        return Err(SimError::NonFinite);
    }
    if frequency_hz <= 0.0 || slant_tec_tecu < 0.0 {
        return Err(SimError::InvalidArgument(
            "invalid ionospheric input".into(),
        ));
    }
    let electrons_per_m2 = slant_tec_tecu * 1.0e16;
    let delay_m = IONO_COEFFICIENT_M_HZ2_PER_ELECTRON_M2 * electrons_per_m2 / frequency_hz.powi(2);
    Ok(IonosphericCorrection {
        group_delay_m: delay_m,
        group_delay_s: delay_m / SPEED_OF_LIGHT_M_PER_S,
        phase_advance_m: -delay_m,
        faraday_rotation_rad,
    })
}

/// Computes Faraday rotation from an externally integrated line-of-sight
/// magnetic-field/TEC product. `integral_b_parallel_ne` is in T·electrons/m².
pub fn faraday_rotation_from_integral(
    frequency_hz: f64,
    integral_b_parallel_ne: f64,
) -> Result<f64> {
    if !frequency_hz.is_finite() || !integral_b_parallel_ne.is_finite() {
        return Err(SimError::NonFinite);
    }
    if frequency_hz <= 0.0 {
        return Err(SimError::InvalidArgument(
            "frequency must be positive".into(),
        ));
    }
    // SI cold-plasma constant, radians when B is tesla and Ne is electrons/m².
    Ok(2.36e4 * integral_b_parallel_ne / frequency_hz.powi(2))
}
