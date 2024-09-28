use semver::Version;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct WhackLockfile {
    pub package: Vec<WhackLockfilePackage>,
}

#[derive(Serialize, Deserialize)]
pub struct WhackLockfilePackage {
    pub name: String,
    pub version: Version,
    pub source: Option<String>,
    pub dependencies: Option<Vec<String>>,
}