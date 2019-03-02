//! Stores all settings related to the application from a user perspective

use app_dirs::*;
use clap::ArgMatches;
use crate::io::constants::{APP_INFO, SCALE};
use std::path::PathBuf;

use log::LevelFilter;
use log4rs;
use log4rs::append::console::ConsoleAppender;
use log4rs::config::{Appender, Config, Root};
use log4rs::encode::pattern::PatternEncoder;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApplicationSettings {
    pub rom_file_name: String,
    pub debug_mode: bool,
    pub trace_mode: bool,
    pub memvis_mode: bool,
    pub debugger_on: bool,
    pub vulkan_mode: bool,
    config_path: Option<PathBuf>,
    pub data_path: Option<PathBuf>,
    pub ui_scale: f32,
}

impl ApplicationSettings {
    pub fn new(arguments: &ArgMatches) -> Result<ApplicationSettings, String> {
        // Attempt to read ROM first
        let rom_file_name = arguments
            .value_of("game")
            .expect("Could not open specified rom")
            .to_string();
        let debug_mode = arguments.is_present("debug");
        let trace_mode = arguments.is_present("trace");
        let memvis_mode = arguments.is_present("visualize");
        let vulkan_mode = arguments.is_present("vulkan");

        // Set up logging
        let stdout = ConsoleAppender::builder()
            .encoder(Box::new(PatternEncoder::new("{h({l})} {m} {n}")))
            .build();

        let config = Config::builder()
            .appender(Appender::builder().build("stdout", Box::new(stdout)))
            .build(
                Root::builder()
                    .appender("stdout")
                    .build(match (trace_mode, debug_mode) {
                        (true, _) => LevelFilter::Trace,
                        (false, true) => LevelFilter::Debug,
                        _ => LevelFilter::Info,
                    }),
            )
            .or_else(|_| Err("Could not build Config".to_string()))?;

        // Set up debugging or command-line logging
        let (should_debugger, _handle) = if debug_mode && cfg!(feature = "debugger") {
            info!("Running in debug mode");
            (true, None)
        } else {
            let handle = log4rs::init_config(config).or_else(|_| Err("Could not init Config"))?;
            (false, Some(handle))
        };

        let data_path = match app_root(AppDataType::UserData, &APP_INFO) {
            Ok(v) => {
                debug!("Using user data path: {:?}", v);
                Some(v)
            }
            Err(e) => {
                error!("Could not open a user data path: {}", e);
                None
            }
        };

        let config_path = match app_root(AppDataType::UserConfig, &APP_INFO) {
            Ok(v) => {
                debug!("Using user config path: {:?}", v);
                Some(v)
            }
            Err(e) => {
                error!("Could not open a user config path: {}", e);
                None
            }
        };

        Ok(ApplicationSettings {
            rom_file_name,
            debug_mode,
            trace_mode,
            memvis_mode,
            vulkan_mode,
            config_path,
            data_path,
            debugger_on: should_debugger,
            //               logger_handle: handle,
            ui_scale: SCALE,
        })
    }
}
