use clap::{
    app_from_crate, crate_authors, crate_description, crate_name, crate_version, Arg, SubCommand,
};
use goblin::mach::Mach;
use goblin::Object;
use std::io::Write;

mod visibility;

fn main() {
    let matches = app_from_crate!()
        .arg(
            Arg::with_name("v")
                .short("v")
                .multiple(true)
                .help("Sets the level of verbosity"),
        )
        .subcommand(visibility::subcommand())
        .get_matches();

    let verbosity = matches.occurrences_of("v");

    match matches.subcommand() {
        ("visibility", Some(submatches)) => visibility::run(submatches, verbosity),
        _ => panic!("unknown subommand"),
    }
    .unwrap_or_else(|e| writeln!(std::io::stderr(), "Error: {}", e).unwrap());
}
