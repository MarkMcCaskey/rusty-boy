//! Graphics utility functions

use sdl2;
use io::constants::*;
use cpu::constants::MemAddr;

/// Saves the current screen to file
pub fn save_screenshot(renderer: &sdl2::render::Renderer, filename: String) {
    let window = renderer.window().unwrap();
    let (w, h) = window.size();
    let format = window.window_pixel_format();
    let mut pixels = renderer.read_pixels(None, format).unwrap();
    let slices = pixels.as_mut_slice();
    let pitch = format.byte_size_of_pixels(w as usize) as u32;
    let masks = format.into_masks().unwrap();
    let surface = sdl2::surface::Surface::from_data_pixelmasks(slices, w, h, pitch, masks).unwrap();
    match surface.save_bmp(filename.clone()) {
        Ok(_) => (),
        Err(_) => error!("Could not save screenshot to {}", filename),
    }
}

/// Returns maybe a memory address given the coordinates of the Gameboy screen?
pub fn screen_coord_to_mem_addr(x: i32, y: i32) -> Option<MemAddr> {
    let x_scaled = ((x as f32) / X_SCALE) as i32;
    let y_scaled = ((y as f32) / Y_SCALE) as i32;
    // FIXME this check is not correct
    if x_scaled < MEM_DISP_WIDTH && y_scaled < MEM_DISP_HEIGHT + 1 {
        Some((x_scaled + y_scaled * MEM_DISP_WIDTH) as u16)
    } else {
        None
    }
}
