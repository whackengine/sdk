use clap::ArgAction;

#[tokio::main]
async fn main() {
    let cmd = clap::Command::new("whack")
        .bin_name("whack")
        .subcommand_required(true)
        .subcommand(
            clap::command!("check")
                .about("Verifies ActionScript sources for errors and warnings.")
                .arg(clap::arg!(--"builtins" <PATH>)
                    .help("Path to the Whack package defining the ActionScript built-ins.")
                    .value_parser(clap::value_parser!(std::path::PathBuf)))
                .arg(clap::arg!(--"package" <NAME>)
                    .help("For a workspace, specifies the Whack package to operate on.")
                    .alias("p"))
                .arg(clap::arg!(--"define" <KEYVALUE>)
                    .help("Defines a configuration constant with the syntax NS::NAME=val.")
                    .action(ArgAction::Append))
        );

    let matches = cmd.get_matches();

    match matches.subcommand() {
        Some(("check", matches)) => {
            whackengine_whack::commandprocesses::check_process(matches).await;
        },
        _ => unreachable!(),
    }
}