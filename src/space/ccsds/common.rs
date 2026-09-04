use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{ReferenceFrame, Result, SimError, TimeScale};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CcsdsKeyValue {
    pub key: String,
    pub value: String,
    pub unit: Option<String>,
}

impl CcsdsKeyValue {
    pub fn parse(key: &str, raw: &str) -> Self {
        let raw = raw.trim();
        if let Some(start) = raw.rfind('[') {
            if raw.ends_with(']') && start > 0 {
                return Self {
                    key: key.trim().to_ascii_uppercase(),
                    value: raw[..start].trim().to_string(),
                    unit: Some(raw[start + 1..raw.len() - 1].trim().to_string()),
                };
            }
        }
        Self {
            key: key.trim().to_ascii_uppercase(),
            value: raw.to_string(),
            unit: None,
        }
    }
}

pub fn parse_time_scale(value: &str) -> Result<TimeScale> {
    match value.trim().to_ascii_uppercase().as_str() {
        "UTC" => Ok(TimeScale::Utc),
        "TAI" => Ok(TimeScale::Tai),
        "TT" | "TDT" => Ok(TimeScale::Tt),
        "UT1" => Ok(TimeScale::Ut1),
        "GPS" => Ok(TimeScale::Gps),
        other => Err(SimError::InvalidArgument(format!(
            "unsupported CCSDS time system: {other}"
        ))),
    }
}

pub fn parse_reference_frame(value: &str) -> Result<ReferenceFrame> {
    let normalized = value.trim().to_ascii_uppercase().replace('-', "");
    match normalized.as_str() {
        "ICRF" => Ok(ReferenceFrame::Icrf),
        "GCRF" => Ok(ReferenceFrame::Gcrf),
        "EME2000" | "J2000" => Ok(ReferenceFrame::Eme2000),
        "TEME" => Ok(ReferenceFrame::Teme),
        "ITRF" | "ITRF2000" | "ITRF2008" | "ITRF2014" | "ITRF2020" => Ok(ReferenceFrame::Itrf),
        other => Err(SimError::InvalidArgument(format!(
            "unsupported CCSDS reference frame: {other}"
        ))),
    }
}

pub fn parse_f64(value: &str, field: &str) -> Result<f64> {
    let v = value
        .trim()
        .parse::<f64>()
        .map_err(|_| SimError::InvalidArgument(format!("CCSDS field {field} must be numeric")))?;
    if !v.is_finite() {
        return Err(SimError::NonFinite);
    }
    Ok(v)
}

pub fn get_required<'a>(map: &'a BTreeMap<String, String>, key: &str) -> Result<&'a str> {
    map.get(key)
        .map(String::as_str)
        .ok_or_else(|| SimError::InvalidArgument(format!("missing CCSDS field {key}")))
}

pub fn convert_distance_to_m(value: f64, unit: Option<&str>) -> Result<f64> {
    match unit.map(str::trim).map(str::to_ascii_lowercase).as_deref() {
        None | Some("km") => Ok(value * 1_000.0),
        Some("m") => Ok(value),
        Some(other) => Err(SimError::InvalidArgument(format!(
            "unsupported CCSDS distance unit {other}"
        ))),
    }
}

pub fn convert_velocity_to_m_per_s(value: f64, unit: Option<&str>) -> Result<f64> {
    match unit
        .map(str::trim)
        .map(|v| v.to_ascii_lowercase().replace(' ', ""))
        .as_deref()
    {
        None | Some("km/s") | Some("km.s-1") | Some("km*s^-1") => Ok(value * 1_000.0),
        Some("m/s") | Some("m.s-1") | Some("m*s^-1") => Ok(value),
        Some(other) => Err(SimError::InvalidArgument(format!(
            "unsupported CCSDS velocity unit {other}"
        ))),
    }
}


pub fn convert_acceleration_to_m_per_s2(value: f64, unit: Option<&str>) -> Result<f64> {
    match unit
        .map(str::trim)
        .map(|v| v.to_ascii_lowercase().replace(' ', ""))
        .as_deref()
    {
        None | Some("km/s^2") | Some("km/s2") | Some("km.s-2") | Some("km*s^-2") => {
            Ok(value * 1_000.0)
        }
        Some("m/s^2") | Some("m/s2") | Some("m.s-2") | Some("m*s^-2") => Ok(value),
        Some(other) => Err(SimError::InvalidArgument(format!(
            "unsupported CCSDS acceleration unit {other}"
        ))),
    }
}
