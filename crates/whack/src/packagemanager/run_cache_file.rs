use serde::{Serialize, Deserialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct RunCacheFile {
    pub packages: Vec<RunCacheFilePackage>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct RunCacheFilePackage {
    /// Path relative to the entry directory, which contains the
    /// whack.toml file.
    pub path: String,
    pub manifest_last_modified: u64,
    pub build_script_run: bool,
}