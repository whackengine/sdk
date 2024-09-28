use crate::packagemanager::*;
use colored::*;

pub async fn check_process(matches: &clap::ArgMatches) {
    let builtins = matches.get_one::<std::path::PathBuf>("builtins");
    let package = matches.get_one::<String>("package");

    let dir = std::env::current_dir().unwrap();

    // Read lockfile
    let lockfile_path = dir.join("whack.lock");
    let mut lockfile: Option<WhackLockfile> = None;
    if std::fs::exists(&lockfile_path).unwrap() && std::fs::metadata(&lockfile_path).unwrap().is_file() {
        lockfile = toml::from_str::<WhackLockfile>(&std::fs::read_to_string(&lockfile_path).unwrap()).ok();
    }

    // Process directed acyclic graph
    let dag = match Dag::retrieve(&dir, &dir, package.cloned(), lockfile.as_ref()).await {
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

    // Check the built-ins first
    fixme();

    // Check each dependency in ascending order for AS3 and MXML errors,
    // running the build script if required.
    fixme();
}