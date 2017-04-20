//! The wrapper around the information needed to meaningfully run this program
//!
//! NOTE: in the process of further abstracting IO logic with this --
//! expect things to break

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
use io::memvis::MemVisState;
use io::vidram::{VidRamBGDisplay, VidRamTileDisplay};
use io::sound::*;

use log::LogLevelFilter;
use log4rs::append::console::ConsoleAppender;
use log4rs::encode::pattern::PatternEncoder;
use log4rs::config::{Appender, Config, Root};

use sdl2;
use sdl2::rect::{Point, Rect};

use std::num::Wrapping;

/// Holds all the data needed to use the emulator in meaningful ways
#[allow(dead_code)] //suppress pointers to things to keep them alive,
// could almost certainly be done in a better way
pub struct ApplicationState {
    pub gameboy: cpu::Cpu,
    sdl_context: Sdl, //  sdl_sound: sdl2::audio,
    sound_system: AudioDevice<GBSound>,
    renderer: render::Renderer<'static>,
    cycle_count: u64,
    prev_time: u64,
    debugger: Option<Debugger>,
    /// counts cycles for hsync updates
    prev_hsync_cycles: u64,
    /// counts cycles since last timer update
    timer_cycles: u64,
    /// counts cycles since last divider register update
    div_timer_cycles: u64,
    /// counts cycles since last sound update
    sound_cycles: u64,
    initial_gameboy_state: cpu::Cpu,
    logger_handle: Option<log4rs::Handle>, // storing to keep alive
    controller: Option<sdl2::controller::GameController>, // storing to keep alive
    screenshot_frame_num: Wrapping<u64>,
    ui_scale: f32,
    ui_offset: Point, // TODO whole interface pan
    widgets: Vec<PositionedFrame>,
}



