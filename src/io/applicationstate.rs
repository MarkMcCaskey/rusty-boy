//! The wrapper around the information needed to meaningfully run this program
//!
//! NOTE: in the process of further abstracting IO logic with this --
//! expect things to break

use std;

use debugger::graphics::*;
use cpu;
use io::constants::*;
use io::input::*;

use io::sound::*;
use io::applicationsettings::ApplicationSettings;
use io::graphics::renderer::Renderer;
use io::graphics;


use std::num::Wrapping;


/// Holds all the data needed to use the emulator in meaningful ways
pub struct ApplicationState {
    pub gameboy: cpu::Cpu,
    //sound_system: AudioDevice<GBSound>,
    //renderer: render::Renderer<'static>,
    cycle_count: u64,
    prev_time: u64,
    /// counts cycles for hsync updates
    prev_hsync_cycles: u64,
    /// counts cycles since last timer update
    timer_cycles: u64,
    /// counts cycles since last divider register update
    div_timer_cycles: u64,
    /// counts cycles since last sound update
    sound_cycles: u64,
    debugger: Option<Debugger>,
    initial_gameboy_state: cpu::Cpu,
    //    logger_handle: Option<log4rs::Handle>, // storing to keep alive
    screenshot_frame_num: Wrapping<u64>,
    //ui_offset: Point, // TODO whole interface pan
    application_settings: ApplicationSettings,
    renderer: Box<Renderer>,
//    texture_creator: TextureCreator<WindowContext>,
}

impl ApplicationState {
    //! Sets up the environment for running in memory visualization mode
    pub fn new(app_settings: ApplicationSettings) -> Result<ApplicationState, String> {

        // Set up gameboy and other state
        let mut gameboy = cpu::Cpu::new();
        trace!("loading ROM");
        gameboy.load_rom(app_settings.rom_file_name.as_ref(),
                         app_settings.data_path.clone());

        // delay debugger so loading rom can be logged if need be
        let debugger = if app_settings.debugger_on {
            Some(Debugger::new(&gameboy))
        } else {
            None
        };

        let renderer: Box<Renderer> = if app_settings.vulkan_mode {
            Box::new(graphics::vulkan::VulkanRenderer::new(&app_settings)?)
        } else {
            Box::new(graphics::sdl2::Sdl2Renderer::new(&app_settings)?)
        };


        let gbcopy = gameboy.clone();

        // let txt_format = sdl2::pixels::PixelFormatEnum::RGBA8888;
        //let w = MEM_DISP_WIDTH as u32;
        //let h = MEM_DISP_HEIGHT as u32;
        //let memvis_texture = tc.create_texture_static(txt_format, w, h).unwrap();


        Ok(ApplicationState {
               gameboy: gameboy,
               //sound_system: device,
               cycle_count: 0,
               prev_time: 0,
               // FIXME sound_cycles is probably wrong or not needed
               sound_cycles: 0,
               debugger: debugger,
               prev_hsync_cycles: 0,
               timer_cycles: 0,
               div_timer_cycles: 0,
               initial_gameboy_state: gbcopy,
               //logger_handle: handle,
               screenshot_frame_num: Wrapping(0),
               //ui_offset: Point::new(0, 0),
               application_settings: app_settings,
               renderer: renderer,
            //texture_creator: tc,
           })
    }


    /// Handles both controller input and keyboard/mouse debug input
    /// NOTE: does not handle input for ncurses debugger
    //this should be properly abstracted... allow for rebinding too
    pub fn handle_events(&mut self) {
        use self::graphics::renderer::EventResponse;
        for event in self.renderer
                .handle_events(&mut self.gameboy, &self.application_settings)
                .iter() {
            match *event {
                EventResponse::ProgramTerminated => {
                    info!("Program exiting!");
                    if let Some(ref mut debugger) = self.debugger {
                        debugger.die();
                    }
                    self.gameboy
                        .save_ram(self.application_settings.data_path.clone());
                    std::process::exit(0);
                }
                _ => unimplemented!(),
            }
        }
    }

    /// Runs the game application forward one "unit of time"
    /// Attepmts to load a controller if it can find one every time a frame is drawn
    /// TODO: elaborate
    pub fn step(&mut self) {
        //TODO optimize here (quite a bit; need to reduce branches and
        // allow for more consecutive instructions to be executed)
        'steploop: loop {
            let current_op_time = if self.gameboy.state != cpu::constants::CpuState::Crashed {
                self.gameboy.dispatch_opcode() as u64
            } else {
                10 // FIXME think about what to return here or refactor code around this
            };

            self.cycle_count += current_op_time;

            // FF04 (DIV) Divider Register stepping
            self.div_timer_cycles += current_op_time;
            if self.div_timer_cycles >= CPU_CYCLES_PER_DIVIDER_STEP {
                self.gameboy.inc_div();
                self.div_timer_cycles -= CPU_CYCLES_PER_DIVIDER_STEP;
            }

            // FF05 (TIMA) Timer counter stepping
            self.timer_cycles += current_op_time;
            let timer_hz = self.gameboy.timer_frequency_hz();
            let cpu_cycles_per_timer_counter_step =
                (CPU_CYCLES_PER_SECOND as f64 / (timer_hz as f64)) as u64;
            if self.timer_cycles >= cpu_cycles_per_timer_counter_step {
                //           std::thread::sleep_ms(16);
                // trace!("Incrementing the timer!");
                self.gameboy.timer_cycle();
                self.timer_cycles -= cpu_cycles_per_timer_counter_step;
            }

            // Faking hsync to get the games running
            let fake_display_hsync = true;
            if fake_display_hsync {
                // update LY respective to cycles spent execing instruction
                let cycle_count = self.cycle_count;
                loop {
                    if cycle_count < self.prev_hsync_cycles {
                        break;
                    }
                    self.gameboy.inc_ly();
                    self.prev_hsync_cycles += CYCLES_PER_HSYNC;
                }
            }

            if (self.cycle_count - self.prev_time) >= CPU_CYCLES_PER_VBLANK {

                /*//check for new controller every frame
                self.load_controller_if_none_exist();*/

                if let Some(ref mut dbg) = self.debugger {
                    dbg.step(&mut self.gameboy);
                }

                let cycle_count = self.cycle_count;
                self.prev_time = cycle_count;

                self.renderer
                    .draw_gameboy(&self.gameboy, &self.application_settings);
                //for memory visualization
                self.gameboy.remove_old_events();
                break 'steploop;
            }

        }
    }
}
