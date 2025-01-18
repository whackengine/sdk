use std::{collections::HashMap, path::PathBuf};
use colored::Colorize;
use semver::Version;
use crate::packagemanager::*;

pub struct DependencyUpdate;

impl DependencyUpdate {
    pub async fn update_dependencies(entry_dir: &PathBuf, manifest: &WhackManifest, run_cache_file: &mut RunCacheFile, conflicting_dependencies_tracker: &mut HashMap<String, HashMap<String, Version>>) {
        // TODO: detect version conflicts by contributing to and using the
        // `conflicting_dependencies_tracker` table.

        fixme();
    }
}