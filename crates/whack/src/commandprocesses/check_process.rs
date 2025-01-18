use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;
use crate::packagemanager::*;
use hydroperfox_filepaths::FlexPath;
use colored::Colorize;
use semver::Version;

use super::CommandProcessCommons;

pub async fn check_process(matches: &clap::ArgMatches) {
    let builtins = matches.get_one::<std::path::PathBuf>("builtins").cloned().unwrap_or(PathBuf::from_str("../builtins/packages/whack.base").unwrap());
    let package = matches.get_one::<String>("package");

    let dir = std::env::current_dir().unwrap();

    // Detect entry point directory and read lockfile
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
        let mut next_dir = dir.resolve("..");
        if dir == next_dir || dir.to_string().is_empty() {
            break;
        }
    }

    if !found_base_manifest {
        println!("{} Currently not inside a Whack project.", "Error:".red());
        std::process::exit(1);
    }

    // Target path
    let target_path = PathBuf::from_str(&dir.resolve("target").to_string_with_flex_separator()).unwrap();

    // Read the run cache file
    let mut run_cache_file: Option<RunCacheFile> = None;
    let run_cache_path = PathBuf::from_str(&dir.resolve("target/.run-cache.toml").to_string_with_flex_separator()).unwrap();
    if std::fs::exists(&run_cache_path).unwrap() && std::fs::metadata(&run_cache_path).unwrap().is_file() {
        run_cache_file = Some(toml::from_str::<RunCacheFile>(&std::fs::read_to_string(&run_cache_path).unwrap()).unwrap());
    }
    if run_cache_file.is_none() {
        run_cache_file = Some(RunCacheFile {
            packages: vec![]
        });
    }
    let mut run_cache_file = run_cache_file.unwrap();

    // Initial lockfile
    if lockfile.is_none() {
        lockfile = Some(WhackLockfile {
            package: vec![]
        });
    }
    let mut lockfile = lockfile.unwrap();

    // Entry point directory
    let dir = PathBuf::from_str(&dir.to_string_with_flex_separator()).unwrap();

    // Conflicting dependencies tracker
    let mut conflicting_dependencies_tracker = HashMap::<String, HashMap<String, Version>>::new();

    // Package internator
    let mut package_internator = WhackPackageInternator::new();

    // Cycle prevention list (vector of package absolute paths)
    let mut cycle_prevention_list = Vec::<PathBuf>::new();

    // Process directed acyclic graph
    let (mut dag, mut build_script_dag) = match Dag::retrieve(dir.clone(), &dir, package.cloned(), &mut lockfile, &mut run_cache_file, &mut conflicting_dependencies_tracker, &mut package_internator, cycle_prevention_list).await {
        Ok(dag) => dag,
        Err(error) => {
            CommandProcessCommons::print_package_processing_error(error);
            return;
        },
    };

    // Process the built-ins as well.
    let (builtins_dag, builtins_build_script_dag) = match Dag::retrieve(dir.clone(), &dir, package.cloned(), &mut lockfile, &mut run_cache_file, &mut conflicting_dependencies_tracker, &mut package_internator, cycle_prevention_list).await {
        Ok(dag) => dag,
        Err(error) => {
            CommandProcessCommons::print_package_processing_error(error);
            return;
        },
    };
    dag.prepend_dag(builtins_dag);
    build_script_dag.prepend_dag(builtins_build_script_dag);

    // Check each dependency in ascending order for AS3 and MXML errors,
    // running the build script if required.
    // (REMEMBER to ignore .include.as files)
    fixme();

    // Write to the run cache file
    std::fs::create_dir_all(&target_path).unwrap();
    std::fs::write(&run_cache_path, toml::to_string::<RunCacheFile>(&run_cache_file).unwrap()).unwrap();

    // Write to the lock file
    std::fs::write(&lockfile_path, toml::to_string::<WhackLockfile>(&lockfile).unwrap()).unwrap();
}