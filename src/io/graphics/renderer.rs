use crate::cpu::Cpu;
use crate::io::constants::{GB_SCREEN_HEIGHT, GB_SCREEN_WIDTH};

#[derive(Debug, Copy, Clone)]
pub enum EventResponse {
    ProgramTerminated,
    Reset,
}

pub trait Renderer {
    fn draw_frame(&mut self, frame: &[[(u8, u8, u8); GB_SCREEN_WIDTH]; GB_SCREEN_HEIGHT]);
    // TOOD: readd important data to args here later
    fn draw_memory_visualization(&mut self, _: &Cpu) {
        unimplemented!();
    }
    fn handle_events(&mut self, _: &mut Cpu) -> Vec<EventResponse>;
}
