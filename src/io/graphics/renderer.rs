use cpu::Cpu;
use io::applicationsettings::ApplicationSettings;

#[derive(Debug, Copy, Clone)]
pub enum EventResponse {
    ProgramTerminated,
    Reset,
}

pub trait Renderer {
    fn draw_gameboy(&mut self, &Cpu, &ApplicationSettings);
    fn draw_memory_visualization(&mut self, &Cpu, &ApplicationSettings);
    fn handle_events(&mut self, &mut Cpu, &ApplicationSettings) -> Vec<EventResponse>;
}
