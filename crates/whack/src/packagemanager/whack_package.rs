use std::path::PathBuf;

pub struct WhackPackage {
    /// Physical path relative to the entry path.
    pub path: PathBuf,
}