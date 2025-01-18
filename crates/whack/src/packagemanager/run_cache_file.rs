use serde::{Serialize, Deserialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct RunCacheFile {
    pub packages: Vec<RunCacheFilePackage>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct RunCacheFilePackage {
    /// Path relative to the entry point directory, which contains the
    /// whack.toml file.
    pub path: String,
    /// Time in seconds indicating when the manifest was last modified since
    /// last operation.
    pub manifest_last_modified: u64,
    /// Indicates whether the build script has been run or not.
    pub build_script_run: bool,
}