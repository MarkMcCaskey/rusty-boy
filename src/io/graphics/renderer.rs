use crate::cpu::Cpu;
use crate::io::applicationsettings::ApplicationSettings;

#[derive(Debug, Copy, Clone)]
pub enum EventResponse {
    ProgramTerminated,
    Reset,
}

pub trait Renderer {
    fn draw_gameboy(&mut self, _: &Cpu, _: &ApplicationSettings);
    fn draw_memory_visualization(&mut self, _: &Cpu, _: &ApplicationSettings);
    fn handle_events(&mut self, _: &mut Cpu, _: &ApplicationSettings) -> Vec<EventResponse>;
}
