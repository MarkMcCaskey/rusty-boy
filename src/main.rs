//! `RustyBoy` a Gameboy emulator and related tools in rust.
//!
//! # Introduction
//!
//! An interpreter and various debugging tools for the Gameboy
//! This project includes an assembler, disassembler, memory visualization,
//! text-based interactive debugger and language, and standard execution.
//!
//! Memory visualization inspired by [ICU64 / Frodo Redpill v0.1](https://icu64.blogspot.com/2009/09/first-public-release-of-icu64frodo.html)

extern crate app_dirs;
extern crate clap;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;
extern crate log4rs;
#[cfg(feature = "debugger")]
extern crate ncurses;
#[cfg(any(feature = "debugger", feature = "asm"))]
extern crate nom;
extern crate rand; //for channel4 noise sound
extern crate sdl2;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate time;
#[cfg(feature = "vulkan")]
#[macro_use]
extern crate vulkano;
#[cfg(feature = "vulkan")]
#[macro_use]
extern crate vulkano_shader_derive;
#[cfg(feature = "vulkan")]
extern crate vulkano_win;
#[cfg(feature = "vulkan")]
extern crate winit;
#[cfg(feature = "opengl")]
extern crate gl;

/// Simple Gameboy-flavored Z80 assembler
pub mod assembler;

/// The bulk of the hardware emulation
pub mod cpu;

/// Ncurses-based text debugger and parser for debugging language
pub mod debugger;

/// Naive disassembler
pub mod disasm;

/// Functionality for making the Gameboy emulator useful
pub mod io;

use crate::io::applicationsettings::*;
use crate::io::applicationstate::*;

#[allow(unused_variables)]
fn main() {
    let arguments = io::arguments::read_arguments();
    let application_settings = match ApplicationSettings::new(&arguments) {
        Ok(app_settings) => app_settings,
        Err(e) => {
            eprintln!(
                "Fatal error: could not initialize application settings: {}",
                e
            );
            return ();
        }
    };

    // Set up gameboy and app state
    let mut appstate = match ApplicationState::new(application_settings) {
        Ok(apst) => apst,
        Err(e) => {
            eprintln!("Fatal error: could not create Gameboy: {}", e);
            return ();
        }
    };

    use time;
    loop {
        let time_since_last_frame = time::PreciseTime::now();
        appstate.handle_events();
        appstate.step();

        let time_diff = time_since_last_frame.to(time::PreciseTime::now());
        if time_diff < time::Duration::milliseconds(16) {
            std::thread::sleep(
                time::Duration::milliseconds(16 - time_diff.num_milliseconds())
                    .to_std()
                    .unwrap(),
            );
        }
    }
}
