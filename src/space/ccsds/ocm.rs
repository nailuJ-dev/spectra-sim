use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{AbsoluteEpoch, AbsoluteOrbitState, ReferenceFrame, Result, SimError};

use super::{parse_f64, parse_reference_frame, parse_time_scale, parse_xml_tree, XmlNode};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OcmTrajectoryRecord {
    pub epoch: AbsoluteEpoch,
    /// Numeric trajectory elements exactly as parsed from the message.
    pub raw_fields: Vec<f64>,
    /// SI Cartesian PV state when TRAJ_TYPE is CARTPV or CARTPVA.
    /// Other CCSDS trajectory element types remain available losslessly in raw_fields.
    pub cartesian_state: Option<AbsoluteOrbitState>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OcmTrajectoryBlock {
    pub metadata: BTreeMap<String, String>,
    pub records: Vec<OcmTrajectoryRecord>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OcmTrajectoryState {
    pub epoch: AbsoluteEpoch,
    pub state: AbsoluteOrbitState,
    pub raw_fields: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OcmMessage {
    pub header: BTreeMap<String, String>,
    pub metadata: BTreeMap<String, String>,
    pub trajectory_blocks: Vec<OcmTrajectoryBlock>,
    /// Convenience flattened view containing only Cartesian PV/PVA trajectory records.
    pub trajectory_states: Vec<OcmTrajectoryState>,
    pub physical_properties: BTreeMap<String, String>,
    pub orbit_determination: BTreeMap<String, String>,
    pub perturbations: BTreeMap<String, String>,
    pub maneuvers: Vec<BTreeMap<String, String>>,
    pub user_defined: BTreeMap<String, String>,
    /// Raw covariance data rows. Typed covariance interpretation is intentionally
    /// left to the declared COV_TYPE/COV_UNITS metadata or the Orekit backend.
    pub covariance_rows: Vec<Vec<f64>>,
    /// Unknown/unsupported sections are retained rather than silently discarded.
    pub extensions: BTreeMap<String, Vec<String>>,
}

impl OcmMessage {
    pub fn from_kvn(input: &str) -> Result<Self> {
        if input.len() > 32 * 1024 * 1024 {
            return Err(SimError::DimensionLimit {
                actual: input.len(),
                max: 32 * 1024 * 1024,
            });
        }
        let mut result = Self::empty();
        let mut section = String::from("HEADER");
        let mut current_maneuver = BTreeMap::new();
        let mut current_traj_metadata = BTreeMap::new();
        let mut current_traj_lines: Vec<String> = Vec::new();

        for raw in input.lines() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with("COMMENT") {
                continue;
            }
            if line.ends_with("_START") && !line.contains('=') {
                section = line.trim_end_matches("_START").to_ascii_uppercase();
                if section == "MAN" || section == "MANEUVER" {
                    current_maneuver.clear();
                }
                if section == "TRAJ" || section == "TRAJECTORY" {
                    current_traj_metadata.clear();
                    current_traj_lines.clear();
                }
                continue;
            }
            if line.ends_with("_STOP") && !line.contains('=') {
                if (section == "MAN" || section == "MANEUVER") && !current_maneuver.is_empty() {
                    result.maneuvers.push(current_maneuver.clone());
                    current_maneuver.clear();
                }
                if section == "TRAJ" || section == "TRAJECTORY" {
                    let block = build_trajectory_block(
                        &result.metadata,
                        current_traj_metadata.clone(),
                        current_traj_lines.clone(),
                    )?;
                    result.push_trajectory_block(block);
                    current_traj_metadata.clear();
                    current_traj_lines.clear();
                }
                section = String::from("DATA");
                continue;
            }
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim().to_ascii_uppercase();
                let value = strip_unit(value.trim());
                match section.as_str() {
                    "HEADER" => {
                        result.header.insert(key, value);
                    }
                    "META" | "METADATA" => {
                        result.metadata.insert(key, value);
                    }
                    "TRAJ" | "TRAJECTORY" => {
                        current_traj_metadata.insert(key, value);
                    }
                    "PHYS" | "PHYSICAL_PROPERTIES" => {
                        result.physical_properties.insert(key, value);
                    }
                    "OD" | "ORBIT_DETERMINATION" => {
                        result.orbit_determination.insert(key, value);
                    }
                    "PERT" | "PERTURBATIONS" => {
                        result.perturbations.insert(key, value);
                    }
                    "MAN" | "MANEUVER" => {
                        current_maneuver.insert(key, value);
                    }
                    "USER" | "USER_DEFINED" => {
                        result.user_defined.insert(key, value);
                    }
                    other => {
                        result
                            .extensions
                            .entry(other.to_string())
                            .or_default()
                            .push(format!("{key}={value}"));
                    }
                }
                continue;
            }
            match section.as_str() {
                "TRAJ" | "TRAJECTORY" => current_traj_lines.push(line.to_string()),
                "COV" | "COVARIANCE" => {
                    let row = line
                        .split_whitespace()
                        .filter_map(|value| value.parse::<f64>().ok())
                        .collect::<Vec<_>>();
                    if !row.is_empty() {
                        result.covariance_rows.push(row);
                    } else {
                        result
                            .extensions
                            .entry("COV".into())
                            .or_default()
                            .push(line.to_string());
                    }
                }
                other => result
                    .extensions
                    .entry(other.to_string())
                    .or_default()
                    .push(line.to_string()),
            }
        }
        if section == "TRAJ" || section == "TRAJECTORY" {
            return Err(SimError::InvalidArgument(
                "OCM trajectory section is not terminated".into(),
            ));
        }
        result.validate()?;
        Ok(result)
    }

    pub fn from_xml(input: &str) -> Result<Self> {
        let root = parse_xml_tree(input)?;
        let mut result = Self::empty();
        if let Some(header) = root.descendant("header") {
            result.header = header.leaf_map();
        }
        if let Some(metadata) = root.descendant("metadata") {
            result.metadata = metadata.leaf_map();
        }

        copy_first_section(
            &root,
            &["phys", "physicalProperties"],
            &mut result.physical_properties,
        );
        copy_first_section(
            &root,
            &["od", "orbitDetermination"],
            &mut result.orbit_determination,
        );
        copy_first_section(&root, &["pert", "perturbations"], &mut result.perturbations);
        copy_first_section(
            &root,
            &["user", "userDefined", "userDefinedParameters"],
            &mut result.user_defined,
        );

        let mut maneuver_nodes = Vec::new();
        root.descendants("man", &mut maneuver_nodes);
        if maneuver_nodes.is_empty() {
            root.descendants("maneuver", &mut maneuver_nodes);
        }
        result.maneuvers = maneuver_nodes.into_iter().map(XmlNode::leaf_map).collect();

        let mut traj_nodes = Vec::new();
        root.descendants("traj", &mut traj_nodes);
        for traj in traj_nodes {
            let mut metadata = BTreeMap::new();
            let mut lines = Vec::new();
            for child in &traj.children {
                if child.name.eq_ignore_ascii_case("trajLine") {
                    if !child.text.trim().is_empty() {
                        lines.push(child.text.trim().to_string());
                    }
                } else if child.children.is_empty() && !child.text.trim().is_empty() {
                    metadata.insert(
                        child.name.to_ascii_uppercase(),
                        child.text.trim().to_string(),
                    );
                }
            }
            let block = build_trajectory_block(&result.metadata, metadata, lines)?;
            result.push_trajectory_block(block);
        }

        let mut cov_nodes = Vec::new();
        root.descendants("cov", &mut cov_nodes);
        for cov in cov_nodes {
            for child in &cov.children {
                if child.name.eq_ignore_ascii_case("covLine") {
                    let numbers = child
                        .text
                        .split_whitespace()
                        .filter_map(|value| value.parse::<f64>().ok())
                        .collect::<Vec<_>>();
                    if !numbers.is_empty() {
                        result.covariance_rows.push(numbers);
                    }
                }
            }
        }

        result.validate()?;
        Ok(result)
    }

    fn empty() -> Self {
        Self {
            header: BTreeMap::new(),
            metadata: BTreeMap::new(),
            trajectory_blocks: Vec::new(),
            trajectory_states: Vec::new(),
            physical_properties: BTreeMap::new(),
            orbit_determination: BTreeMap::new(),
            perturbations: BTreeMap::new(),
            maneuvers: Vec::new(),
            user_defined: BTreeMap::new(),
            covariance_rows: Vec::new(),
            extensions: BTreeMap::new(),
        }
    }

    fn push_trajectory_block(&mut self, block: OcmTrajectoryBlock) {
        for record in &block.records {
            if let Some(state) = &record.cartesian_state {
                self.trajectory_states.push(OcmTrajectoryState {
                    epoch: record.epoch,
                    state: state.clone(),
                    raw_fields: record.raw_fields.clone(),
                });
            }
        }
        self.trajectory_blocks.push(block);
    }

    fn validate(&self) -> Result<()> {
        if self.metadata.is_empty() {
            return Err(SimError::InvalidArgument("OCM metadata is empty".into()));
        }
        if self
            .trajectory_states
            .windows(2)
            .any(|pair| pair[1].epoch.julian_date() <= pair[0].epoch.julian_date())
        {
            return Err(SimError::InvalidArgument(
                "flattened OCM Cartesian trajectory epochs must increase".into(),
            ));
        }
        Ok(())
    }
}

fn build_trajectory_block(
    message_metadata: &BTreeMap<String, String>,
    block_metadata: BTreeMap<String, String>,
    lines: Vec<String>,
) -> Result<OcmTrajectoryBlock> {
    let time_scale = parse_time_scale(
        message_metadata
            .get("TIME_SYSTEM")
            .map(String::as_str)
            .unwrap_or("UTC"),
    )?;
    let traj_type = block_metadata
        .get("TRAJ_TYPE")
        .map(|value| value.to_ascii_uppercase())
        .unwrap_or_else(|| "CARTPV".to_string());
    let frame = trajectory_frame(message_metadata, &block_metadata).ok();
    let units = parse_units(block_metadata.get("TRAJ_UNITS").map(String::as_str));
    let mut records = Vec::new();
    for line in lines {
        let columns = line.split_whitespace().collect::<Vec<_>>();
        if columns.len() < 2 {
            return Err(SimError::InvalidArgument(
                "OCM trajLine contains no elements".into(),
            ));
        }
        let epoch = AbsoluteEpoch::parse_ccsds(columns[0], time_scale)?;
        let raw_fields = columns[1..]
            .iter()
            .map(|value| parse_f64(value, "OCM trajectory"))
            .collect::<Result<Vec<_>>>()?;
        let cartesian_state = match traj_type.as_str() {
            "CARTPV" | "CARTPVA" if raw_fields.len() >= 6 => {
                let frame = frame.ok_or_else(|| {
                    SimError::InvalidArgument(
                        "Cartesian OCM trajectory block is missing TRAJ_REF_FRAME".into(),
                    )
                })?;
                let position = [
                    convert_ocm_position(raw_fields[0], units.first().map(String::as_str))?,
                    convert_ocm_position(raw_fields[1], units.get(1).map(String::as_str))?,
                    convert_ocm_position(raw_fields[2], units.get(2).map(String::as_str))?,
                ];
                let velocity = [
                    convert_ocm_velocity(raw_fields[3], units.get(3).map(String::as_str))?,
                    convert_ocm_velocity(raw_fields[4], units.get(4).map(String::as_str))?,
                    convert_ocm_velocity(raw_fields[5], units.get(5).map(String::as_str))?,
                ];
                Some(AbsoluteOrbitState::new(epoch, frame, position, velocity)?)
            }
            _ => None,
        };
        records.push(OcmTrajectoryRecord {
            epoch,
            raw_fields,
            cartesian_state,
        });
    }
    if records
        .windows(2)
        .any(|pair| pair[1].epoch.julian_date() <= pair[0].epoch.julian_date())
    {
        return Err(SimError::InvalidArgument(
            "OCM trajectory block epochs must increase".into(),
        ));
    }
    Ok(OcmTrajectoryBlock {
        metadata: block_metadata,
        records,
    })
}

fn trajectory_frame(
    message_metadata: &BTreeMap<String, String>,
    block_metadata: &BTreeMap<String, String>,
) -> Result<ReferenceFrame> {
    for map in [block_metadata, message_metadata] {
        for key in ["TRAJ_REF_FRAME", "REF_FRAME", "CENTER_REF_FRAME"] {
            if let Some(value) = map.get(key) {
                return parse_reference_frame(value);
            }
        }
    }
    Err(SimError::InvalidArgument(
        "OCM metadata missing trajectory reference frame".into(),
    ))
}

fn parse_units(raw: Option<&str>) -> Vec<String> {
    raw.unwrap_or("")
        .trim_matches(|character| character == '[' || character == ']')
        .split(|character: char| character.is_ascii_whitespace() || character == ',')
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect()
}

fn convert_ocm_position(value: f64, unit: Option<&str>) -> Result<f64> {
    match unit.map(normalize_unit).as_deref() {
        None | Some("km") => Ok(value * 1_000.0),
        Some("m") => Ok(value),
        Some(other) => Err(SimError::InvalidArgument(format!(
            "unsupported OCM Cartesian position unit {other}"
        ))),
    }
}

fn convert_ocm_velocity(value: f64, unit: Option<&str>) -> Result<f64> {
    match unit.map(normalize_unit).as_deref() {
        None | Some("km/s") | Some("km*s^-1") | Some("km.s-1") => Ok(value * 1_000.0),
        Some("m/s") | Some("m*s^-1") | Some("m.s-1") => Ok(value),
        Some(other) => Err(SimError::InvalidArgument(format!(
            "unsupported OCM Cartesian velocity unit {other}"
        ))),
    }
}

fn normalize_unit(value: &str) -> String {
    value.trim().to_ascii_lowercase().replace(' ', "")
}

fn copy_first_section(root: &XmlNode, names: &[&str], target: &mut BTreeMap<String, String>) {
    for name in names {
        if let Some(node) = root.descendant(name) {
            *target = node.leaf_map();
            return;
        }
    }
}

fn strip_unit(value: &str) -> String {
    if let Some(start) = value.rfind('[') {
        if value.ends_with(']') {
            return value[..start].trim().to_string();
        }
    }
    value.trim().to_string()
}
