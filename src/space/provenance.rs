use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::{Result, SimError};

/// Version of the `sgp4` crate linked by this build.
///
/// `Cargo.toml` pins the exact same version; a unit test below prevents drift
/// between the executable dependency graph and serialized provenance.
pub const SGP4_CRATE_VERSION: &str = "2.4.0";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpaceProvenance {
    pub algorithm_version: String,
    pub input_sha256: String,
    pub eop_source_sha256: Option<String>,
    pub orekit_data_sha256: Option<String>,
    pub sgp4_version: Option<String>,
    pub standards: Vec<String>,
}

impl SpaceProvenance {
    pub fn for_input(input: &[u8]) -> Self {
        Self {
            algorithm_version: env!("CARGO_PKG_VERSION").to_string(),
            input_sha256: sha256_hex(input),
            eop_source_sha256: None,
            orekit_data_sha256: None,
            sgp4_version: Some(SGP4_CRATE_VERSION.to_string()),
            standards: vec![
                "CCSDS 502.0-B-3".to_string(),
                "IERS Conventions 2010 / IAU 2006-2000A".to_string(),
                "ITU-R P.676-13".to_string(),
                "ITU-R P.840-9".to_string(),
            ],
        }
    }

    pub fn with_eop_source(mut self, bytes: &[u8]) -> Self {
        self.eop_source_sha256 = Some(sha256_hex(bytes));
        self
    }

    pub fn with_orekit_data_dir(mut self, path: impl AsRef<Path>) -> Result<Self> {
        self.orekit_data_sha256 = Some(sha256_directory(path.as_ref())?);
        Ok(self)
    }
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    hex_digest(&digest)
}

/// Deterministic digest of a directory tree. Paths are normalized to `/`,
/// sorted lexicographically and length-prefixed before file content.
pub fn sha256_directory(root: &Path) -> Result<String> {
    if !root.is_dir() {
        return Err(SimError::InvalidArgument(format!(
            "provenance path is not a directory: {}",
            root.display()
        )));
    }
    let mut files = Vec::new();
    collect_files(root, root, &mut files)?;
    files.sort();
    let mut hasher = Sha256::new();
    for relative in files {
        let full = root.join(&relative);
        let name = relative.to_string_lossy().replace('\\', "/");
        let bytes = std::fs::read(&full)?;
        hasher.update((name.len() as u64).to_le_bytes());
        hasher.update(name.as_bytes());
        hasher.update((bytes.len() as u64).to_le_bytes());
        hasher.update(&bytes);
    }
    {
        let digest = hasher.finalize();
        Ok(hex_digest(&digest))
    }
}

fn collect_files(root: &Path, current: &Path, output: &mut Vec<PathBuf>) -> Result<()> {
    for entry in std::fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            collect_files(root, &path, output)?;
        } else if file_type.is_file() {
            let relative = path.strip_prefix(root).map_err(|_| {
                SimError::InvalidArgument("failed to compute provenance relative path".into())
            })?;
            output.push(relative.to_path_buf());
        }
    }
    Ok(())
}

fn hex_digest(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

#[cfg(test)]
mod tests {
    use super::SGP4_CRATE_VERSION;

    #[test]
    fn sgp4_pin_matches_declared_provenance() {
        let manifest = include_str!("../../Cargo.toml");
        let expected = format!("sgp4 = \"={SGP4_CRATE_VERSION}\"");
        assert!(
            manifest.contains(&expected),
            "Cargo.toml must pin {expected} to keep space provenance truthful"
        );
    }
}
