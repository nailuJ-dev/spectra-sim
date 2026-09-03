use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{Result, SimError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrekitBackend {
    java_binary: PathBuf,
    sidecar_jar: PathBuf,
    orekit_data_dir: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrekitResponse {
    pub ok: bool,
    pub result: Option<Value>,
    pub error: Option<String>,
    #[serde(rename = "orekitVersion")]
    pub orekit_version: Option<String>,
}

impl OrekitBackend {
    pub fn new(
        java_binary: impl Into<PathBuf>,
        sidecar_jar: impl Into<PathBuf>,
        orekit_data_dir: impl Into<PathBuf>,
    ) -> Result<Self> {
        let this = Self {
            java_binary: java_binary.into(),
            sidecar_jar: sidecar_jar.into(),
            orekit_data_dir: orekit_data_dir.into(),
        };
        this.validate_paths()?;
        Ok(this)
    }

    pub fn request(&self, request: &Value) -> Result<OrekitResponse> {
        self.validate_paths()?;
        let mut child = Command::new(&self.java_binary)
            .arg("-jar")
            .arg(&self.sidecar_jar)
            .env("OREKIT_DATA_DIR", &self.orekit_data_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| {
                SimError::InvalidArgument(format!("failed to start Orekit sidecar: {error}"))
            })?;
        {
            let stdin = child.stdin.as_mut().ok_or_else(|| {
                SimError::InvalidArgument("Orekit sidecar stdin unavailable".into())
            })?;
            let mut payload = serde_json::to_vec(request)?;
            payload.push(b'\n');
            stdin.write_all(&payload)?;
        }
        let output = child.wait_with_output()?;
        if !output.status.success() {
            return Err(SimError::InvalidArgument(format!(
                "Orekit sidecar exited with {}: {}",
                output.status,
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }
        let line = String::from_utf8(output.stdout)
            .map_err(|_| SimError::InvalidArgument("Orekit sidecar output is not UTF-8".into()))?;
        let response: OrekitResponse = serde_json::from_str(line.trim())?;
        if !response.ok {
            return Err(SimError::InvalidArgument(
                response
                    .error
                    .clone()
                    .unwrap_or_else(|| "Orekit sidecar failed".into()),
            ));
        }
        Ok(response)
    }

    pub fn health(&self) -> Result<OrekitResponse> {
        self.request(&serde_json::json!({"op":"health"}))
    }

    pub fn propagate_tle(
        &self,
        line1: &str,
        line2: &str,
        epoch_utc: &str,
    ) -> Result<OrekitResponse> {
        self.request(&serde_json::json!({
            "op": "propagate_tle",
            "line1": line1,
            "line2": line2,
            "epochUtc": epoch_utc,
        }))
    }

    pub fn transform_state(
        &self,
        source_frame: &str,
        target_frame: &str,
        epoch_utc: &str,
        position_m: [f64; 3],
        velocity_m_per_s: [f64; 3],
    ) -> Result<OrekitResponse> {
        self.request(&serde_json::json!({
            "op": "transform_state",
            "sourceFrame": source_frame,
            "targetFrame": target_frame,
            "epochUtc": epoch_utc,
            "positionM": position_m,
            "velocityMPerS": velocity_m_per_s,
        }))
    }

    pub fn parse_oem_summary(&self, content: &str) -> Result<OrekitResponse> {
        self.request(&serde_json::json!({"op":"parse_oem_summary", "content":content}))
    }

    pub fn parse_ocm_summary(&self, content: &str) -> Result<OrekitResponse> {
        self.request(&serde_json::json!({"op":"parse_ocm_summary", "content":content}))
    }

    fn validate_paths(&self) -> Result<()> {
        for (label, path) in [
            ("Java binary", self.java_binary.as_path()),
            ("Orekit sidecar JAR", self.sidecar_jar.as_path()),
            ("orekit-data directory", self.orekit_data_dir.as_path()),
        ] {
            if !path.exists() {
                return Err(SimError::InvalidArgument(format!(
                    "{label} does not exist: {}",
                    path.display()
                )));
            }
        }
        if !self.orekit_data_dir.is_dir() {
            return Err(SimError::InvalidArgument(
                "orekit-data path is not a directory".into(),
            ));
        }
        Ok(())
    }
}
