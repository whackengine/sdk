use crate::packagemanager::*;
use colored::*;

pub fn check_process(matches: &clap::ArgMatches) {
    let builtins = matches.get_one::<std::path::PathBuf>("builtins");
    let package = matches.get_one::<String>("package");

    let dag = match Dag::retrieve(package.cloned()) {
        Ok(dag) => dag,
        Err(error) => {
            match error {
                DagError::ManifestNotFound => {
                    println!("{} {}", "Error:".red(), "Whack manifest not found.");
                },
                DagError::PackageMustBeSpecified => {
                    println!("{} {}", "Error:".red(), "Package must be specified.");
                },
            }

            return;
        },
    };

    // Check for the built-ins first
    fixme();

    // Check each dependency in ascending order for AS3 and MXML errors,
    // running the build script if required.
    fixme();
}