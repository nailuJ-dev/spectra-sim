use serde::{Deserialize, Serialize};

use crate::{Result, SimError};

use super::{ecef_delta_to_enu, GroundStation};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AzElRange {
    pub azimuth_rad: f64,
    pub elevation_rad: f64,
    pub range_m: f64,
    pub visible: bool,
}

pub fn ground_station_az_el_range(
    station: &GroundStation,
    target_itrf_m: [f64; 3],
) -> Result<AzElRange> {
    if target_itrf_m.iter().any(|v| !v.is_finite()) {
        return Err(SimError::NonFinite);
    }
    let origin = station.ecef_m();
    let delta = [
        target_itrf_m[0] - origin[0],
        target_itrf_m[1] - origin[1],
        target_itrf_m[2] - origin[2],
    ];
    let enu = ecef_delta_to_enu(delta, station.position);
    let range = (enu[0] * enu[0] + enu[1] * enu[1] + enu[2] * enu[2]).sqrt();
    if range <= f64::EPSILON {
        return Err(SimError::InvalidArgument(
            "target coincides with ground station".into(),
        ));
    }
    let horizontal = (enu[0] * enu[0] + enu[1] * enu[1]).sqrt();
    let mut az = enu[0].atan2(enu[1]);
    if az < 0.0 {
        az += 2.0 * std::f64::consts::PI;
    }
    let el = enu[2].atan2(horizontal);
    Ok(AzElRange {
        azimuth_rad: az,
        elevation_rad: el,
        range_m: range,
        visible: el >= station.minimum_elevation_rad,
    })
}

pub fn off_axis_angle_rad(boresight_unit: [f64; 3], target_unit: [f64; 3]) -> Result<f64> {
    let norm = |v: [f64; 3]| (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    let a = norm(boresight_unit);
    let b = norm(target_unit);
    if !a.is_finite() || !b.is_finite() || a <= f64::EPSILON || b <= f64::EPSILON {
        return Err(SimError::InvalidArgument(
            "pointing vectors must be finite and non-zero".into(),
        ));
    }
    let dot = (boresight_unit[0] * target_unit[0]
        + boresight_unit[1] * target_unit[1]
        + boresight_unit[2] * target_unit[2])
        / (a * b);
    Ok(dot.clamp(-1.0, 1.0).acos())
}
