use std::{collections::HashMap, path::PathBuf};
use colored::Colorize;
use lazy_regex::regex_is_match;
use semver::Version;
use crate::commandprocesses::WhackPackageProcessingError;
use crate::packagemanager::*;

pub struct DependencyUpdate;

impl DependencyUpdate {
    pub async fn update_dependencies(entry_dir: &PathBuf, manifest: &WhackManifest, run_cache_file: &mut RunCacheFile, conflicting_dependencies_tracker: &mut HashMap<String, HashMap<String, Version>>, lockfile: &mut WhackLockfile) -> Result<(), WhackPackageProcessingError> {
        // TODO: detect version conflicts by reading the
        // `conflicting_dependencies_tracker` table.

        let mut deps = HashMap::<String, ManifestDependency>::new();
        if let Some(deps1) = manifest.dependencies.as_ref() {
            deps.extend(deps1.iter().map(|(k, v)| (k.clone(), v.clone())));
        }
        if let Some(deps1) = manifest.build_dependencies.as_ref() {
            deps.extend(deps1.iter().map(|(k, v)| (k.clone(), v.clone())));
        }

        for (name, dep) in deps.iter() {
            if !regex_is_match!(r"[A-Za-z0-9.\-_]+", name) {
                return Err(WhackPackageProcessingError::IllegalPackageName { name: name.clone() });
            }
            match dep {
                ManifestDependency::Version(_ver) => {
                    panic!("Registry dependencies are not implemented yet.");
                },
                ManifestDependency::Advanced { version: _, path, git, rev, branch } => {
                    if path.is_none() {
                        panic!("Registry or Git dependencies are not implemented yet.");
                    }
                },
            }
        }

        Ok(())
    }
}