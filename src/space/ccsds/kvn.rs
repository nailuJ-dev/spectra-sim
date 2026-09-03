use std::collections::BTreeMap;

use crate::{Result, SimError};

use super::CcsdsKeyValue;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KvnRecord {
    pub key: String,
    pub value: String,
    pub unit: Option<String>,
    pub line_number: usize,
}

pub fn parse_kvn_records(input: &str) -> Result<Vec<KvnRecord>> {
    if input.len() > 32 * 1024 * 1024 {
        return Err(SimError::DimensionLimit {
            actual: input.len(),
            max: 32 * 1024 * 1024,
        });
    }
    let mut records = Vec::new();
    for (index, raw) in input.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with("COMMENT") {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            let kv = CcsdsKeyValue::parse(key, value);
            records.push(KvnRecord {
                key: kv.key,
                value: kv.value,
                unit: kv.unit,
                line_number: index + 1,
            });
        }
    }
    Ok(records)
}

pub fn records_to_map(records: &[KvnRecord]) -> BTreeMap<String, String> {
    records
        .iter()
        .map(|record| (record.key.clone(), record.value.clone()))
        .collect()
}
