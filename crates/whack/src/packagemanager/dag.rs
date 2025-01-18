use std::collections::HashMap;
use std::rc::Rc;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::{Duration, SystemTime};
use crate::packagemanager::*;
use hydroperfox_filepaths::FlexPath;
use colored::Colorize;
use semver::Version;

/// Directed acyclic graph of the dependency tree.
pub struct Dag {
    pub vertices: Vec<Rc<WhackPackage>>,
    pub edges: Vec<DagEdge>,
    pub first: Rc<WhackPackage>,
    pub last: Rc<WhackPackage>,
}

impl Dag {
    /// Retrieves the directed acyclic graph of the dependency tree.
    ///
    /// # Parameters
    /// 
    /// - `entry_dir` - The directory where the entry point "whack.toml" file lies and where
    ///   the "target" directory is stored.
    pub async fn retrieve(mut dir: &PathBuf, entry_dir: &PathBuf, mut package: Option<String>, mut lockfile: Option<&mut WhackLockfile>, run_cache_file: &mut RunCacheFile, conflicting_dependencies_tracker: &mut HashMap<String, HashMap<String, Version>>) -> Result<(Dag, Dag), DagError> {
        let mut manifest: Option<WhackManifest> = None;

        // Read the Whack manifest
        let mut flexdir = FlexPath::new_native(dir.to_str().unwrap());
        let mut manifest_path = PathBuf::from_str(&flexdir.resolve("whack.toml").to_string_with_flex_separator()).unwrap();

        if std::fs::exists(&manifest_path).unwrap() && std::fs::metadata(&manifest_path).unwrap().is_file() {
            let contents = std::fs::read_to_string(&manifest_path).unwrap();
            match toml::from_str::<WhackManifest>(&contents) {
                Ok(m) => {
                    manifest = Some(m);
                },
                Err(error) => {
                    println!("{} Whack manifest at {} contains invalid TOML: {}", "Error:".red(), manifest_path.to_str().unwrap(), error.message());
                    std::process::exit(1);
                }
            }
        }

        let mut manifest = manifest.unwrap();

        // If current project is a workspace, then require a package name
        // to be specified unless a default package is described.
        if let Some(workspace) = manifest.workspace.as_ref() {
            let mut package_ok = false;

            if let Some(p) = package.as_ref() {
                if let Some(workspace_default_package) = manifest.package.as_ref() {
                    if p == &workspace_default_package.name {
                        package_ok = true;
                    }
                }

                if !package_ok {
                    // Read the specified package's manifest and move into its directory
                    let (new_dir, new_manifest_path, new_manifest) = Dag::move_into_workspace_member(&flexdir, p, &workspace.members);
                    dir = &new_dir;
                    flexdir = FlexPath::new_native(&dir.to_str().unwrap());
                    manifest_path = new_manifest_path;
                    manifest = new_manifest;
                }
            } else if let Some(workspace_default_package) = manifest.package.as_ref() {
                package_ok = true;
            }

            if !package_ok {
                println!("{} Must specify which package to be processed in Whack workspace.", "Error:".red());
                std::process::exit(1);
            }
        }

        // Make sure manifest describes a package.
        if manifest.package.is_none() {
            println!("{} Whack manifest at {} does not describe a package.", "Error:".red(), manifest_path.to_str().unwrap());
            std::process::exit(1);
        }

        // Check for manifest updates (check the RunCacheFile). Mutate the
        // RunCacheFile, as well; writing new content to it.
        let mut manifest_last_modified = std::fs::metadata(&manifest_path).unwrap().modified().unwrap();
        let cur_relative_path = FlexPath::new_native(&entry_dir.to_str().unwrap()).relative(&dir.to_str().unwrap());
        let manifest_updated = Dag::check_manifest_modified(manifest_last_modified, cur_relative_path, run_cache_file, &manifest.dependencies, &manifest.build_dependencies, &flexdir, entry_dir);

        // Contribute dependencies to the `conflicting_dependencies_tracker` table.
        let package_name: &String = &manifest.package.as_ref().unwrap().name;
        if conflicting_dependencies_tracker.get(package_name).is_none() {
            conflicting_dependencies_tracker.insert(package_name.clone(), HashMap::new());
        }
        let mut tracker1 = conflicting_dependencies_tracker.get_mut(package_name).unwrap();
        let deps = HashMap::<String, ManifestDependency>::new();
        if let Some(deps1) = manifest.dependencies {
            deps.extend(deps1.iter().map(|(k, v)| (k.clone(), v.clone())));
        }
        if let Some(deps1) = manifest.build_dependencies {
            deps.extend(deps1.iter().map(|(k, v)| (k.clone(), v.clone())));
        }
        for (name, dep) in deps.iter() {
            match dep {
                ManifestDependency::Version(ver) => {
                    tracker1.insert(name.clone(), ver.clone());
                },
                ManifestDependency::Advanced { version, .. } => {
                    if let Some(version) = version {
                        tracker1.insert(name.clone(), version.clone());
                    }
                },
            }
        }

        // If the manifest has been updated,
        // either in the entry package or in another local package,
        // update dependencies and clear up their run cache files.
        // Remember that the lock file must be considered for the
        // exact versions of registry dependencies.
        if manifest_updated {
            DependencyUpdate::update_dependencies(entry_dir, &manifest, run_cache_file, conflicting_dependencies_tracker).await;
        }

        // Build a directed acyclic graph (DAG) of the dependencies:
        // one for the project's dependencies and one for the
        // build script's dependencies.
        fixme();

        // Return result
        fixme()
    }

