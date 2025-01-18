use std::{path::PathBuf, str::FromStr};
use crate::packagemanager::*;
use colored::Colorize;
use hydroperfox_filepaths::FlexPath;
use whackengine_verifier::ns::*;

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
            WhackPackageProcessingError::FileNotFound { path } => {
                println!("{} File not found: {}", "Error:".red(), path);
            },
            WhackPackageProcessingError::UnrecognizedSourceFileExtension { path } => {
                println!("{} Unrecognized source file extension at: {}", "Error:".red(), path);
            },
        }
    }

    pub fn recurse_source_files(path: &PathBuf) -> Result<Vec<Rc<CompilationUnit>>, WhackPackageProcessingError> {
        if !std::fs::exists(path).unwrap() {
            return Err(WhackPackageProcessingError::FileNotFound {
                path: path.to_str().unwrap().to_owned(),
            });
        }
        let m = std::fs::metadata(path).unwrap();
        if m.is_file() {
            let flexpath = FlexPath::new_native(path.to_str().unwrap());
            if !flexpath.has_extensions([".as", ".mxml"]) {
                return Err(WhackPackageProcessingError::UnrecognizedSourceFileExtension {
                    path: path.to_str().unwrap().to_owned(),
                });
            }
            let text = std::fs::read_to_string(path).unwrap();
            return Ok(vec![CompilationUnit::new(Some(path.canonicalize().unwrap().to_str().unwrap().to_owned()), text)]);
        }
        if m.is_dir() {
            let mut r: Vec<Rc<CompilationUnit>> = vec![];
            for filename in std::fs::read_dir(path).unwrap() {
                let subpath = filename.unwrap().path();
                let m = std::fs::metadata(&subpath).unwrap();
                if m.is_dir() {
                    r.extend(CommandProcessCommons::recurse_source_files(&subpath)?);
                    continue;
                }
                if subpath.ends_with(".include.as") {
                    continue;
                }
                if subpath.ends_with(".as") || subpath.ends_with(".mxml") {
                    let text = std::fs::read_to_string(&subpath).unwrap();
                    r.push(CompilationUnit::new(Some(subpath.canonicalize().unwrap().to_str().unwrap().to_owned()), text));
                }
            }
            return Ok(r);
        }
        Ok(vec![])
    }
}

pub enum WhackPackageProcessingError {
    ManifestNotFound,
    PackageMustBeSpecified,
    CircularDependency {
        directory: String,
    },
    InvalidManifest {
        manifest_path: String,
        message: String,
    },
    UnspecifiedWorkspaceMember,
    ManifestIsNotAPackage {
        manifest_path: String,
    },
    IllegalPackageName {
        name: String,
    },
    FileNotFound {
        path: String,
    },
    UnrecognizedSourceFileExtension {
        path: String
    },
}