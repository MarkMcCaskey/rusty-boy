//! Graphics utility functions

use sdl2;
use sdl2::pixels::*;
use std::fmt::Debug;
use sdl2::rect::{Rect, Point};

use cpu::Cpu;

/// Saves the current screen to file
pub fn save_screenshot(renderer: &sdl2::render::Renderer, filename: &str) {
    let window = renderer.window().unwrap();
    let (w, h) = window.size();
    let format = window.window_pixel_format();
    let mut pixels = renderer.read_pixels(None, format).unwrap();
    let slices = pixels.as_mut_slice();
    let pitch = format.byte_size_of_pixels(w as usize) as u32;
    let masks = format.into_masks().unwrap();
    let surface = sdl2::surface::Surface::from_data_pixelmasks(slices, w, h, pitch, masks).unwrap();

    match surface.save_bmp(filename) {
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


/// Simple "window" inside of main gui window. Contains Drawable and
/// is Drawable itself. It moves contained drawable `vis` to new
/// position on screen given by `rect`. Sets clipping and viewport
/// before calling draw() of `vis` and resets it after.
pub struct PositionedFrame {
    /// Position and size
    pub rect: Rect,
    pub scale: f32,
    pub vis: Box<Drawable>,
}

impl PositionedFrame {
    #[inline]
    fn before_draw(&self, renderer: &mut sdl2::render::Renderer) {
        let r = self.rect;

        let view_rect = r;
        let clip_rect = r;

        renderer.set_clip_rect(Some(clip_rect));
        renderer.set_viewport(Some(view_rect));
    }

    #[inline]
    fn after_draw(&self, renderer: &mut sdl2::render::Renderer) {
        let (s_x, s_y) = renderer.scale();
        renderer.set_scale(s_x / self.scale, s_y / self.scale).unwrap();
        renderer.set_clip_rect(None);
        renderer.set_viewport(None);
    }
}


pub trait Drawable {
    fn get_initial_size(&self) -> (u32, u32);
    fn draw(&mut self, renderer: &mut sdl2::render::Renderer, cpu: &mut Cpu);
    fn click(&mut self, button: sdl2::mouse::MouseButton, position: Point, cpu: &mut Cpu);
}


impl Drawable for PositionedFrame {
    fn get_initial_size(&self) -> (u32, u32) {
        self.vis.get_initial_size()
    }

    fn draw(&mut self, renderer: &mut sdl2::render::Renderer, cpu: &mut Cpu) {
        self.before_draw(renderer);
        // draw_frame_bounds(self, renderer); // Use to debug clipping
        self.vis.draw(renderer, cpu);
        self.after_draw(renderer);
    }

    fn click(&mut self, button: sdl2::mouse::MouseButton, position: Point, cpu: &mut Cpu) {
        let rel_point = position - self.rect.top_left();
        debug!("Clicked at relative {:?} with {:?}", rel_point, button);
        self.vis.click(button, rel_point, cpu);
    }
}


pub fn draw_frame_bounds(frame: &PositionedFrame, renderer: &mut sdl2::render::Renderer) {
    renderer.set_draw_color(Color::RGB(0, 0, 0));
    renderer.fill_rect(Rect::new(0, 0, frame.rect.width(), frame.rect.height())).unwrap();

    renderer.set_draw_color(Color::RGB(255, 0, 255));
    renderer.fill_rect(frame.rect).unwrap();
    renderer.set_draw_color(Color::RGB(0, 0, 0));
    renderer.draw_line(frame.rect.top_left(), frame.rect.bottom_right()).unwrap();
    renderer.draw_line(frame.rect.top_right(), frame.rect.bottom_left()).unwrap();

    renderer.set_draw_color(Color::RGB(255, 255, 255));
    let w = frame.rect.width();
    let h = frame.rect.height();
    renderer.draw_line(Point::new(0, 0), Point::new(w as i32, h as i32)).unwrap();
    renderer.draw_line(Point::new(w as i32, 0), Point::new(0, h as i32)).unwrap();
}
