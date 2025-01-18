use crate::packagemanager::*;
use colored::Colorize;

pub struct CommandProcessCommons;

impl CommandProcessCommons {
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