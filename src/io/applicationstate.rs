use std;

use sdl2::*;
use sdl2::audio::AudioDevice;
use sdl2::keyboard::Keycode;
use log4rs;

use debugger::graphics::*;
use cpu;
use io::constants::*;
use io::input::*;
use io::graphics::*;
use io::memvis;
use io::memvis::MemVisState;
use io::sound::*;
use io::vidram;

use log::LogLevelFilter;
use log4rs::append::console::ConsoleAppender;
use log4rs::encode::pattern::PatternEncoder;
use log4rs::config::{Appender, Config, Root};

use sdl2;
use sdl2::rect::{Point, Rect};

use std::num::Wrapping;

/// Holds all the data needed to use the emulator in meaningful ways
pub struct ApplicationState {
    pub gameboy: cpu::Cpu,
    mem_vis_state: MemVisState,
    sdl_context: Sdl, //  sdl_sound: sdl2::audio,
    sound_system: AudioDevice<SquareWave>,
    renderer: render::Renderer<'static>,
    cycle_count: u64,
    prev_time: u64,
    debugger: Option<Debugger>,
    prev_hsync_cycles: u64,
    clock_cycles: u64,
    initial_gameboy_state: cpu::Cpu,
    logger_handle: Option<log4rs::Handle>,
    controller: Option<sdl2::controller::GameController>,
    screenshot_frame_num: Wrapping<u64>,
}



impl ApplicationState {
    pub fn new(trace_mode: bool, debug_mode: bool, rom_file_name: &str) -> ApplicationState {
        // Set up logging
        let stdout = ConsoleAppender::builder()
            .encoder(Box::new(PatternEncoder::new("{h({l})} {m} {n}")))
            .build();

        let config = Config::builder()
            .appender(Appender::builder().build("stdout", Box::new(stdout)))
            .build(Root::builder().appender("stdout").build(if trace_mode {
                LogLevelFilter::Trace
            } else {
                LogLevelFilter::Debug
            }))
            .unwrap();

        // Set up debugging or command-line logging
        let (debugger, handle) = if debug_mode {
            info!("Running in debug mode");
            // Some(Debugger::new(&mut gameboy))
            (None, None)
        } else {
            let handle = log4rs::init_config(config).unwrap();
            (None, Some(handle))
        };


        // Set up gameboy and other state
        let mut gameboy = cpu::Cpu::new();
        let mem_vis_state = MemVisState::new();

        trace!("loading ROM");
        gameboy.load_rom(rom_file_name);

        let sdl_context = sdl2::init().unwrap();
        let device = setup_audio(&sdl_context);
        let controller = setup_controller_subsystem(&sdl_context);

        // Set up graphics and window
        trace!("Opening window");
        let video_subsystem = sdl_context.video().unwrap();
        let window = video_subsystem.window(gameboy.get_game_name().as_str(),
                    RB_SCREEN_WIDTH,
                    RB_SCREEN_HEIGHT)
            .position_centered()
            .build()
            .unwrap();

        let renderer = window.renderer()
            .accelerated()
            .build()
            .unwrap();



        let gbcopy = gameboy.clone();

        ApplicationState {
            gameboy: gameboy,
            mem_vis_state: mem_vis_state,
            sdl_context: sdl_context,
            sound_system: device,
            renderer: renderer,
            cycle_count: 0,
            prev_time: 0,
            debugger: debugger,
            prev_hsync_cycles: 0,
            clock_cycles: 0,
            initial_gameboy_state: gbcopy,
            logger_handle: handle,
            controller: controller,
            screenshot_frame_num: Wrapping(0),
        }
    }




