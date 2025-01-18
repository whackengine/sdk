use std::rc::Rc;
use std::path::PathBuf;
use std::str::FromStr;
use crate::packagemanager::*;
use hydroperfox_filepaths::FlexPath;
use colored::Colorize;

/// Directed acyclic graph of the dependency tree.
pub struct Dag {
    pub vertices: Vec<Rc<WhackPackage>>,
    pub edges: Vec<DagEdge>,
    pub first: Rc<WhackPackage>,
    pub last: Rc<WhackPackage>,
}

impl Dag {
    /// Retrieves the directed acyclic graph of the dependency tree.
    pub async fn retrieve(mut dir: &PathBuf, entry_dir: &PathBuf, mut package: Option<String>, mut lockfile: Option<&mut WhackLockfile>) -> Result<(Dag, Dag), DagError> {
        let mut manifest: Option<WhackManifest> = None;

        // Read the Whack manifest
        let mut flexdir = FlexPath::new_native(dir.to_str().unwrap());
        let manifest_path = PathBuf::from_str(&flexdir.resolve("whack.toml").to_string_with_flex_separator()).unwrap();

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
                    let (new_dir, new_manifest) = Dag::move_into_workspace_member(&flexdir, p, &workspace.members);
                    dir = &new_dir;
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

        // Check for manifest updates.
        fixme();

        // If the manifest has been updated,
        // either in the entry package or in another local package,
        // update dependencies and clear up the build script's artifacts.
        // Remember that the lock file must be considered for the
        // exact versions of registry dependencies.
        fixme();

        // Build a directed acyclic graph (DAG) of the dependencies:
        // one for the project's dependencies and one for the
        // build script's dependencies.
        fixme();

        // Return result
        fixme()
    }

    fn move_into_workspace_member(flexdir: &FlexPath, package: &str, members: &Vec<String>) -> (PathBuf, WhackManifest) {
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
                                return (member_dir, m);
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
}

pub struct DagEdge {
    pub from: Rc<WhackPackage>,
    pub to: Rc<WhackPackage>,
}

pub enum DagError {
    ManifestNotFound,
    PackageMustBeSpecified,
}