    fn move_into_workspace_member(flexdir: &FlexPath, package: &str, members: &Vec<String>) -> (PathBuf, PathBuf, WhackManifest) {
        for member in members.iter() {
            let member_flexdir = flexdir.resolve(member);
            let member_manifest_flexpath = member_flexdir.resolve("whack.toml");

            let member_dir = PathBuf::from_str(&member_flexdir.to_string_with_flex_separator()).unwrap();
            let member_manifest_path = PathBuf::from_str(&member_manifest_flexpath.to_string_with_flex_separator()).unwrap();

            if std::fs::exists(&member_dir).unwrap() && std::fs::metadata(&member_dir).unwrap().is_dir()
            && std::fs::exists(&member_manifest_path).unwrap() && std::fs::metadata(&member_manifest_path).unwrap().is_file() {
                let contents = std::fs::read_to_string(&member_manifest_path).unwrap();
                match toml::from_str::<WhackManifest>(&contents) {
                    Ok(m) => {
                        if let Some(p) = m.package.as_ref() {
                            if p.name == package {
                                return (member_dir, member_manifest_path, m);
                            }
                        }
                    },
                    Err(error) => {
                        println!("{} Whack manifest at {} contains invalid TOML: {}", "Error:".red(), member_manifest_path.to_str().unwrap(), error.message());
                        std::process::exit(1);
                    }
                }
            }
        }

        println!("{} Could not find member {}", "Error:".red(), package);
        std::process::exit(1);
    }

    fn check_manifest_modified(manifest_last_modified: SystemTime, cur_relative_path: String, run_cache_file: &mut RunCacheFile, dependencies: &Option<HashMap<String, ManifestDependency>>, build_dependencies: &Option<HashMap<String, ManifestDependency>>, flexdir: &FlexPath, entry_dir: &PathBuf) -> bool {
        let mut manifest_updated: bool = true;
        let mut found_run_cache = false;
        for p in run_cache_file.packages.iter_mut() {
            if cur_relative_path == p.path {
                found_run_cache = true;

                // Found the package into the run cache file
                if (SystemTime::UNIX_EPOCH + Duration::from_secs(p.manifest_last_modified)) == manifest_last_modified {
                    manifest_updated = false;
                } else {
                    p.manifest_last_modified = manifest_last_modified.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
                    p.build_script_run = false;
                }
                break;
            }
        }
        if !found_run_cache {
            run_cache_file.packages.push(RunCacheFilePackage {
                path: cur_relative_path,
                manifest_last_modified: manifest_last_modified.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs(),
                build_script_run: false,
            });
        }

        // Check in local dependencies
        let mut deps = HashMap::<String, ManifestDependency>::new();
        if let Some(deps1) = dependencies {
            deps.extend(deps1.iter().map(|(k, v)| (k.clone(), v.clone())));
        }
        if let Some(deps1) = build_dependencies {
            deps.extend(deps1.iter().map(|(k, v)| (k.clone(), v.clone())));
        }
        for (_, dep) in deps.iter() {
            if let ManifestDependency::Advanced { ref path, .. } = dep {
                if let Some(path) = path {
                    let local_dep_flexdir = flexdir.resolve(path);
                    let local_dep_dir = local_dep_flexdir.to_string_with_flex_separator();
                    let local_dep_manifest_path = flexdir.resolve_n([path, "whack.toml"]).to_string_with_flex_separator();
                    if std::fs::exists(&local_dep_manifest_path).unwrap() && std::fs::metadata(&local_dep_manifest_path).unwrap().is_file() {
                        let contents = std::fs::read_to_string(&local_dep_manifest_path).unwrap();
                        match toml::from_str::<WhackManifest>(&contents) {
                            Ok(m) => {
                                if m.package.is_some() {
                                    let manifest_last_modified = std::fs::metadata(&local_dep_manifest_path).unwrap().modified().unwrap();
                                    let cur_relative_path = FlexPath::new_native(&entry_dir.to_str().unwrap()).relative(&local_dep_dir);
                                    let local_dep_manifest_updated = Dag::check_manifest_modified(manifest_last_modified, cur_relative_path, run_cache_file, &m.dependencies, &m.build_dependencies, &local_dep_flexdir, entry_dir);
                                    manifest_updated = manifest_updated || local_dep_manifest_updated;
                                }
                            },
                            _ => {},
                        }
                    }
                }
            }
        }

        manifest_updated
    }
}

pub struct DagEdge {
    pub from: Rc<WhackPackage>,
    pub to: Rc<WhackPackage>,
}

pub enum DagError {
    ManifestNotFound,
    PackageMustBeSpecified,
}