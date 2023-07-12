//! Command line arguments
//! Used to change behavior when launching

use clap::{App, Arg, ArgMatches};

// Parses command line arguments
pub fn read_arguments<'input>() -> ArgMatches<'input> {
    #[allow(unused_mut)]
    let mut app_builder = App::new("rusty-boy")
        .version("-0.1")
        .author("Mark McCaskey, spawnedartifact, and friends")
        .about("Kappa")
        .arg(
            Arg::with_name("game")
                .short("g")
                .long("game")
                .value_name("FILE")
                .help("Specifies ROM to load")
                .required(true)
                .index(1)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("trace")
                .short("t")
                .multiple(true)
                .long("trace")
                .help("Runs with verbose trace")
                .takes_value(false),
        )
        .arg(
            Arg::with_name("debug")
                .multiple(true)
                .long("debug")
                .help("Runs with debug logging on")
                .takes_value(false),
        )
        .arg(
            Arg::with_name("visualize")
                .short("z")
                .long("visualize")
                .help("Turns on interactive memory visualization")
                .takes_value(false),
        )
        .arg(
            Arg::with_name("disasm")
                .long("disasm")
                .help("Disassemble a ROM, print it, and exit")
                .takes_value(false),
        )
        .arg(
            Arg::with_name("remove-nops")
                .long("remove-nops")
                .help("Don't show NOPs in disassembly output (disassembler only)")
                .takes_value(false),
        );

    #[cfg(feature = "debugger")]
    {
        app_builder = app_builder.arg(
            Arg::with_name("debug")
                .short("d")
                .multiple(true)
                .long("debug")
                .help("Runs ncurses debugger in the background")
                .takes_value(false),
        );
    }

    #[cfg(feature = "vulkan")]
    {
        app_builder = app_builder.arg(
            Arg::with_name("vulkan")
                .short("v")
                .long("vulkan")
                .help("Runs graphics on the GPU with Vulkan")
                .takes_value(false),
        );
    }

    app_builder.get_matches()
}
