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
        sgp4::Elements::from_tle(object_name.clone(), line1.as_bytes(), line2.as_bytes())
            .map_err(|error| SimError::InvalidArgument(format!("invalid TLE: {error}")))?;
        Ok(Self {
            object_name,
            line1,
            line2,
        })
    }

    pub fn elements(&self) -> Result<sgp4::Elements> {
        sgp4::Elements::from_tle(
            self.object_name.clone(),
            self.line1.as_bytes(),
            self.line2.as_bytes(),
        )
        .map_err(|error| SimError::InvalidArgument(format!("invalid TLE: {error}")))
    }
}
