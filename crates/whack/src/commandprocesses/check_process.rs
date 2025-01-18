use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;
use crate::packagemanager::*;
use hydroperfox_filepaths::FlexPath;
use colored::Colorize;
use semver::Version;

pub async fn check_process(matches: &clap::ArgMatches) {
    let builtins = matches.get_one::<std::path::PathBuf>("builtins");
    let package = matches.get_one::<String>("package");

    let dir = std::env::current_dir().unwrap();

    // Read lockfile
    let mut lockfile: Option<WhackLockfile> = None;
    let mut dir = FlexPath::new_native(dir.to_str().unwrap());
    let mut found_base_manifest = false;
    loop {
        let manifest_path = PathBuf::from_str(&dir.resolve("whack.toml").to_string_with_flex_separator()).unwrap();

        if std::fs::exists(&manifest_path).unwrap() && std::fs::metadata(&manifest_path).unwrap().is_file() {
            found_base_manifest = true;

            let lockfile_path = PathBuf::from_str(&dir.resolve("whack.lock").to_string_with_flex_separator()).unwrap();
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

    // Entry point directory
    let dir = PathBuf::from_str(&dir.to_string_with_flex_separator()).unwrap();

    // Conflicting dependencies tracker
    let mut conflicting_dependencies_tracker = HashMap::<String, HashMap<String, Version>>::new();

    // Process directed acyclic graph
    let (dag, build_script_dag) = match Dag::retrieve(&dir, &dir, package.cloned(), lockfile.as_mut(), &mut run_cache_file, &mut conflicting_dependencies_tracker).await {
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

    // Check the built-ins first (process their Whack package and combine their DAG with each of the above DAGs)
    fixme();

    // Check each dependency in ascending order for AS3 and MXML errors,
    // running the build script if required.
    // (REMEMBER to ignore .include.as files)
    fixme();

    // Write to the run cache file
    std::fs::create_dir_all(&target_path).unwrap();
    std::fs::write(&run_cache_path, toml::to_string::<RunCacheFile>(&run_cache_file).unwrap()).unwrap();
}