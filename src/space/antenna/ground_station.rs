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

/// Inverse of [`ecef_delta_to_enu`]: rotates a local ENU displacement into
/// ECEF without introducing a flat-Earth distance approximation.
pub fn enu_delta_to_ecef(delta_enu: [f64; 3], origin: GeodeticPosition) -> [f64; 3] {
    let (sin_lat, cos_lat) = origin.latitude_rad.sin_cos();
    let (sin_lon, cos_lon) = origin.longitude_rad.sin_cos();
    let east = delta_enu[0];
    let north = delta_enu[1];
    let up = delta_enu[2];
    [
        -sin_lon * east - sin_lat * cos_lon * north + cos_lat * cos_lon * up,
        cos_lon * east - sin_lat * sin_lon * north + cos_lat * sin_lon * up,
        cos_lat * north + sin_lat * up,
    ]
}

/// Converts WGS-84 ECEF coordinates to geodetic latitude, longitude and height.
/// Uses Bowring's closed-form auxiliary latitude, which is stable for terrestrial
/// and near-Earth positions including the polar axis.
pub fn ecef_to_geodetic(ecef_m: [f64; 3]) -> Result<GeodeticPosition> {
    if ecef_m.iter().any(|value| !value.is_finite()) {
        return Err(SimError::NonFinite);
    }
    const A: f64 = 6_378_137.0;
    const F: f64 = 1.0 / 298.257_223_563;
    const B: f64 = A * (1.0 - F);
    let e2 = F * (2.0 - F);
    let ep2 = (A * A - B * B) / (B * B);
    let x = ecef_m[0];
    let y = ecef_m[1];
    let z = ecef_m[2];
    let p = x.hypot(y);
    if p <= f64::EPSILON {
        if z.abs() <= f64::EPSILON {
            return Err(SimError::InvalidArgument(
                "ECEF origin has no geodetic latitude".into(),
            ));
        }
        return Ok(GeodeticPosition {
            latitude_rad: z.signum() * std::f64::consts::FRAC_PI_2,
            longitude_rad: 0.0,
            height_m: z.abs() - B,
        });
    }
    let longitude_rad = y.atan2(x);
    let theta = (z * A).atan2(p * B);
    let (sin_theta, cos_theta) = theta.sin_cos();
    let latitude_rad = (z + ep2 * B * sin_theta.powi(3)).atan2(p - e2 * A * cos_theta.powi(3));
    let (sin_lat, cos_lat) = latitude_rad.sin_cos();
    let n = A / (1.0 - e2 * sin_lat * sin_lat).sqrt();
    let height_m = if cos_lat.abs() > 1e-12 {
        p / cos_lat - n
    } else {
        z.abs() / sin_lat.abs() - n * (1.0 - e2)
    };
    Ok(GeodeticPosition {
        latitude_rad,
        longitude_rad,
        height_m,
    })
}
