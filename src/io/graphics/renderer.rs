use crate::cpu::Cpu;
use crate::io::constants::{
    GBA_SCREEN_HEIGHT, GBA_SCREEN_WIDTH, GB_SCREEN_HEIGHT, GB_SCREEN_WIDTH,
};

#[derive(Debug, Copy, Clone)]
pub enum EventResponse {
    ProgramTerminated,
    Reset,
}

pub trait Renderer {
    fn draw_frame(&mut self, frame: &[[(u8, u8, u8); GB_SCREEN_WIDTH]; GB_SCREEN_HEIGHT]);
    fn draw_gba_frame(&mut self, frame: &[[(u8, u8, u8); GBA_SCREEN_WIDTH]; GBA_SCREEN_HEIGHT]) {
        unimplemented!("No GBA support");
    }
    // TOOD: readd important data to args here later
    fn draw_memory_visualization(&mut self, _: &Cpu) {
        unimplemented!();
    }
    fn handle_events(&mut self, _: &mut Cpu) -> Vec<EventResponse>;

    fn audio_step(&mut self, _gb: &Cpu) {
        unimplemented!();
    }
}
