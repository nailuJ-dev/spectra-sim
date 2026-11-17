use serde::{Deserialize, Serialize};

use crate::{Result, SimError};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tle {
    pub object_name: Option<String>,
    pub line1: String,
    pub line2: String,
}

impl Tle {
    pub fn parse(
        object_name: Option<String>,
        line1: impl Into<String>,
        line2: impl Into<String>,
    ) -> Result<Self> {
        let line1 = line1.into();
        let line2 = line2.into();
        check_checksum_columns(&line1, &line2)?;
        sgp4::Elements::from_tle(object_name.clone(), line1.as_bytes(), line2.as_bytes())
            .map_err(|error| SimError::InvalidArgument(format!("invalid TLE: {error}")))?;
        Ok(Self {
            object_name,
            line1,
            line2,
        })
    }

    pub fn elements(&self) -> Result<sgp4::Elements> {
        check_checksum_columns(&self.line1, &self.line2)?;
        sgp4::Elements::from_tle(
            self.object_name.clone(),
            self.line1.as_bytes(),
            self.line2.as_bytes(),
        )
        .map_err(|error| SimError::InvalidArgument(format!("invalid TLE: {error}")))
    }
}

/// The upstream parser computes `checksum_digit - b'0'` on byte 68 of each
/// line without checking it is a digit, which underflows (and panics in
/// overflow-checked builds) for any other byte. Reject that case up front.
fn check_checksum_columns(line1: &str, line2: &str) -> Result<()> {
    for (number, line) in [(1, line1), (2, line2)] {
        if line.as_bytes().get(68).is_some_and(|b| !b.is_ascii_digit()) {
            return Err(SimError::InvalidArgument(format!(
                "invalid TLE: line {number} checksum column must be a digit"
            )));
        }
    }
    Ok(())
}
