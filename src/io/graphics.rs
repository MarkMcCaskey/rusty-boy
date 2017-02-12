//! Graphics utility functions

use sdl2;
use sdl2::rect::Rect;
use sdl2::rect::Point;
use io::constants::*;
use cpu::constants::MemAddr;
use cpu::*;

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

}
