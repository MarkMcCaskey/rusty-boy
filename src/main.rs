//! `RustyBoy` a Gameboy emulator and related tools in rust.
//!
//! This is the entrypoint for the Desktop version of the emulator.
//!
//! # Introduction
//!
//! An interpreter and various debugging tools for the Gameboy
//! This project includes an assembler, disassembler, memory visualization,
//! text-based interactive debugger and language, and standard execution.
//!
//! Memory visualization inspired by [ICU64 / Frodo Redpill v0.1](https://icu64.blogspot.com/2009/09/first-public-release-of-icu64frodo.html)

#[macro_use]
extern crate log;

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

pub mod gba;

pub mod prelude;

use crate::debugger::graphics::Debugger;
use crate::io::applicationsettings::*;
use crate::io::applicationstate::*;
use crate::io::graphics::renderer::EventResponse;
use crate::io::graphics::renderer::Renderer;

#[allow(unused_variables)]
fn main() {
    let arguments = io::arguments::read_arguments();

    // disassembly thanks to Twitch viewer SpawnedArtifact
    if arguments.is_present("disasm") {
        disasm::disasm_main(&arguments);
        std::process::exit(0);
    }

    let application_settings = match ApplicationSettings::new(&arguments) {
        Ok(app_settings) => app_settings,
        Err(e) => {
            eprintln!(
                "Fatal error: could not initialize application settings: {}",
                e
            );
            return;
        }
    };

    use crate::io::dr_sdl2;

    #[cfg(feature = "vulkan")]
    let renderer: Box<Renderer> = if app_settings.vulkan_mode {
        Box::new(graphics::vulkan::VulkanRenderer::new(&app_settings)?)
    } else {
        Box::new(dr_sdl2::Sdl2Renderer::new(&app_settings)?)
    };

    #[cfg(not(feature = "vulkan"))]
    let renderer: Box<dyn Renderer> =
        Box::new(dr_sdl2::Sdl2Renderer::new(&application_settings).expect("Create SDL2 renderer"));

    // Set up gameboy and app state
    let mut appstate = match ApplicationState::new(renderer) {
        Ok(apst) => apst,
        Err(e) => {
            eprintln!("Fatal error: could not create Gameboy: {}", e);
            return;
        }
    };

    trace!("loading ROM");
    let is_gba = application_settings.rom_file_name.ends_with("gba");
    let rom_bytes = {
        use std::fs::File;
        use std::io::Read;

        let mut rom = File::open(application_settings.rom_file_name)
            .map_err(|e| format!("Could not open ROM file: {}", e))
            .unwrap();
        let mut rom_buffer = Vec::with_capacity(0x4000);
        rom.read_to_end(&mut rom_buffer)
            .map_err(|e| format!("Could not read ROM data from file: {}", e))
            .unwrap();
        rom_buffer
    };
    if is_gba {
        info!("GameBoy Advance ROM detected... Attempting to run");
        appstate.gba.load_rom(rom_bytes);
        if std::path::Path::new("../roms/gba_bios.bin").exists() {
            use std::io::Read;
            info!("Loading BIOS");
            let mut bios = std::fs::File::open("../roms/gba_bios.bin").unwrap();
            let mut bios_buffer = Vec::with_capacity(0x4000);
            bios.read_to_end(&mut bios_buffer).unwrap();

            appstate.gba.load_bios(&bios_buffer);
        } else {
            info!("No BIOS found, skipping");
        }
    } else {
        appstate.gameboy.load_rom(rom_bytes);
    }
    //    application_settings.data_path.clone(),

    // delay debugger so loading rom can be logged if need be
    let mut debugger = if application_settings.debugger_on {
        Some(Debugger::new(&appstate.gameboy))
    } else {
        None
    };

    let mut running_frame_counter = 0;
    let mut running_frame_time = std::time::Instant::now();
    if is_gba {
        loop {
            let time_since_last_frame = std::time::Instant::now();
            for event in appstate
                .renderer
                .handle_events(&mut appstate.gba /* , &application_settings*/)
                .iter()
            {
                match *event {
                    EventResponse::ProgramTerminated => {
                        info!("Program exiting!");
                        if let Some(ref mut debugger) = debugger {
                            debugger.die();
                        }
                        /*
                        appstate
                            .gameboy
                            .save_ram(application_settings.data_path.clone());
                        */
                        std::process::exit(0);
                    }
                    EventResponse::Reset => {
                        info!("Resetting gameboy advance");
                        todo!("GBA reset");
                        //appstate.gba.reset();
                    }
                }
            }

            appstate.step_gba();
            /*//check for new controller every frame
            self.load_controller_if_none_exist();*/
            running_frame_counter += 1;
            if running_frame_counter > (60 * 1) {
                let time_diff = running_frame_time.elapsed();
                let fps = (running_frame_counter as f32) / time_diff.as_secs_f32();
                running_frame_counter = 0;
                running_frame_time = std::time::Instant::now();
                appstate.renderer.update_fps(fps);
            }

            let time_diff = time_since_last_frame.elapsed();
            if time_diff < std::time::Duration::from_millis(16) {
                std::thread::sleep(std::time::Duration::from_millis(16) - time_diff);
            }
        }
    }

    loop {
        let time_since_last_frame = std::time::Instant::now();
        for event in appstate
            .renderer
            .handle_events(&mut appstate.gameboy /* , &application_settings*/)
            .iter()
        {
            match *event {
                EventResponse::ProgramTerminated => {
                    info!("Program exiting!");
                    if let Some(ref mut debugger) = debugger {
                        debugger.die();
                    }
                    appstate
                        .gameboy
                        .save_ram(application_settings.data_path.clone());
                    std::process::exit(0);
                }
                EventResponse::Reset => {
                    info!("Resetting gameboy");
                    appstate.gameboy.reset();
                }
            }
        }

        appstate.step();
        if let Some(ref mut dbg) = debugger {
            dbg.step(&mut appstate.gameboy);
        }

        /*//check for new controller every frame
        self.load_controller_if_none_exist();*/

        let time_diff = time_since_last_frame.elapsed();
        if time_diff < std::time::Duration::from_millis(16) {
            std::thread::sleep(std::time::Duration::from_millis(16) - time_diff);
        }
    }
}
