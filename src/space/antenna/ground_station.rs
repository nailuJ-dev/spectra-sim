use serde::{Deserialize, Serialize};

use crate::{Result, SimError};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GeodeticPosition {
    pub latitude_rad: f64,
    pub longitude_rad: f64,
    pub height_m: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GroundStation {
    pub name: String,
    pub position: GeodeticPosition,
    pub minimum_elevation_rad: f64,
}

impl GroundStation {
    pub fn new(
        name: impl Into<String>,
        position: GeodeticPosition,
        minimum_elevation_rad: f64,
    ) -> Result<Self> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(SimError::InvalidArgument(
                "ground station name is empty".into(),
            ));
        }
        if [
            position.latitude_rad,
            position.longitude_rad,
            position.height_m,
            minimum_elevation_rad,
        ]
        .iter()
        .any(|v| !v.is_finite())
        {
            return Err(SimError::NonFinite);
        }
        if !minimum_elevation_rad.is_finite()
            || !(-std::f64::consts::FRAC_PI_2..=std::f64::consts::FRAC_PI_2)
                .contains(&minimum_elevation_rad)
        {
            return Err(SimError::InvalidArgument(
                "invalid ground station coordinates".into(),
            ));
        }
        Ok(Self {
            name,
            position,
            minimum_elevation_rad,
        })
    }

    pub fn ecef_m(&self) -> [f64; 3] {
        geodetic_to_ecef(self.position)
    }
}

pub fn geodetic_to_ecef(position: GeodeticPosition) -> [f64; 3] {
    const A: f64 = 6_378_137.0;
    const F: f64 = 1.0 / 298.257_223_563;
    let e2 = F * (2.0 - F);
    let (sin_lat, cos_lat) = position.latitude_rad.sin_cos();
    let (sin_lon, cos_lon) = position.longitude_rad.sin_cos();
    let n = A / (1.0 - e2 * sin_lat * sin_lat).sqrt();
    [
        (n + position.height_m) * cos_lat * cos_lon,
        (n + position.height_m) * cos_lat * sin_lon,
        (n * (1.0 - e2) + position.height_m) * sin_lat,
    ]
}

pub fn ecef_delta_to_enu(delta_ecef: [f64; 3], station: GeodeticPosition) -> [f64; 3] {
    let (sin_lat, cos_lat) = station.latitude_rad.sin_cos();
    let (sin_lon, cos_lon) = station.longitude_rad.sin_cos();
    [
        -sin_lon * delta_ecef[0] + cos_lon * delta_ecef[1],
        -sin_lat * cos_lon * delta_ecef[0] - sin_lat * sin_lon * delta_ecef[1]
            + cos_lat * delta_ecef[2],
        cos_lat * cos_lon * delta_ecef[0]
            + cos_lat * sin_lon * delta_ecef[1]
            + sin_lat * delta_ecef[2],
    ]
}
