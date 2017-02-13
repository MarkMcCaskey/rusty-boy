//! Graphics utility functions

use sdl2;
use sdl2::pixels::*;
use std::fmt::Debug;

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

/// Dumb gui button
pub struct Toggle<T> {
    pub rect: sdl2::rect::Rect,
    current: usize,
    values: Vec<T>,
}

impl<T> Toggle<T>
    where T: Clone + Debug
{
    pub fn new(rect: sdl2::rect::Rect, values: Vec<T>) -> Toggle<T> {
        Toggle {
            rect: rect,
            current: 0,
            values: values,
        }
    }
    pub fn click(&mut self) {
        self.current = (self.current + 1) % self.values.len();
        debug!("Click! {} {:?}", self.current, self.value());
    }
    pub fn draw(&self, renderer: &mut sdl2::render::Renderer) {
        renderer.set_draw_color(Color::RGB(255, 0, 0));
        renderer.draw_rect(self.rect).unwrap();
    }
    pub fn value(&self) -> Option<T> {
        if !self.values.is_empty() {
            Some(self.values[self.current].clone())
        } else {
            None
        }

    }
}