    pub fn handle_events(&mut self) {
        for event in self.sdl_context.event_pump().unwrap().poll_iter() {
            use sdl2::event::Event;

            match event {
                Event::ControllerAxisMotion { axis, value: val, .. } => {
                    let dead_zone = 10000;
                    if val > dead_zone || val < -dead_zone {
                        debug!("Axis {:?} moved to {}", axis, val);
                        //                   match axis {
                        // controller::Axis::LeftX =>,
                        // controller::Axis::LeftY =>,
                        // _ => (),
                        // }
                        //
                    }
                }
                Event::ControllerButtonDown { button, .. } => {
                    debug!("Button {:?} down", button);
                    match button {
                        controller::Button::A => {
                            self.gameboy.press_a();
                            // TODO: sound
                            // device.resume();
                        }
                        controller::Button::B => self.gameboy.press_b(),
                        controller::Button::Back => self.gameboy.press_select(),
                        controller::Button::Start => self.gameboy.press_start(),
                        _ => (),
                    }
                }

                Event::ControllerButtonUp { button, .. } => {
                    debug!("Button {:?} up", button);
                    match button {
                        controller::Button::A => {
                            self.gameboy.unpress_a();
                            // TODO: sound
                            // device.pause();
                        }
                        controller::Button::B => self.gameboy.unpress_b(),
                        controller::Button::Back => self.gameboy.unpress_select(),
                        controller::Button::Start => self.gameboy.unpress_start(),
                        _ => (),
                    }
                }
                Event::Quit { .. } => {
                    info!("Program exiting!");
                    std::process::exit(0);
                }
                Event::KeyDown { keycode: Some(keycode), .. } => {
                    match keycode {
                        Keycode::Escape => {
                            info!("Program exiting!");
                            std::process::exit(0);
                        }
                        Keycode::F3 => self.gameboy.toggle_logger(),
                        Keycode::R => {
                            // Reset/reload emu
                            // TODO Keep previous visualization settings
                            self.gameboy.reset();
                            let gbcopy = self.initial_gameboy_state.clone();
                            self.gameboy = gbcopy;

                            // // This way makes it possible to edit rom
                            // // with external editor and see changes
                            // // instantly.
                            // gameboy = Cpu::new();
                            // gameboy.load_rom(rom_file);
                        }
                        _ => (),
                    }
                }
                Event::MouseButtonDown { x, y, mouse_btn, .. } => {
                    match mouse_btn {
                        sdl2::mouse::MouseButton::Left => {
                            // Print info about clicked address
                            let scaled_x = x / self.mem_vis_state.scale as i32;
                            let scaled_y = y / self.mem_vis_state.scale as i32;
                            memvis::memvis_handle_click(&self.gameboy, scaled_x, scaled_y);

                            // Switch Tile Map manually
                            let point = Point::new(scaled_x, scaled_y);
                            if self.mem_vis_state.tile_data_mode_button.rect.contains(point) {
                                self.mem_vis_state.tile_data_mode_button.click();
                            }
                        }
                        sdl2::mouse::MouseButton::Right => {
                            // Jump to clicked addr and bring cpu back to lifetimes
                            let scaled_x = x / self.mem_vis_state.scale as i32;
                            let scaled_y = y / self.mem_vis_state.scale as i32;

                            if let Some(pc) = memvis::screen_coord_to_mem_addr(scaled_x, scaled_y) {
                                info!("Jumping to ${:04X}", pc);
                                self.gameboy.pc = pc;
                                if self.gameboy.state != cpu::constants::CpuState::Normal {
                                    info!("CPU was '{:?}', forcing run.", self.gameboy.state);
                                    self.gameboy.state = cpu::constants::CpuState::Normal;
                                }
                            }

                        }
                        _ => (),
                    }
                }
                Event::MouseWheel { y, .. } => {
                    self.mem_vis_state.scale += y as f32;
                }
                _ => (),
            }
        }
    }

