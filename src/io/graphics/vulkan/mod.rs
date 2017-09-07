use io::graphics::renderer::Renderer;
use io::applicationsettings::ApplicationSettings;
use cpu::Cpu;
use super::renderer;

pub struct VulkanRenderer {}

impl VulkanRenderer {
    pub fn new(app_settings: &ApplicationSettings) -> Result<Self, String> {
        unimplemented!()
    }
}

impl Renderer for VulkanRenderer {
    fn draw_gameboy(&mut self, gameboy: &Cpu, app_settings: &ApplicationSettings) {
        unimplemented!();
    }

    fn draw_memory_visualization(&mut self, gameboy: &Cpu, app_settings: &ApplicationSettings) {
        unimplemented!();
    }

    fn handle_events(&mut self,
                     gameboy: &mut Cpu,
                     app_settings: &ApplicationSettings)
                     -> Vec<renderer::EventResponse> {
        unimplemented!();
    }
}
