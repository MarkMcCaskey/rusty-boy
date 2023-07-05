use crate::cpu::Cpu;
use crate::io::applicationsettings::ApplicationSettings;
use crate::io::constants::{GB_SCREEN_HEIGHT, GB_SCREEN_WIDTH};

#[derive(Debug, Copy, Clone)]
pub enum EventResponse {
    ProgramTerminated,
    Reset,
}

pub trait Renderer {
    fn draw_frame(&mut self, frame: &[[u8; GB_SCREEN_WIDTH]; GB_SCREEN_HEIGHT]);
    fn draw_gameboy(&mut self, _: &mut Cpu, _: &ApplicationSettings) -> usize;
    fn draw_memory_visualization(&mut self, _: &Cpu, _: &ApplicationSettings);
    fn handle_events(&mut self, _: &mut Cpu, _: &ApplicationSettings) -> Vec<EventResponse>;
}
