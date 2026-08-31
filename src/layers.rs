use serde::{Deserialize, Serialize};

use crate::{Complex64, MaterialProperties, Polarization, Result, SimError, fresnel_coefficients, propagation_constant};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MediumLayer {
    pub thickness_m: Option<f64>,
    pub material: MaterialProperties,
}

impl MediumLayer {
    pub fn new(thickness_m: Option<f64>, material: MaterialProperties) -> Result<Self> {
        material.validate()?;
        if let Some(value) = thickness_m {
            if !value.is_finite() || value < 0.0 {
                return Err(SimError::InvalidArgument("layer thickness must be finite and non-negative".into()));
            }
        }
        Ok(Self { thickness_m, material })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MediumStack { pub layers: Vec<MediumLayer> }

impl MediumStack {
    pub fn new(layers: Vec<MediumLayer>) -> Result<Self> {
        if layers.is_empty() { return Err(SimError::InvalidArgument("medium stack requires at least one layer".into())); }
        Ok(Self { layers })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LayerPathResult {
    pub complex_gain: Complex64,
    pub attenuation_db: f64,
    pub phase_rad: f64,
}

pub fn propagate_stack(
    stack: &MediumStack,
    frequency_hz: f64,
    incidence_rad: f64,
    polarization: Polarization,
) -> Result<LayerPathResult> {
    if stack.layers.is_empty() { return Err(SimError::InvalidArgument("medium stack requires at least one layer".into())); }
    let mut gain = Complex64::new(1.0, 0.0);
    for (index, layer) in stack.layers.iter().enumerate() {
        if index > 0 {
            let previous = &stack.layers[index - 1];
            let interface = fresnel_coefficients(frequency_hz, &previous.material, &layer.material, incidence_rad, polarization)?;
            gain = gain.mul(interface.transmission);
        }
        if let Some(distance_m) = layer.thickness_m {
            if distance_m > 0.0 {
                let gamma = propagation_constant(frequency_hz, &layer.material)?;
                let propagation = Complex64::new(-gamma.alpha_np_per_m * distance_m, -gamma.beta_rad_per_m * distance_m).exp();
                gain = gain.mul(propagation);
            }
        }
    }
    let amplitude = gain.abs().max(f64::MIN_POSITIVE);
    Ok(LayerPathResult {
        complex_gain: gain,
        attenuation_db: -20.0 * amplitude.log10(),
        phase_rad: gain.arg(),
    })
}
