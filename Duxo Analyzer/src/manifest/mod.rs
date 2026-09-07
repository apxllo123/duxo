use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Serialize)]
pub struct GameManifest {
    pub root: PathBuf,
    pub files: Vec<FileRecord>,
    pub executables: Vec<ExecutableRecord>,
}

#[derive(Debug, Serialize)]
pub struct FileRecord {
    pub path: PathBuf,
    pub size: u64,
    pub sha256: String,
    pub extension: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ExecutableRecord {
    pub path: PathBuf,
    pub format: String,
    pub architecture: Option<String>,
    pub protection: ProtectionSignals,
}

#[derive(Debug, Default, Serialize)]
pub struct ProtectionSignals {
    /// Heuristic packer/protector matches found in the executable's sections or bytes.
    pub packers_or_protectors: Vec<String>,
    /// Heuristic anti-cheat matches found in the executable or nearby file name.
    pub anti_cheats: Vec<String>,
}