    /// Runs the game application forward one "unit of time"
    /// TODO: elaborate
    pub fn step(&mut self) {

        // handle_events(&mut sdl_context, &mut gameboy);

        let current_op_time = if self.gameboy.state != cpu::constants::CpuState::Crashed {
            self.gameboy.dispatch_opcode() as u64
        } else {
            10 // FIXME think about what to return here or refactor code around this
        };

        self.cycle_count += current_op_time;
        self.clock_cycles += current_op_time;
        let timer_khz = self.gameboy.timer_frequency();
        let time_in_ms_per_cycle = (1000.0 / ((timer_khz as f64) * 1000.0)) as u64;
        self.clock_cycles += self.cycle_count;

        // TODO: remove prev_time
        let prev_time = self.prev_time;
        let ticks = self.cycle_count - prev_time;

        let time_in_cpu_cycle_per_cycle =
            ((time_in_ms_per_cycle as f64) / (1.0 / (4.19 * 1000.0 * 1000.0))) as u64;

        if self.clock_cycles >= time_in_cpu_cycle_per_cycle {
            //           std::thread::sleep_ms(16);
            // trace!("Incrementing the timer!");
            self.gameboy.timer_cycle();
            self.clock_cycles = 0;
        }

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

        // Gameboy screen is 256x256
        // only 160x144 are displayed at a time
        //
        // Background tile map is 32x32 of tiles. Scrollx and scrolly
        // determine how this is actually rendered (it wraps)
        // These numbers index the tile data table
        //

        // 16384hz, call inc_div
        // CPU is at 4.194304MHz (or 1.05MHz) 105000000hz
        // hsync at 9198KHz = 9198000hz
        // vsync at 59.73Hz


        let scale = self.mem_vis_state.scale;
        match self.renderer.set_scale(scale, scale) {
            Ok(_) => (),
            Err(_) => error!("Could not set render scale"),
        }

        // 1ms before drawing in terms of CPU time we must throw a vblank interrupt
        // TODO make this variable based on whether it's GB, SGB, etc.

        if ticks >= CPU_CYCLES_PER_VBLANK {
            if let Some(ref mut dbg) = self.debugger {
                dbg.step(&mut self.gameboy);
            }

            let cycle_count = self.cycle_count;
            self.prev_time = cycle_count;
            self.renderer.set_draw_color(NICER_COLOR);
            self.renderer.clear();

            let tile_patterns_x_offset = (MEM_DISP_WIDTH + SCREEN_BUFFER_SIZE_X as i32) as i32 + 4;
            vidram::draw_tile_patterns(&mut self.renderer, &self.gameboy, tile_patterns_x_offset);

            // TODO add toggle for this also?
            let tile_map_offset = TILE_MAP_1_START;

            let bg_select = self.mem_vis_state.tile_data_mode_button.value().unwrap();

            let tile_patterns_offset = match bg_select {
                TileDataSelect::Auto => {
                    if self.gameboy.lcdc_bg_tile_map() {
                        TILE_PATTERN_TABLE_1_ORIGIN
                    } else {
                        TILE_PATTERN_TABLE_2_ORIGIN
                    }
                }
                TileDataSelect::Mode1 => TILE_PATTERN_TABLE_1_ORIGIN,
                TileDataSelect::Mode2 => TILE_PATTERN_TABLE_2_ORIGIN,
            };


            let bg_disp_x_offset = MEM_DISP_WIDTH + 2;

            vidram::draw_background_buffer(&mut self.renderer,
                                           &self.gameboy,
                                           tile_map_offset,
                                           tile_patterns_offset,
                                           bg_disp_x_offset);

            if self.mem_vis_state.mem_val_display_enabled {
                // // dynamic mem access vis
                // memvis::draw_memory_values(&mut renderer, &gameboy);
                memvis::draw_memory_access(&mut self.renderer, &self.gameboy);

                memvis::draw_memory_events(&mut self.renderer, &mut self.gameboy);
            }


            self.mem_vis_state.tile_data_mode_button.draw(&mut self.renderer);


            //   00111100 1110001 00001000
            //   01111110 1110001 00010100
            //   11111111 1110001 00101010
            //

            // TODO add a way to enable/disable this while running
            let record_screen = false;
            if record_screen {
                save_screenshot(&self.renderer,
                                format!("screen{:010}.bmp",
                                        self.screenshot_frame_num.0));
                self.screenshot_frame_num += Wrapping(1);
            }

            if self.gameboy.get_sound1() {
                self.sound_system.resume();
            } else {
                self.sound_system.pause();
            }

            let mut sound_system = self.sound_system.lock();
            sound_system.wave_duty = self.gameboy.channel1_wave_pattern_duty();
            sound_system.phase_inc = 1.0 /
                                     (131072.0 / (2048 - self.gameboy.channel1_frequency()) as f32);
            sound_system.add = self.gameboy.channel1_sweep_increase();
            //            131072 / (2048 - gb)


            self.renderer.present();
        }
    }
}
