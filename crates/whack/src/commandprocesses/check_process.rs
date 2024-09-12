pub fn check_process(matches: &clap::ArgMatches) {
    let builtins = matches.get_one::<std::path::PathBuf>("builtins");
    let package = matches.get_one::<String>("package");

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

    // Check each dependency in ascending order for AS3 and MXML errors,
    // running the build script if required.
    fixme();
}