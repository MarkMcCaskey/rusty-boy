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
pub struct ApplicationState {
    pub gameboy: cpu::Cpu,
    sdl_context: Sdl, //  sdl_sound: sdl2::audio,
    sound_system: AudioDevice<SquareWave>,
    renderer: render::Renderer<'static>,
    cycle_count: u64,
    prev_time: u64,
    debugger: Option<Debugger>,
    prev_hsync_cycles: u64,
    clock_cycles: u64,
    initial_gameboy_state: cpu::Cpu,
    logger_handle: Option<log4rs::Handle>, // storing to keep alive
    controller: Option<sdl2::controller::GameController>, // storing to keep alive
    screenshot_frame_num: Wrapping<u64>,
    ui_scale: f32,
    ui_offset: Point, // TODO whole interface pan
    widgets: Vec<PositionedFrame>,
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


        // TODO function for widget creation and automaic layout
        let widget_memvis = {
            let vis = MemVisState::new();
            let (w, h) = vis.get_initial_size();
            PositionedFrame { rect: Rect::new(1, 1, w, h),
                              scale: 1.0,
                              vis: Box::new(vis) }
        };

        let widget_vidram_bg = {
            let vis = VidRamBGDisplay { tile_data_select: TileDataSelect::Auto };
            let (w, h) = vis.get_initial_size();
            PositionedFrame { rect: Rect::new(MEM_DISP_WIDTH+3, 1, w, h),
                              scale: 1.0,
                              vis: Box::new(vis) }
        };

        let widget_vidram_tiles = {
            let vis = VidRamTileDisplay { tile_data_select: TileDataSelect::Auto };
            let (w, h) = vis.get_initial_size();
            PositionedFrame { rect: Rect::new((MEM_DISP_WIDTH +
                                               SCREEN_BUFFER_SIZE_X as i32)
                                              as i32 + 5,
                                              0, w, h),
                              scale: 1.0,
                              vis: Box::new(vis) }
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
            debugger: debugger,
            prev_hsync_cycles: 0,
            clock_cycles: 0,
            initial_gameboy_state: gbcopy,
            logger_handle: handle,
            controller: controller,
            screenshot_frame_num: Wrapping(0),
            ui_scale: SCALE,
            ui_offset: Point::new(0, 0),
            widgets: widgets,
        }
    }

    pub fn display_coords_to_ui_point(&self, x: i32, y: i32) -> Point {
        let s_x = (x as f32 / self.ui_scale) as i32;
        let s_y = (y as f32 / self.ui_scale) as i32;
        Point::new(s_x, s_y)
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
                    // Transform screen coordinates in UI coordinates
                    let click_point = self.display_coords_to_ui_point(x, y);
                    
                    // Find clicked widget
                    for widget in self.widgets.iter_mut() {
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


        let scale = self.ui_scale;
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

            // Draw all widgets
            for ref widget in self.widgets.iter_mut() {
                widget.draw(&mut self.renderer, &self.gameboy);
            }

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
