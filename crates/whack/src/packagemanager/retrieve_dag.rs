/// Retrieves the directed acyclic graph of the dependency tree.
pub fn retrieve_dag(package: Option<String>) -> Result<Dag, DagError> {
    // Read the Whack manifest
    fixme();

    // If current project is a workspace, then require a package name
    // to be specified, at which the check process executes.
    fixme();

    // Check for manifest updates.
    fixme();

    // If the manifest has been updated, update dependencies
    // and clear up the build script's artifacts.
    fixme();

    // Build a directed acyclic graph (DAG) of the dependencies.
    fixme();
}