use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Number, Value};

use crate::{Result, SimError};

use super::ccsds::{parse_xml_tree, CcsdsKeyValue};

const FLOAT_KEYS: &[&str] = &[
    "MEAN_MOTION",
    "ECCENTRICITY",
    "INCLINATION",
    "RA_OF_ASC_NODE",
    "ARG_OF_PERICENTER",
    "MEAN_ANOMALY",
    "BSTAR",
    "MEAN_MOTION_DOT",
    "MEAN_MOTION_DDOT",
];
const INTEGER_KEYS: &[&str] = &[
    "EPHEMERIS_TYPE",
    "NORAD_CAT_ID",
    "ELEMENT_SET_NO",
    "REV_AT_EPOCH",
];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OmmMessage {
    pub fields: BTreeMap<String, String>,
}

impl OmmMessage {
    pub fn from_json(input: &str) -> Result<Self> {
        let value: Value = serde_json::from_str(input)?;
        let object = value
            .as_object()
            .ok_or_else(|| SimError::InvalidArgument("OMM JSON must be an object".into()))?;
        let fields = object
            .iter()
            .map(|(k, v)| (k.clone(), json_scalar(v)))
            .collect();
        let message = Self { fields };
        message.elements()?;
        Ok(message)
    }

    pub fn from_kvn(input: &str) -> Result<Self> {
        let mut fields = BTreeMap::new();
        for line in input.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with("COMMENT") {
                continue;
            }
            if let Some((key, value)) = line.split_once('=') {
                let parsed = CcsdsKeyValue::parse(key.trim(), value.trim());
                fields.insert(parsed.key, parsed.value);
            }
        }
        let message = Self { fields };
        message.elements()?;
        Ok(message)
    }

    pub fn from_xml(input: &str) -> Result<Self> {
        let tree = parse_xml_tree(input)?;
        let mut fields = BTreeMap::new();
        tree.collect_leaf_values(&mut fields);
        let message = Self { fields };
        message.elements()?;
        Ok(message)
    }

    pub fn elements(&self) -> Result<sgp4::Elements> {
        let mut map = Map::new();
        for (key, value) in &self.fields {
            if FLOAT_KEYS.contains(&key.as_str()) {
                let number = value.parse::<f64>().map_err(|_| {
                    SimError::InvalidArgument(format!("OMM field {key} must be numeric"))
                })?;
                let number = Number::from_f64(number).ok_or(SimError::NonFinite)?;
                map.insert(key.clone(), Value::Number(number));
            } else if INTEGER_KEYS.contains(&key.as_str()) {
                let number = value.parse::<u64>().map_err(|_| {
                    SimError::InvalidArgument(format!(
                        "OMM field {key} must be an unsigned integer"
                    ))
                })?;
                map.insert(key.clone(), Value::Number(Number::from(number)));
            } else {
                map.insert(key.clone(), Value::String(value.clone()));
            }
        }
        serde_json::from_value(Value::Object(map))
            .map_err(|error| SimError::InvalidArgument(format!("invalid OMM fields: {error}")))
    }
}

fn json_scalar(value: &Value) -> String {
    match value {
        Value::String(v) => v.clone(),
        Value::Number(v) => v.to_string(),
        Value::Bool(v) => v.to_string(),
        _ => value.to_string(),
    }
}
