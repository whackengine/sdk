pub fn check_process(matches: &clap::ArgMatches) {
    let builtins = matches.get_one::<std::path::PathBuf>("builtins");
    let package = matches.get_one::<String>("package");

    match crate::packagemanager::retrieve_dag(package.cloned()) {
    }

    // Check for the built-ins first
    fixme();

    // Check each dependency in ascending order for AS3 and MXML errors,
    // running the build script if required.
    fixme();
}