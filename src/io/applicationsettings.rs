//! Stores all settings related to the application from a user perspective

use clap::ArgMatches;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApplicationSettings {
    pub rom_file_name: String,
    pub debug_mode: bool,
    pub trace_mode: bool,
    pub memvis_mode: bool,
}

impl ApplicationSettings {
    pub fn new(arguments: &ArgMatches) -> ApplicationSettings {
        // Attempt to read ROM first
        let rom_file_name = arguments.value_of("game").expect("Could not open specified rom").to_string();
        let debug_mode = arguments.is_present("debug");
        let trace_mode = arguments.is_present("trace");
        let memvis_mode = arguments.is_present("visualize");

        ApplicationSettings {
            rom_file_name,
            debug_mode,
            trace_mode,
            memvis_mode,
        }
    }
}
