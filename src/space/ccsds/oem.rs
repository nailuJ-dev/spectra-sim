use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{AbsoluteEpoch, AbsoluteOrbitState, ReferenceFrame, Result, SimError};

use super::{
    convert_acceleration_to_m_per_s2, convert_distance_to_m, convert_velocity_to_m_per_s,
    get_required, parse_f64, parse_reference_frame, parse_time_scale, parse_xml_tree, XmlNode,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OemCovariance {
    pub epoch: AbsoluteEpoch,
    pub frame: ReferenceFrame,
    pub lower_triangular_si: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OemSegment {
    pub metadata: BTreeMap<String, String>,
    pub states: Vec<AbsoluteOrbitState>,
    pub covariances: Vec<OemCovariance>,
    pub extensions: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OemMessage {
    pub header: BTreeMap<String, String>,
    pub segments: Vec<OemSegment>,
}

type RawOemState = (String, [f64; 6], Option<[f64; 3]>);
type RawOemCovariance = (String, String, Vec<f64>);

impl OemMessage {
    pub fn from_kvn(input: &str) -> Result<Self> {
        if input.len() > 32 * 1024 * 1024 {
            return Err(SimError::DimensionLimit {
                actual: input.len(),
                max: 32 * 1024 * 1024,
            });
        }
        let mut header = BTreeMap::new();
        let mut segments = Vec::new();
        let mut metadata = BTreeMap::new();
        let mut states_raw: Vec<RawOemState> = Vec::new();
        let mut covariance_raw: Vec<RawOemCovariance> = Vec::new();
        let mut covariance_epoch: Option<String> = None;
        let mut covariance_frame: Option<String> = None;
        let mut covariance_numbers: Vec<f64> = Vec::new();
        let mut in_meta = false;
        let mut in_covariance = false;
        let mut seen_segment_data = false;

        let flush_segment = |metadata: &mut BTreeMap<String, String>,
                             states_raw: &mut Vec<RawOemState>,
                             covariance_raw: &mut Vec<RawOemCovariance>,
                             segments: &mut Vec<OemSegment>|
         -> Result<()> {
            if metadata.is_empty() && states_raw.is_empty() && covariance_raw.is_empty() {
                return Ok(());
            }
            segments.push(build_segment(
                metadata.clone(),
                states_raw.clone(),
                covariance_raw.clone(),
            )?);
            metadata.clear();
            states_raw.clear();
            covariance_raw.clear();
            Ok(())
        };

        for raw in input.lines() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with("COMMENT") {
                continue;
            }
            match line {
                "META_START" => {
                    if seen_segment_data {
                        flush_segment(
                            &mut metadata,
                            &mut states_raw,
                            &mut covariance_raw,
                            &mut segments,
                        )?;
                        seen_segment_data = false;
                    }
                    in_meta = true;
                    continue;
                }
                "META_STOP" => {
                    in_meta = false;
                    continue;
                }
                "COVARIANCE_START" => {
                    in_covariance = true;
                    covariance_epoch = None;
                    covariance_frame = None;
                    covariance_numbers.clear();
                    continue;
                }
                "COVARIANCE_STOP" => {
                    in_covariance = false;
                    let epoch = covariance_epoch.take().ok_or_else(|| {
                        SimError::InvalidArgument("OEM covariance missing EPOCH".into())
                    })?;
                    let frame = covariance_frame
                        .take()
                        .unwrap_or_else(|| metadata.get("REF_FRAME").cloned().unwrap_or_default());
                    if covariance_numbers.len() != 21 {
                        return Err(SimError::InvalidArgument(format!(
                            "OEM covariance requires 21 lower-triangular values, got {}",
                            covariance_numbers.len()
                        )));
                    }
                    covariance_raw.push((epoch, frame, covariance_numbers.clone()));
                    seen_segment_data = true;
                    continue;
                }
                _ => {}
            }
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim().to_ascii_uppercase();
                let value = value.trim().to_string();
                if in_covariance {
                    match key.as_str() {
                        "EPOCH" => covariance_epoch = Some(value),
                        "COV_REF_FRAME" => covariance_frame = Some(value),
                        _ => {}
                    }
                } else if in_meta {
                    metadata.insert(key, strip_unit(&value).0);
                } else if segments.is_empty() && metadata.is_empty() && states_raw.is_empty() {
                    header.insert(key, strip_unit(&value).0);
                }
                continue;
            }
            if in_covariance {
                for token in line.split_whitespace() {
                    covariance_numbers.push(parse_f64(token, "OEM covariance")?);
                }
                continue;
            }
            let cols: Vec<&str> = line.split_whitespace().collect();
            if (cols.len() == 7 || cols.len() == 10) && cols[0].contains('T') {
                let mut values = [0.0; 6];
                for (index, target) in values.iter_mut().enumerate() {
                    *target = parse_f64(cols[index + 1], "OEM state")?;
                }
                let acceleration = if cols.len() == 10 {
                    Some([
                        parse_f64(cols[7], "OEM X_DDOT")?,
                        parse_f64(cols[8], "OEM Y_DDOT")?,
                        parse_f64(cols[9], "OEM Z_DDOT")?,
                    ])
                } else {
                    None
                };
                states_raw.push((cols[0].to_string(), values, acceleration));
                seen_segment_data = true;
            } else if cols.len() > 1
                && cols[0].contains('T')
                && cols[1..].iter().all(|column| column.parse::<f64>().is_ok())
            {
                return Err(SimError::InvalidArgument(format!(
                    "OEM ephemeris line must carry 6 or 9 numeric fields, found {}",
                    cols.len() - 1
                )));
            }
        }
        flush_segment(
            &mut metadata,
            &mut states_raw,
            &mut covariance_raw,
            &mut segments,
        )?;
        if segments.is_empty() {
            return Err(SimError::InvalidArgument("OEM contains no segments".into()));
        }
        Ok(Self { header, segments })
    }

    pub fn from_xml(input: &str) -> Result<Self> {
        let root = parse_xml_tree(input)?;
        let mut header = BTreeMap::new();
        if let Some(node) = root.descendant("header") {
            header = node.leaf_map();
        }
        let mut segment_nodes = Vec::new();
        root.descendants("segment", &mut segment_nodes);
        if segment_nodes.is_empty() {
            return Err(SimError::InvalidArgument(
                "OEM XML contains no segment".into(),
            ));
        }
        let mut segments = Vec::new();
        for segment in segment_nodes {
            let metadata_node = segment.child("metadata").ok_or_else(|| {
                SimError::InvalidArgument("OEM XML segment missing metadata".into())
            })?;
            let metadata = metadata_node.leaf_map();
            let data = segment
                .child("data")
                .ok_or_else(|| SimError::InvalidArgument("OEM XML segment missing data".into()))?;
            let mut states_raw = Vec::new();
            let mut state_nodes = Vec::new();
            data.descendants("stateVector", &mut state_nodes);
            for state in state_nodes {
                let fields = state.leaf_map();
                let epoch = get_required(&fields, "EPOCH")?.to_string();
                let values = [
                    state_component(state, "X", false)?,
                    state_component(state, "Y", false)?,
                    state_component(state, "Z", false)?,
                    state_component(state, "X_DOT", true)?,
                    state_component(state, "Y_DOT", true)?,
                    state_component(state, "Z_DOT", true)?,
                ];
                let acceleration_names = ["X_DDOT", "Y_DDOT", "Z_DDOT"];
                let present = acceleration_names.map(|name| state.child(name).is_some());
                let acceleration_si = match present {
                    [false, false, false] => None,
                    [true, true, true] => Some([
                        state_acceleration_component(state, "X_DDOT")?,
                        state_acceleration_component(state, "Y_DDOT")?,
                        state_acceleration_component(state, "Z_DDOT")?,
                    ]),
                    _ => {
                        return Err(SimError::InvalidArgument(
                            "OEM XML acceleration must provide X_DDOT, Y_DDOT and Z_DDOT together"
                                .into(),
                        ));
                    }
                };
                // Position/velocity values are normalized here to SI then mapped
                // back to the KVN-default km/km/s storage used by build_segment.
                states_raw.push((
                    epoch,
                    values.map(|value| value / 1_000.0),
                    acceleration_si.map(|a| a.map(|value| value / 1_000.0)),
                ));
            }
            let mut covariance_raw = Vec::new();
            let mut cov_nodes = Vec::new();
            data.descendants("covarianceMatrix", &mut cov_nodes);
            for covariance in cov_nodes {
                let fields = covariance.leaf_map();
                let epoch = get_required(&fields, "EPOCH")?.to_string();
                let frame = fields
                    .get("COV_REF_FRAME")
                    .cloned()
                    .unwrap_or_else(|| metadata.get("REF_FRAME").cloned().unwrap_or_default());
                let ordered = [
                    "CX_X",
                    "CY_X",
                    "CY_Y",
                    "CZ_X",
                    "CZ_Y",
                    "CZ_Z",
                    "CX_DOT_X",
                    "CX_DOT_Y",
                    "CX_DOT_Z",
                    "CX_DOT_X_DOT",
                    "CY_DOT_X",
                    "CY_DOT_Y",
                    "CY_DOT_Z",
                    "CY_DOT_X_DOT",
                    "CY_DOT_Y_DOT",
                    "CZ_DOT_X",
                    "CZ_DOT_Y",
                    "CZ_DOT_Z",
                    "CZ_DOT_X_DOT",
                    "CZ_DOT_Y_DOT",
                    "CZ_DOT_Z_DOT",
                ];
                let values = ordered
                    .iter()
                    .map(|key| get_required(&fields, key).and_then(|v| parse_f64(v, key)))
                    .collect::<Result<Vec<_>>>()?;
                covariance_raw.push((epoch, frame, values));
            }
            segments.push(build_segment(metadata, states_raw, covariance_raw)?);
        }
        Ok(Self { header, segments })
    }
}

fn build_segment(
    metadata: BTreeMap<String, String>,
    states_raw: Vec<RawOemState>,
    covariance_raw: Vec<RawOemCovariance>,
) -> Result<OemSegment> {
    let time_scale = parse_time_scale(get_required(&metadata, "TIME_SYSTEM")?)?;
    let frame = parse_reference_frame(get_required(&metadata, "REF_FRAME")?)?;
    if states_raw.len() < 2 {
        return Err(SimError::InvalidArgument(
            "OEM segment requires at least two state vectors".into(),
        ));
    }
    let acceleration_count = states_raw
        .iter()
        .filter(|(_, _, acceleration)| acceleration.is_some())
        .count();
    if acceleration_count != 0 && acceleration_count != states_raw.len() {
        return Err(SimError::InvalidArgument(
            "OEM segment must use a consistent P/V or P/V/A state layout".into(),
        ));
    }
    let mut states = Vec::with_capacity(states_raw.len());
    for (epoch_raw, values, acceleration) in states_raw {
        let epoch = AbsoluteEpoch::parse_ccsds(&epoch_raw, time_scale)?;
        states.push(AbsoluteOrbitState::new_with_acceleration(
            epoch,
            frame,
            [
                values[0] * 1_000.0,
                values[1] * 1_000.0,
                values[2] * 1_000.0,
            ],
            [
                values[3] * 1_000.0,
                values[4] * 1_000.0,
                values[5] * 1_000.0,
            ],
            acceleration.map(|a| a.map(|value| value * 1_000.0)),
        )?);
    }
    if states
        .windows(2)
        .any(|pair| pair[1].epoch.julian_date() <= pair[0].epoch.julian_date())
    {
        return Err(SimError::InvalidArgument(
            "OEM state epochs must be strictly increasing".into(),
        ));
    }
    let mut covariances = Vec::new();
    for (epoch_raw, frame_raw, values) in covariance_raw {
        let epoch = AbsoluteEpoch::parse_ccsds(&epoch_raw, time_scale)?;
        let covariance_frame = parse_reference_frame(&frame_raw)?;
        // OEM KVN/XML covariance uses km², km²/s and km²/s² by standard default.
        let mut converted = Vec::with_capacity(values.len());
        for value in &values {
            let scale = 1.0e6;
            converted.push(value * scale);
        }
        covariances.push(OemCovariance {
            epoch,
            frame: covariance_frame,
            lower_triangular_si: converted,
        });
    }
    Ok(OemSegment {
        metadata,
        states,
        covariances,
        extensions: BTreeMap::new(),
    })
}

fn state_component(node: &XmlNode, name: &str, velocity: bool) -> Result<f64> {
    let child = node
        .child(name)
        .ok_or_else(|| SimError::InvalidArgument(format!("OEM XML state vector missing {name}")))?;
    let value = parse_f64(child.text.trim(), name)?;
    let unit = child
        .attributes
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case("units"))
        .map(|(_, value)| value.as_str());
    if velocity {
        convert_velocity_to_m_per_s(value, unit)
    } else {
        convert_distance_to_m(value, unit)
    }
}

fn state_acceleration_component(node: &XmlNode, name: &str) -> Result<f64> {
    let child = node
        .child(name)
        .ok_or_else(|| SimError::InvalidArgument(format!("OEM XML state vector missing {name}")))?;
    let value = parse_f64(child.text.trim(), name)?;
    let unit = child
        .attributes
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case("units"))
        .map(|(_, value)| value.as_str());
    convert_acceleration_to_m_per_s2(value, unit)
}

fn strip_unit(value: &str) -> (String, Option<String>) {
    if let Some(start) = value.rfind('[') {
        if value.ends_with(']') {
            return (
                value[..start].trim().to_string(),
                Some(value[start + 1..value.len() - 1].trim().to_string()),
            );
        }
    }
    (value.trim().to_string(), None)
}
