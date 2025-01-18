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
#[derive(Clone)]
pub struct Dag {
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
    pub async fn retrieve(mut dir: PathBuf, entry_dir: &PathBuf, package: Option<String>, lockfile: &mut WhackLockfile, run_cache_file: &mut RunCacheFile, conflicting_dependencies_tracker: &mut HashMap<String, HashMap<String, Version>>, package_internator: &mut WhackPackageInternator, cycle_prevention_list: Vec<PathBuf>) -> Result<(Dag, Dag), WhackPackageProcessingError> {
        if cycle_prevention_list.contains(&dir.canonicalize().unwrap()) {
            return Err(WhackPackageProcessingError::CircularDependency { directory: dir.to_str().unwrap().to_owned() });
        }
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
                    return Err(WhackPackageProcessingError::InvalidManifest{ manifest_path: manifest_path.to_str().unwrap().to_owned(), message: error.message().to_owned() });
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
                    dir = new_dir;
                    flexdir = FlexPath::new_native(&dir.to_str().unwrap());
                    manifest_path = new_manifest_path;
                    manifest = new_manifest;
                }
            } else if manifest.package.is_some() {
                package_ok = true;
            }

            if !package_ok {
                return Err(WhackPackageProcessingError::UnspecifiedWorkspaceMember);
            }
        }

        // Make sure manifest describes a package.
        if manifest.package.is_none() {
            return Err(WhackPackageProcessingError::ManifestIsNotAPackage { manifest_path: manifest_path.to_str().unwrap().to_owned() });
        }

        // Check for manifest updates (check the RunCacheFile). Mutate the
        // RunCacheFile, as well; writing new content to it.
        let manifest_last_modified = std::fs::metadata(&manifest_path).unwrap().modified().unwrap();
        let cur_relative_path = FlexPath::new_native(&entry_dir.to_str().unwrap()).relative(&dir.to_str().unwrap());
        let manifest_updated = Dag::check_manifest_modified(manifest_last_modified, cur_relative_path.clone(), run_cache_file, manifest.dependencies.as_ref(), manifest.build_dependencies.as_ref(), &flexdir, entry_dir);

        // Contribute dependencies to the `conflicting_dependencies_tracker` table.
        let package_name: &String = &manifest.package.as_ref().unwrap().name;
        if conflicting_dependencies_tracker.get(package_name).is_none() {
            conflicting_dependencies_tracker.insert(package_name.clone(), HashMap::new());
        }
        let tracker1 = conflicting_dependencies_tracker.get_mut(package_name).unwrap();
        let mut deps = HashMap::<String, ManifestDependency>::new();
        if let Some(deps1) = manifest.dependencies.as_ref() {
            deps.extend(deps1.iter().map(|(k, v)| (k.clone(), v.clone())));
        }
        if let Some(deps1) = manifest.build_dependencies.as_ref() {
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
            Box::pin(DependencyUpdate::update_dependencies(entry_dir, &manifest, run_cache_file, conflicting_dependencies_tracker, lockfile)).await?;
        }

        // Build a directed acyclic graph (DAG) of the dependencies:
        // one for the project's dependencies and one for the
        // build script's dependencies.
        let mut edges1: Vec<DagEdge> = vec![];
        let mut first1: Option<Rc<WhackPackage>> = None;
        let mut last1: Option<Rc<WhackPackage>> = None;

        // build script parts
        let mut edges2: Vec<DagEdge> = vec![];
        let mut first2: Option<Rc<WhackPackage>> = None;
        let mut last2: Option<Rc<WhackPackage>> = None;

        let mut next_cycle_prevention_list = cycle_prevention_list.clone();
        next_cycle_prevention_list.push(dir.canonicalize().unwrap());

        if let Some(deps) = manifest.dependencies.as_ref() {
            for (dep_name, dep) in deps.iter() {
                match dep {
                    ManifestDependency::Version(_) => {
                        let next_dir = PathBuf::from_str(&FlexPath::from_n_native([entry_dir.to_str().unwrap(), "target", dep_name]).to_string_with_flex_separator()).unwrap();
                        let (prepend_dag_1, prepend_dag_2) = Box::pin(Dag::retrieve(next_dir, entry_dir, None, lockfile, run_cache_file, conflicting_dependencies_tracker, package_internator, next_cycle_prevention_list.clone())).await?;
                        do_append_dag(prepend_dag_1, &mut edges1, &mut first1, &mut last1);
                        do_append_dag(prepend_dag_2, &mut edges2, &mut first2, &mut last2);
                    },
                    ManifestDependency::Advanced { path, .. } => {
                        let next_dir: PathBuf;
                        if let Some(path) = path {
                            next_dir = PathBuf::from_str(&FlexPath::from_n_native([dir.to_str().unwrap(), path]).to_string_with_flex_separator()).unwrap();
                        } else {
                            next_dir = PathBuf::from_str(&FlexPath::from_n_native([entry_dir.to_str().unwrap(), "target", dep_name]).to_string_with_flex_separator()).unwrap();
                        }
                        let (prepend_dag_1, prepend_dag_2) = Box::pin(Dag::retrieve(next_dir, entry_dir, None, lockfile, run_cache_file, conflicting_dependencies_tracker, package_internator, next_cycle_prevention_list.clone())).await?;
                        do_append_dag(prepend_dag_1, &mut edges1, &mut first1, &mut last1);
                        do_append_dag(prepend_dag_2, &mut edges2, &mut first2, &mut last2);
                    },
                }
            }
        }

        if let Some(deps) = manifest.build_dependencies.as_ref() {
            for (dep_name, dep) in deps.iter() {
                match dep {
                    ManifestDependency::Version(_version) => {
                        let next_dir = PathBuf::from_str(&FlexPath::from_n_native([entry_dir.to_str().unwrap(), "target", dep_name]).to_string_with_flex_separator()).unwrap();
                        let (prepend_dag_1, prepend_dag_2) = Box::pin(Dag::retrieve(next_dir, entry_dir, None, lockfile, run_cache_file, conflicting_dependencies_tracker, package_internator, next_cycle_prevention_list.clone())).await?;
                        do_append_dag(prepend_dag_1, &mut edges1, &mut first1, &mut last1);
                        do_append_dag(prepend_dag_2, &mut edges2, &mut first2, &mut last2);
                    },
                    ManifestDependency::Advanced { path, .. } => {
                        let next_dir: PathBuf;
                        if let Some(path) = path {
                            next_dir = PathBuf::from_str(&FlexPath::from_n_native([dir.to_str().unwrap(), path]).to_string_with_flex_separator()).unwrap();
                        } else {
                            next_dir = PathBuf::from_str(&FlexPath::from_n_native([entry_dir.to_str().unwrap(), "target", dep_name]).to_string_with_flex_separator()).unwrap();
                        }
                        let (prepend_dag_1, prepend_dag_2) = Box::pin(Dag::retrieve(next_dir, entry_dir, None, lockfile, run_cache_file, conflicting_dependencies_tracker, package_internator, next_cycle_prevention_list.clone())).await?;
                        do_append_dag(prepend_dag_1, &mut edges1, &mut first1, &mut last1);
                        do_append_dag(prepend_dag_2, &mut edges2, &mut first2, &mut last2);
                    },
                }
            }
        }

        let this_pckg = package_internator.intern(&dir, &cur_relative_path, &manifest);

        // 1 (not build script)
        if last1.is_some() {
            edges1.last_mut().unwrap().to = this_pckg.clone();
        }
        if first1.is_none() {
            first1 = Some(this_pckg.clone());
        }
        last1 = Some(this_pckg.clone());
        edges1.push(DagEdge {
            from: this_pckg.clone(),
            to: this_pckg.clone(),
        });

        // 2 (build script)
        if last2.is_some() {
            edges2.last_mut().unwrap().to = this_pckg.clone();
        }
        if first2.is_none() {
            first2 = Some(this_pckg.clone());
        }
        last2 = Some(this_pckg.clone());
        edges2.push(DagEdge {
            from: this_pckg.clone(),
            to: this_pckg,
        });

        Ok((
            Dag {
                edges: edges1,
                first: first1.unwrap(),
                last: last1.unwrap(),
            },
            Dag {
                edges: edges2,
                first: first2.unwrap(),
                last: last2.unwrap(),
            }
        ))
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

    fn check_manifest_modified(manifest_last_modified: SystemTime, cur_relative_path: String, run_cache_file: &mut RunCacheFile, dependencies: Option<&HashMap<String, ManifestDependency>>, build_dependencies: Option<&HashMap<String, ManifestDependency>>, flexdir: &FlexPath, entry_dir: &PathBuf) -> bool {
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
                path: cur_relative_path.clone(),
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
                                    let local_dep_manifest_updated = Dag::check_manifest_modified(manifest_last_modified, cur_relative_path, run_cache_file, m.dependencies.as_ref(), m.build_dependencies.as_ref(), &local_dep_flexdir, entry_dir);
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

    pub fn iter<'a>(&'a self) -> DagIterator<'a> {
        DagIterator {
            dag: self,
            current_package: Some(self.first.clone()),
        }
    }

    pub fn append_dag(&mut self, dag: Dag) {
        let mut fst = Some(self.first.clone());
        let mut lst = Some(self.last.clone());
        do_append_dag(dag, &mut self.edges, &mut fst, &mut lst);
        self.first = fst.unwrap();
        self.last = lst.unwrap();
    }

    pub fn prepend_dag(&mut self, dag: Dag) {
        do_prepend_dag(dag, &mut self.edges, &mut self.first);
    }
}

