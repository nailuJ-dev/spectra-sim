use serde::{Deserialize, Serialize};

use crate::{Result, SimError};

use super::{
    integrate_gaseous_attenuation_p676_13, ionospheric_first_order, AtmosphericLayer,
    IonosphericCorrection,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EarthSpaceEnvironment {
    pub atmospheric_layers: Vec<AtmosphericLayer>,
    pub rain_attenuation_db: Option<f64>,
    pub cloud_attenuation_db: Option<f64>,
    pub tropospheric_scintillation_db: Option<f64>,
    pub slant_tec_tecu: Option<f64>,
    pub faraday_rotation_rad: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EarthSpaceCorrections {
    pub gaseous_attenuation_db: f64,
    pub rain_attenuation_db: f64,
    pub cloud_attenuation_db: f64,
    pub tropospheric_scintillation_db: f64,
    pub ionosphere: Option<IonosphericCorrection>,
}

impl EarthSpaceCorrections {
    pub fn total_attenuation_db(&self) -> f64 {
        self.gaseous_attenuation_db
            + self.rain_attenuation_db
            + self.cloud_attenuation_db
            + self.tropospheric_scintillation_db
    }
}

pub fn earth_space_corrections(
    frequency_hz: f64,
    environment: &EarthSpaceEnvironment,
) -> Result<EarthSpaceCorrections> {
    if !frequency_hz.is_finite() || frequency_hz <= 0.0 {
        return Err(SimError::InvalidArgument(
            "frequency must be positive".into(),
        ));
    }
    let validated_loss = |value: Option<f64>, label: &str| -> Result<f64> {
        match value {
            None => Ok(0.0),
            Some(v) if v.is_finite() && v >= 0.0 => Ok(v),
            Some(_) => Err(SimError::InvalidArgument(format!("invalid {label}"))),
        }
    };
    let gaseous = integrate_gaseous_attenuation_p676_13(
        frequency_hz / 1.0e9,
        &environment.atmospheric_layers,
    )?;
    let ionosphere = environment
        .slant_tec_tecu
        .map(|tec| ionospheric_first_order(frequency_hz, tec, environment.faraday_rotation_rad))
        .transpose()?;
    Ok(EarthSpaceCorrections {
        gaseous_attenuation_db: gaseous,
        rain_attenuation_db: validated_loss(environment.rain_attenuation_db, "rain attenuation")?,
        cloud_attenuation_db: validated_loss(
            environment.cloud_attenuation_db,
            "cloud attenuation",
        )?,
        tropospheric_scintillation_db: validated_loss(
            environment.tropospheric_scintillation_db,
            "tropospheric scintillation",
        )?,
        ionosphere,
    })
}
