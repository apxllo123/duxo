pub mod plan;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Stable boundary for adaptation/conversion operations.
///
/// The Electron UI must talk to this layer through an explicit API contract rather
/// than depending on implementation details from the Analyzer.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AdapterCapabilities {
    pub analyzer: bool,
    pub conversion: bool,
}

impl Default for AdapterCapabilities {
    fn default() -> Self {
        Self {
            analyzer: true,
            conversion: false,
        }
    }
}

pub fn validate_input_directory(path: &Path) -> Result<()> {
    if !path.exists() {
        anyhow::bail!("input path does not exist: {}", path.display());
    }
    if !path.is_dir() {
        anyhow::bail!("input path is not a directory: {}", path.display());
    }
    Ok(())
}