impl ApplicationState {
    //! Sets up the environment for running in memory visualization mode
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
                                                                LogLevelFilter::Info
                                                            }))
            .unwrap();


        // Set up debugging or command-line logging
        let (should_debugger, handle) = if debug_mode && cfg!(feature = "debugger") {
            info!("Running in debug mode");
            (true, None)
        } else {
            let handle = log4rs::init_config(config).unwrap();
            (false, Some(handle))
        };


        // Set up gameboy and other state
        let mut gameboy = cpu::Cpu::new();
        trace!("loading ROM");
        gameboy.load_rom(rom_file_name);


        //delay debugger so loading rom can be logged if need be
        let debugger = if should_debugger {
            Some(Debugger::new(&gameboy))
        } else {
            None
        };

        let sdl_context = sdl2::init().unwrap();
        let device = setup_audio(&sdl_context);
        let controller = setup_controller_subsystem(&sdl_context);

        // Set up graphics and window
        trace!("Opening window");
        let video_subsystem = sdl_context.video().unwrap();
        let window = match video_subsystem.window(gameboy.get_game_name().as_str(),
                                     RB_SCREEN_WIDTH,
                                     RB_SCREEN_HEIGHT)
                  .position_centered()
                  .opengl()
                  .build() {
            Ok(v) => v,
            Err(e) => panic!("Fatal error: {}", e),
        };

        //video_subsystem.gl_load_library_default();

        let renderer = window.renderer()
            .accelerated()
            .build()
            .unwrap();

        let gbcopy = gameboy.clone();

        let txt_format = sdl2::pixels::PixelFormatEnum::RGBA8888;
        let w = MEM_DISP_WIDTH as u32;
        let h = MEM_DISP_HEIGHT as u32;
        let memvis_texture = renderer.create_texture_streaming(txt_format, w, h).unwrap();

        // TODO function for widget creation and automaic layout
        let widget_memvis = {
            let vis = MemVisState::new(memvis_texture);
            let (w, h) = vis.get_initial_size();
            PositionedFrame {
                rect: Rect::new(1, 1, w, h),
                scale: 1.0,
                vis: Box::new(vis),
            }
        };


        let widget_vidram_bg = {
            let vis = VidRamBGDisplay { tile_data_select: TileDataSelect::Auto };
            let (w, h) = vis.get_initial_size();
            PositionedFrame {
                rect: Rect::new(MEM_DISP_WIDTH + 3, 1, w, h),
                scale: 1.0,
                vis: Box::new(vis),
            }
        };

        let widget_vidram_tiles = {
            let vis = VidRamTileDisplay { tile_data_select: TileDataSelect::Auto };
            let (w, h) = vis.get_initial_size();
            PositionedFrame {
                rect: Rect::new((MEM_DISP_WIDTH + SCREEN_BUFFER_SIZE_X as i32) as i32 + 5,
                                0,
                                w,
                                h),
                scale: 1.0,
                vis: Box::new(vis),
            }
        };

        let mut widgets = Vec::new();
        widgets.push(widget_memvis);
        widgets.push(widget_vidram_bg);
        widgets.push(widget_vidram_tiles);

        ApplicationState {
            gameboy: gameboy,
            sdl_context: sdl_context,
            sound_system: device,
            renderer: renderer,
            cycle_count: 0,
            prev_time: 0,
            // FIXME sound_cycles is probably wrong or not needed
            sound_cycles: 0,
            debugger: debugger,
            prev_hsync_cycles: 0,
            timer_cycles: 0,
            div_timer_cycles: 0,
            initial_gameboy_state: gbcopy,
            logger_handle: handle,
            controller: controller,
            screenshot_frame_num: Wrapping(0),
            ui_scale: SCALE,
            ui_offset: Point::new(0, 0),
            widgets: widgets,
        }
    }

    pub fn load_controller_if_none_exist(&mut self) {
        let (valid_controller, just_unplugged) = if let Some(ref c) = self.controller {
            if !c.attached() {
                info!("Controller {} has been unplugged", c.name());
                (false, true)
            } else {
                (true, false)
            }
        } else {
            (false, false)
        };

        match (valid_controller, just_unplugged) {
            (true, false) => (),
            (false, true) => self.controller = None,
            (false, false) => self.controller = setup_controller_subsystem(&self.sdl_context),
            _ => unreachable!(),
        }
    }

    pub fn display_coords_to_ui_point(&self, x: i32, y: i32) -> Point {
        let s_x = (x as f32 / self.ui_scale) as i32;
        let s_y = (y as f32 / self.ui_scale) as i32;
        Point::new(s_x, s_y)
    }



    /// Handles both controller input and keyboard/mouse debug input
    /// NOTE: does not handle input for ncurses debugger
    pub fn handle_events(&mut self) {
        for event in self.sdl_context
                .event_pump()
                .unwrap()
                .poll_iter() {
            use sdl2::event::Event;

            match event {
                Event::ControllerAxisMotion { axis, value: val, .. } => {
                    let deadzone = 10000;
                    trace!("Axis {:?} moved to {}", axis, val);
                    match axis {
                        controller::Axis::LeftX if deadzone < (val as i32).abs() => {
                            if val < 0 {
                                self.gameboy.press_left();
                                self.gameboy.unpress_right();
                            } else {
                                self.gameboy.press_right();
                                self.gameboy.unpress_left();
                            };
                        }
                        controller::Axis::LeftX => {
                            self.gameboy.unpress_left();
                            self.gameboy.unpress_right();
                        }
                        controller::Axis::LeftY if deadzone < (val as i32).abs() => {
                            if val < 0 {
                                self.gameboy.press_up();
                                self.gameboy.unpress_down();
                            } else {
                                self.gameboy.press_down();
                                self.gameboy.unpress_up();
                            }
                        }
                        controller::Axis::LeftY => {
                            self.gameboy.unpress_up();
                            self.gameboy.unpress_down();
                        }
                        _ => {}
                    }

                }
                Event::ControllerButtonDown { button, .. } => {
                    trace!("Button {:?} down", button);
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
                    trace!("Button {:?} up", button);
                    match button {
                        controller::Button::A => {
                            self.gameboy.unpress_a();
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
                Event::KeyDown { keycode: Some(keycode), repeat, .. } => {
                    if !repeat {
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
                                self.gameboy.reinit_logger();

                                // // This way makes it possible to edit rom
                                // // with external editor and see changes
                                // // instantly.
                                // gameboy = Cpu::new();
                                // gameboy.load_rom(rom_file);
                            }
                            Keycode::A => self.gameboy.press_a(),
                            Keycode::S => self.gameboy.press_b(),
                            Keycode::D => self.gameboy.press_select(),
                            Keycode::F => self.gameboy.press_start(),
                            Keycode::Up => self.gameboy.press_up(),
                            Keycode::Down => self.gameboy.press_down(),
                            Keycode::Left => self.gameboy.press_left(),
                            Keycode::Right => self.gameboy.press_right(),
                            _ => (),
                        }
                    }
                }
                Event::KeyUp { keycode: Some(keycode), repeat, .. } => {
                    if !repeat {
                        match keycode {
                            Keycode::A => self.gameboy.unpress_a(),
                            Keycode::S => self.gameboy.unpress_b(),
                            Keycode::D => self.gameboy.unpress_select(),
                            Keycode::F => self.gameboy.unpress_start(),
                            Keycode::Up => self.gameboy.unpress_up(),
                            Keycode::Down => self.gameboy.unpress_down(),
                            Keycode::Left => self.gameboy.unpress_left(),
                            Keycode::Right => self.gameboy.unpress_right(),

                            _ => (),
                        }
                    }
                }
                Event::MouseButtonDown { x, y, mouse_btn, .. } => {
                    // Transform screen coordinates in UI coordinates
                    let click_point = self.display_coords_to_ui_point(x, y);

                    // Find clicked widget
                    for widget in &mut self.widgets {
                        if widget.rect.contains(click_point) {
                            widget.click(mouse_btn, click_point, &mut self.gameboy);
                            break;
                        }
                    }
                }
                Event::MouseWheel { y, .. } => {
                    self.ui_scale += y as f32;
                    // self.widgets[0].scale += y as f32;
                }
                // // Event::MouseMotion { x, y, mousestate, xrel, yrel, .. } => {
                // Event::MouseMotion { x, y, .. } => {
                //     // Test widget position
                //     let mouse_pos = self.display_coords_to_ui_point(x+5, y+5);
                //     self.widgets[0].rect.reposition(mouse_pos);
                // }
                _ => (),
            }
        }
    }

    /// Runs the game application forward one "unit of time"
    /// Attepmts to load a controller if it can find one every time a frame is drawn
    /// TODO: elaborate
    pub fn step(&mut self) {
        // handle_events(&mut sdl_context, &mut gameboy);

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


        let scale = self.ui_scale;
        match self.renderer.set_scale(scale, scale) {
            Ok(_) => (),
            Err(_) => error!("Could not set render scale"),
        }

        let sound_upper_limit =
            ((CPU_CYCLES_PER_SECOND as f32) / self.gameboy.channel1_sweep_time()) as u64;

        if self.sound_cycles >= sound_upper_limit {
            self.sound_cycles -= sound_upper_limit;

            if self.gameboy.get_sound1() || self.gameboy.get_sound2() {
                self.sound_system.resume();
            } else {
                self.sound_system.pause();
            }

            let mut sound_system = self.sound_system.lock();
            // TODO move this to channel.update() or something
            sound_system.channel1.wave_duty = self.gameboy.channel1_wave_pattern_duty();
            let channel1_freq = 4194304.0 /
                                (4.0 * 8.0 * (2048.0 - self.gameboy.channel1_frequency() as f32));
            sound_system.channel1.phase_inc = channel1_freq / sound_system.out_freq;

            //sound_system.channel2.wave_duty = self.gameboy.channel2_wave_pattern_duty();
            let channel2_freq = 4194304.0 /
                                (4.0 * 8.0 * (2048.0 - self.gameboy.channel2_frequency() as f32));
            sound_system.channel2.phase_inc = channel2_freq / sound_system.out_freq;

        }

        // 1ms before drawing in terms of CPU time we must throw a vblank interrupt
        // TODO make this variable based on whether it's GB, SGB, etc.

        if (self.cycle_count - self.prev_time) >= CPU_CYCLES_PER_VBLANK {
            // check for controller every time a frame is drawn
            self.load_controller_if_none_exist();

            if let Some(ref mut dbg) = self.debugger {
                dbg.step(&mut self.gameboy);
            }

            let cycle_count = self.cycle_count;
            self.prev_time = cycle_count;
            self.renderer.set_draw_color(NICER_COLOR);
            self.renderer.clear();

            // Draw all widgets
            for ref mut widget in &mut self.widgets {
                widget.draw(&mut self.renderer, &mut self.gameboy);
            }

            //   00111100 1110001 00001000
            //   01111110 1110001 00010100
            //   11111111 1110001 00101010
            //

            // TODO add a way to enable/disable this while running
            let record_screen = false;
            if record_screen {
                save_screenshot(&self.renderer,
                                format!("screen{:010}.bmp", self.screenshot_frame_num.0).as_ref());
                self.screenshot_frame_num += Wrapping(1);
            }
            self.renderer.present();
        }
    }
}
