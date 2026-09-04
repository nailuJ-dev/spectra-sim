use serde::{Deserialize, Serialize};

use crate::{Antenna, KinematicState, Result, SimError, Vec3};

use super::{ecef_delta_to_enu, GroundStation};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AzElRange {
    pub azimuth_rad: f64,
    pub elevation_rad: f64,
    pub range_m: f64,
    pub visible: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BodyDirection {
    pub off_axis_rad: f64,
    pub horizontal_angle_rad: f64,
    pub vertical_angle_rad: f64,
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

/// Expresses a line of sight in the platform body frame.
///
/// Terrestrial attitude convention:
/// - ENU world axes: +x East, +y North, +z Up;
/// - yaw is heading clockwise from North;
/// - pitch is positive nose-up;
/// - roll is positive right-wing-down;
/// - antenna boresight is body +X/forward.
pub fn target_direction_in_body(
    state: KinematicState,
    target_position_enu_m: Vec3,
) -> Result<BodyDirection> {
    state.validate()?;
    target_position_enu_m.validate()?;
    let los = target_position_enu_m
        .sub_vec(state.position_enu_m)
        .normalized()?;

    let yaw = state.yaw_deg.to_radians();
    let pitch = state.pitch_deg.to_radians();
    let roll = state.roll_deg.to_radians();
    let (sin_yaw, cos_yaw) = yaw.sin_cos();
    let (sin_pitch, cos_pitch) = pitch.sin_cos();
    let (sin_roll, cos_roll) = roll.sin_cos();

    let forward = Vec3 {
        x: sin_yaw * cos_pitch,
        y: cos_yaw * cos_pitch,
        z: sin_pitch,
    };
    let right_zero = Vec3 {
        x: cos_yaw,
        y: -sin_yaw,
        z: 0.0,
    };
    let up_zero = right_zero.cross(forward).normalized()?;

    // Positive aerospace roll rotates the right axis downward.
    let right = right_zero.scale(cos_roll).sub_vec(up_zero.scale(sin_roll));
    let up = up_zero.scale(cos_roll).add_vec(right_zero.scale(sin_roll));

    let forward_component = los.dot(forward);
    let right_component = los.dot(right);
    let up_component = los.dot(up);
    let off_axis = forward_component.clamp(-1.0, 1.0).acos();
    let horizontal = right_component.atan2(forward_component);
    let vertical = up_component.atan2(forward_component);

    Ok(BodyDirection {
        off_axis_rad: off_axis,
        horizontal_angle_rad: horizontal,
        vertical_angle_rad: vertical,
    })
}

pub fn effective_directional_gain_dbi(
    antenna: &Antenna,
    state: KinematicState,
    target_position_enu_m: Vec3,
    frequency_hz: f64,
) -> Result<f64> {
    antenna.validate()?;
    let direction = target_direction_in_body(state, target_position_enu_m)?;
    antenna.pattern.gain_dbi(
        antenna.gain_dbi,
        direction.off_axis_rad,
        direction.horizontal_angle_rad,
        direction.vertical_angle_rad,
        frequency_hz,
    )
}

pub fn configured_directional_gain_dbi(
    boresight_gain_dbi: f64,
    pattern: &super::AntennaPatternModel,
    state: KinematicState,
    target_position_enu_m: Vec3,
    frequency_hz: f64,
) -> Result<f64> {
    let direction = target_direction_in_body(state, target_position_enu_m)?;
    pattern.gain_dbi(
        boresight_gain_dbi,
        direction.off_axis_rad,
        direction.horizontal_angle_rad,
        direction.vertical_angle_rad,
        frequency_hz,
    )
}