pub struct DagIterator<'a> {
    dag: &'a Dag,
    current_package: Option<Rc<WhackPackage>>,
}

impl<'a> Iterator for DagIterator<'a> {
    type Item = Rc<WhackPackage>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(current_package) = self.current_package.clone() {
            for edge in self.dag.edges.iter() {
                if Rc::ptr_eq(&edge.from, &current_package) {
                    if Rc::ptr_eq(&edge.to, &current_package) {
                        self.current_package = None;
                    } else {
                        self.current_package = Some(edge.to.clone());
                    }
                    return Some(current_package);
                }
            }
        }
        None
    }
}

fn do_append_dag(append_dag: Dag, edges: &mut Vec<DagEdge>, first: &mut Option<Rc<WhackPackage>>, last: &mut Option<Rc<WhackPackage>>) {
    if first.is_none() {
        *first = Some(append_dag.first.clone());
    }
    if last.is_some() {
        edges.last_mut().unwrap().to = append_dag.first;
    }
    edges.extend(append_dag.edges.iter().cloned());
    *last = Some(append_dag.last);
}

fn do_prepend_dag(mut prepend_dag: Dag, edges: &mut Vec<DagEdge>, first: &mut Rc<WhackPackage>) {
    prepend_dag.edges.last_mut().unwrap().to = first.clone();
    *first = prepend_dag.first;
    for i in 0..prepend_dag.edges.len() {
        edges.insert(i, prepend_dag.edges[i].clone());
    }
}

#[derive(Clone)]
pub struct DagEdge {
    pub from: Rc<WhackPackage>,
    pub to: Rc<WhackPackage>,
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
}