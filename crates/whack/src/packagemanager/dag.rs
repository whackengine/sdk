use std::rc::Rc;
use std::path::PathBuf;
use crate::packagemanager::*;

/// Directed acyclic graph of the dependency tree.
pub struct Dag {
    pub vertices: Vec<Rc<WhackPackage>>,
    pub edges: Vec<DagEdge>,
    pub first: Rc<WhackPackage>,
    pub last: Rc<WhackPackage>,
}

impl Dag {
    /// Retrieves the directed acyclic graph of the dependency tree.
    pub async fn retrieve(dir: &PathBuf, entry_dir: &PathBuf, package: Option<String>, mut lockfile: Option<&mut WhackLockfile>) -> Result<Dag, DagError> {
        // Read the Whack manifest
        fixme();

        // If current project is a workspace, then require a package name
        // to be specified, at which the check process executes.
        fixme();

        // Check for manifest updates.
        fixme();

        // If the manifest has been updated,
        // either in the entry package or in another local package,
        // update dependencies and clear up the build script's artifacts.
        // Remember that the lock file must be considered for the
        // exact versions of registry dependencies.
        fixme();

        // Build a directed acyclic graph (DAG) of the dependencies.
        fixme();
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