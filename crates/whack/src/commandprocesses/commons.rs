use std::{path::PathBuf, str::FromStr};
use crate::packagemanager::*;
use colored::Colorize;
use hydroperfox_filepaths::FlexPath;

pub struct CommandProcessCommons;

impl CommandProcessCommons {
    /// Returns (dir, lockfile, lockfile_path, found_base_manifest).
    pub fn entry_point_lookup(dir: &PathBuf) -> (FlexPath, Option<WhackLockfile>, PathBuf, bool) {
        let mut dir = FlexPath::new_native(dir.to_str().unwrap());
        let mut lockfile: Option<WhackLockfile> = None;
        let lockfile_path = PathBuf::from_str(&dir.resolve("whack.lock").to_string_with_flex_separator()).unwrap();
        let mut found_base_manifest = false;
        loop {
            let manifest_path = PathBuf::from_str(&dir.resolve("whack.toml").to_string_with_flex_separator()).unwrap();
    
            if std::fs::exists(&manifest_path).unwrap() && std::fs::metadata(&manifest_path).unwrap().is_file() {
                found_base_manifest = true;
    
                if std::fs::exists(&lockfile_path).unwrap() && std::fs::metadata(&lockfile_path).unwrap().is_file() {
                    lockfile = toml::from_str::<WhackLockfile>(&std::fs::read_to_string(&lockfile_path).unwrap()).ok();
                }
    
                break;
            }
    
            // Look up
            let next_dir = dir.resolve("..");
            if dir == next_dir || next_dir.to_string().is_empty() {
                break;
            }
            dir = next_dir;
        }

        (dir, lockfile, lockfile_path, found_base_manifest)
    }

    pub fn print_package_processing_error(error: WhackPackageProcessingError) {
        match error {
            WhackPackageProcessingError::ManifestNotFound => {
                println!("{} {}", "Error:".red(), "Whack manifest not found.");
            },
            WhackPackageProcessingError::PackageMustBeSpecified => {
                println!("{} {}", "Error:".red(), "Package must be specified.");
            },
            WhackPackageProcessingError::CircularDependency { directory } => {
                println!("{} Circular dependency is not allowed: {}", "Error:".red(), directory);
            },
            WhackPackageProcessingError::InvalidManifest { manifest_path, message } => {
                println!("{} Whack manifest at {} contains invalid TOML: {}", "Error:".red(), manifest_path, message);
            },
            WhackPackageProcessingError::UnspecifiedWorkspaceMember => {
                println!("{} Must specify which package to be processed in Whack workspace.", "Error:".red());
            },
            WhackPackageProcessingError::ManifestIsNotAPackage { manifest_path } => {
                println!("{} Whack manifest at {} does not describe a package.", "Error:".red(), manifest_path);
            },
            WhackPackageProcessingError::IllegalPackageName { name } => {
                println!("{} Found illegal package name: {}", "Error:".red(), name);
            },
        }
    }
}