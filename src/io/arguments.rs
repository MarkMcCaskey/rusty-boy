//! Command line arguments
//! Used to change behavior when launching

use clap::{Arg, App, ArgMatches};

// Parses command line arguments
pub fn read_arguments<'input>() -> ArgMatches<'input> {
    App::new("rusty-boy")
        .version("-0.1")
        .author("Mark McCaskey, spawnedartifact, and friends")
        .about("Kappa")
        .arg(Arg::with_name("game")
            .short("g")
            .long("game")
            .value_name("FILE")
            .help("Specifies ROM to load")
            .required(true)
            .index(1)
            .takes_value(true))
        .arg(Arg::with_name("debug")
            .short("d")
            .multiple(true)
            .long("debug")
            .help("Runs ncurses debugger in the background")
            .takes_value(false))
        .arg(Arg::with_name("trace")
            .short("t")
            .multiple(true)
            .long("trace")
            .help("Runs with verbose trace")
            .takes_value(false))
        .arg(Arg::with_name("visualize")
             .short("z")
             .long("visualize")
             .help("Turns on interactive memory visualization")
             .takes_value(false))
        .get_matches()
}
