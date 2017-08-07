//! `RustyBoy` a Gameboy emulator and related tools in rust.
//!
//! # Introduction
//!
//! An interpreter and various debugging tools for the Gameboy
//! This project includes an assembler, disassembler, memory visualization,
//! text-based interactive debugger and language, and standard execution.
//!
//! Memory visualization inspired by [ICU64 / Frodo Redpill v0.1](https://icu64.blogspot.com/2009/09/first-public-release-of-icu64frodo.html)

extern crate clap;
#[macro_use]
extern crate log;
extern crate log4rs;
extern crate sdl2;
extern crate ncurses;
extern crate rand; //for channel4 noise sound
extern crate serde;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate nom;
extern crate app_dirs;
extern crate time;
#[macro_use]
extern crate lazy_static;

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


use io::applicationstate::*;
use io::applicationsettings::*;


#[allow(unused_variables)]
fn main() {
    let arguments = io::arguments::read_arguments();
    let application_settings = ApplicationSettings::new(&arguments);


    // Set up gameboy and app state
    let mut appstate = match ApplicationState::new(&application_settings) {
        Ok(apst) => apst,
        Err(e) => {
            eprintln!("Fatal error: could not create Gameboy: {}", e);
            return ();
        }
    };

    loop {
        appstate.handle_events();
        appstate.step();
    }
}
