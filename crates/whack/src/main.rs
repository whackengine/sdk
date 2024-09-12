fn main() {
    let cmd = clap::Command::new("whack")
        .bin_name("whack")
        .subcommand_required(true)
        .subcommand(
            clap::command!("check")
                .arg(clap::arg!(--"builtins" <PATH>)
                    .value_parser(clap::value_parser!(std::path::PathBuf)))
        );
    let matches = cmd.get_matches();
}