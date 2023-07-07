#![cfg(not(feature = "desktop"))]

//! This is the entrypoint for the library version of the emulator.
//!
//! This is mostly useful for creating a standalone wasm32-unknown-unknown
//! version of the emulator that can run in a browser. However it could also
//! be used to create a native library that can be linked to from other
//! code if desired.

#[macro_use]
extern crate log;

pub mod cpu;
/// Naive disassembler
pub mod disasm;
pub mod io;

use crate::io::applicationstate::*;
use crate::io::constants::{GB_SCREEN_HEIGHT, GB_SCREEN_WIDTH};
use crate::io::graphics::renderer::Renderer;

extern "C" {
    /// A pointer pointing to exactly 160x144x3 bytes of memory.
    /// RGB values are stored in order, top to bottom, left to right.
    fn draw_frame(frame: *const u8);
    // TODO: figure out what kind of data we need for audio
    //fn output_audio(audio_buffer: &[i16]);
    fn info_message(message: *const u8, length: usize);
    fn error_message(message: *const u8, length: usize);
    fn warn_message(message: *const u8, length: usize);
    fn debug_message(message: *const u8, length: usize);
    fn trace_message(message: *const u8, length: usize);
}

const LOGGER: ExternalLogger = ExternalLogger;

// TODO: change up the way ROM data is passed in
// TODO: add params to specify settings
#[no_mangle]
pub extern "C" fn create_emulator() -> Option<Box<ApplicationState>> {
    log::set_logger(&LOGGER)
        .map(|()| log::set_max_level(log::LevelFilter::Info))
        .unwrap();
    let external_renderer = ExternalRenderer;
    let application_state = match ApplicationState::new(Box::new(external_renderer)) {
        Ok(apst) => apst,
        Err(e) => {
            error!("Fatal error: could not create Gameboy: {}", e);
            return None;
        }
    };
    Some(Box::new(application_state))
}

#[no_mangle]
pub extern "C" fn destroy_emulator(_application_state: Option<Box<ApplicationState>>) {}

#[no_mangle]
pub extern "C" fn step(application_state: &mut ApplicationState) {
    // TODO: handle events
    application_state.step();
}

#[repr(C)]
pub enum ButtonInput {
    A = 0,
    B = 1,
    Start = 2,
    Select = 3,
    Up = 4,
    Down = 5,
    Left = 6,
    Right = 7,
}

#[no_mangle]
pub extern "C" fn press_button(
    application_state: &mut ApplicationState,
    button: ButtonInput,
    pressed: bool,
) {
    match (button, pressed) {
        (ButtonInput::A, true) => application_state.gameboy.press_a(),
        (ButtonInput::A, false) => application_state.gameboy.unpress_a(),
        (ButtonInput::B, true) => application_state.gameboy.press_b(),
        (ButtonInput::B, false) => application_state.gameboy.unpress_b(),
        (ButtonInput::Start, true) => application_state.gameboy.press_start(),
        (ButtonInput::Start, false) => application_state.gameboy.unpress_start(),
        (ButtonInput::Select, true) => application_state.gameboy.press_select(),
        (ButtonInput::Select, false) => application_state.gameboy.unpress_select(),
        (ButtonInput::Up, true) => application_state.gameboy.press_up(),
        (ButtonInput::Up, false) => application_state.gameboy.unpress_up(),
        (ButtonInput::Down, true) => application_state.gameboy.press_down(),
        (ButtonInput::Down, false) => application_state.gameboy.unpress_down(),
        (ButtonInput::Left, true) => application_state.gameboy.press_left(),
        (ButtonInput::Left, false) => application_state.gameboy.unpress_left(),
        (ButtonInput::Right, true) => application_state.gameboy.press_right(),
        (ButtonInput::Right, false) => application_state.gameboy.unpress_right(),
    }
}

#[no_mangle]
pub extern "C" fn reset(application_state: &mut ApplicationState) {
    application_state.gameboy.reset();
}

/// Load a new ROM into the emulator.
///
/// # Safety
/// Rom_data_ptr must point to a valid slice of memory that is at least rom_data_len bytes long.
#[no_mangle]
pub unsafe extern "C" fn load_rom(
    application_state: &mut ApplicationState,
    rom_data_ptr: *const u8,
    rom_data_len: usize,
) {
    let rom_data = std::slice::from_raw_parts(rom_data_ptr, rom_data_len).to_owned();
    application_state.gameboy.load_rom(rom_data);
}

#[repr(C)]
pub struct ExternalRenderer;

impl Renderer for ExternalRenderer {
    fn draw_frame(&mut self, frame: &[[(u8, u8, u8); GB_SCREEN_WIDTH]; GB_SCREEN_HEIGHT]) {
        let buffer = frame
            .iter()
            .flat_map(|row| row.iter().flat_map(|(r, g, b)| vec![*r, *g, *b]))
            .collect::<Vec<u8>>();
        unsafe {
            draw_frame(buffer.as_ptr());
        }
    }

    fn handle_events(
        &mut self,
        _: &mut crate::cpu::Cpu,
    ) -> Vec<crate::io::graphics::renderer::EventResponse> {
        // there is no need to do anything here
        // TODO: look into restructing the code so this trait isn't required
        vec![]
    }
}

use log::{Level, Metadata, Record};

struct ExternalLogger;

impl log::Log for ExternalLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let message = format!("{}", record.args());
            let bytes = message.as_bytes();
            match record.level() {
                Level::Error => unsafe { error_message(bytes.as_ptr(), bytes.len()) },
                Level::Warn => unsafe { warn_message(bytes.as_ptr(), bytes.len()) },
                Level::Info => unsafe { info_message(bytes.as_ptr(), bytes.len()) },
                Level::Debug => unsafe { debug_message(bytes.as_ptr(), bytes.len()) },
                Level::Trace => unsafe { trace_message(bytes.as_ptr(), bytes.len()) },
            }
        }
    }

    fn flush(&self) {}
}

#[no_mangle]
extern "C" fn allocate_bytes(num_bytes: usize) -> *mut u8 {
    let bytes = vec![0; num_bytes];
    let mut byte_slice: Box<[u8]> = bytes.into_boxed_slice();
    let ptr: *mut u8 = byte_slice.as_mut_ptr();
    std::mem::forget(byte_slice);
    ptr
}

#[no_mangle]
unsafe extern "C" fn free_bytes(ptr: *mut u8, num_bytes: usize) {
    let _bytes: Vec<u8> = Vec::from_raw_parts(ptr, num_bytes, num_bytes);
}
