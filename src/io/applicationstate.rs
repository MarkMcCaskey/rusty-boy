//! The wrapper around the information needed to meaningfully run this program
//!
//! NOTE: in the process of further abstracting IO logic with this --
//! expect things to break

use std;

use sdl2::*;
use sdl2::video::GLProfile;
use sdl2::audio::AudioDevice;
use sdl2::keyboard::Keycode;
use sdl2::surface::Surface;
use sdl2::pixels::PixelFormatEnum;
//use sdl2::render::{TextureCreator, Texture};
use log4rs;

use debugger::graphics::*;
use cpu;
use io::constants::*;
use io::input::*;
use io::graphics::*;
use io::memvis::MemVisState;
use io::vidram::{VidRamBGDisplay, VidRamTileDisplay};
use io::sound::*;
use io::applicationsettings::ApplicationSettings;

use log::LogLevelFilter;
use log4rs::append::console::ConsoleAppender;
use log4rs::encode::pattern::PatternEncoder;
use log4rs::config::{Appender, Config, Root};

use sdl2;
use sdl2::rect::{Point, Rect};

use std::num::Wrapping;
use std::path::PathBuf;


use app_dirs::*;

const APP_INFO: AppInfo = AppInfo {
    name: "rusty-boy",
    author: "Mark McCaskey, SpawnedArtifact, and friends",
};

/// Holds all the data needed to use the emulator in meaningful ways
#[allow(dead_code)] //suppress pointers to things to keep them alive,
// could almost certainly be done in a better way
pub struct ApplicationState {
    pub gameboy: cpu::Cpu,
    sdl_context: Sdl, //  sdl_sound: sdl2::audio,
    sound_system: AudioDevice<GBSound>,
    canvas: render::Canvas<video::Window>,
    //renderer: render::Renderer<'static>,
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
    config_path: Option<PathBuf>,
    data_path: Option<PathBuf>,
//    texture_creator: TextureCreator<WindowContext>,
}



impl ApplicationState {
    //! Sets up the environment for running in memory visualization mode
    pub fn new(app_settings: &ApplicationSettings)
               -> Result<ApplicationState, String> {
        // Set up logging
        let stdout = ConsoleAppender::builder()
            .encoder(Box::new(PatternEncoder::new("{h({l})} {m} {n}")))
            .build();

        let config = Config::builder()
            .appender(Appender::builder().build("stdout", Box::new(stdout)))
            .build(Root::builder()
                   .appender("stdout")
                   .build(match (app_settings.trace_mode,
                                 app_settings.debug_mode) {
                       (true, _) => LogLevelFilter::Trace,
                       (false, true) => LogLevelFilter::Debug,
                       _ => LogLevelFilter::Info,
                   }))
            .or_else(|_| Err("Could not build Config".to_string()))?;


        // Set up debugging or command-line logging
        let (should_debugger, handle) = if app_settings.debug_mode && cfg!(feature = "debugger") {
            info!("Running in debug mode");
            (true, None)
        } else {
            let handle = log4rs::init_config(config)
                .or_else(|_| Err("Could not init Config"))?;
            (false, Some(handle))
        };


        let data_path = match app_root(AppDataType::UserData, &APP_INFO) {
            Ok(v) => {
                debug!("Using user data path: {:?}", v);
                Some(v)
            }
            Err(e) => {
                error!("Could not open a user data path: {}", e);
                None
            }
        };

        let config_path = match app_root(AppDataType::UserConfig, &APP_INFO) {
            Ok(v) => {
                debug!("Using user config path: {:?}", v);
                Some(v)
            }
            Err(e) => {
                error!("Could not open a user config path: {}", e);
                None
            }
        };

        // Set up gameboy and other state
        let mut gameboy = cpu::Cpu::new();
        trace!("loading ROM");
        gameboy.load_rom(app_settings.rom_file_name.as_ref(), data_path.clone());


        // delay debugger so loading rom can be logged if need be
        let debugger = if should_debugger {
            Some(Debugger::new(&gameboy))
        } else {
            None
        };


        let sdl_context = sdl2::init()?;
        let device = setup_audio(&sdl_context)?;
        let controller = setup_controller_subsystem(&sdl_context);

        // Set up graphics and window
        trace!("Opening window");
        let video_subsystem = sdl_context.video()?;

        let gl_attr = video_subsystem.gl_attr();

        gl_attr.set_context_profile(GLProfile::Core);
        ///gl_attr.set_context_flags().debug().set();
        gl_attr.set_context_version(3, 2);

        let (window_width, window_height) =
            if app_settings.memvis_mode { (RB_SCREEN_WIDTH, RB_SCREEN_HEIGHT) }
        else { (((GB_SCREEN_WIDTH as f32) * 2.0) as u32,
                ((GB_SCREEN_HEIGHT as f32) * 2.0) as u32) };
        let window = match video_subsystem
                  .window(gameboy.get_game_name().as_str(),
                          window_width,
                          window_height)
                  .position_centered()
                  .opengl()
                  .build() {
            Ok(v) => v,
            Err(e) => panic!("Fatal error: {}", e),
        };

        // video_subsystem.gl_load_library_default();

        let renderer = window
            .into_canvas()
            .accelerated()
            .present_vsync()
            .build()
            .or_else(|_| Err("Could not create SDL2 window"))?;

        let gbcopy = gameboy.clone();

        // let txt_format = sdl2::pixels::PixelFormatEnum::RGBA8888;
        //let w = MEM_DISP_WIDTH as u32;
        //let h = MEM_DISP_HEIGHT as u32;
        //let memvis_texture = tc.create_texture_static(txt_format, w, h).unwrap();

        // TODO function for widget creation and automaic layout
        let widget_memvis = {
            let vis = MemVisState::new(); //memvis_texture);
            let (w, h) = vis.get_initial_size();
            PositionedFrame {
                rect: Rect::new(1, 1, w, h),
                scale: 1.0,
                vis: Box::new(vis),
            }
        };


        let widget_vidram_bg = {
            let vis = VidRamBGDisplay { tile_data_select: TileDataSelect::Auto };
            let (screen_pos_w, screen_pos_h) =
                if app_settings.memvis_mode { (MEM_DISP_WIDTH + 3, 1) }
                else { (0, 1) };

            let (w, h) = vis.get_initial_size();
            PositionedFrame {
                rect: Rect::new(screen_pos_w, screen_pos_h, w, h),
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
        if app_settings.memvis_mode { widgets.push(widget_memvis); }
        widgets.push(widget_vidram_bg);
        if app_settings.memvis_mode { widgets.push(widget_vidram_tiles); }

        Ok(ApplicationState {
               gameboy: gameboy,
               sdl_context: sdl_context,
               sound_system: device,
               //renderer: renderer,
               canvas: renderer,
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
               config_path: config_path,
               data_path: data_path,
            //texture_creator: tc,
           })
    }

    /// Loads a controller to be used as input if there isn't currently an active controller
    pub fn load_controller_if_none_exist(&mut self) {
        let should_load = if let Some(ref c) = self.controller {
            !c.attached()
        } else {
            true
        };

        if should_load {
            self.controller = setup_controller_subsystem(&self.sdl_context);
            if let Some(ref c) = self.controller {
                info!("Controller {} attached", c.name());
            } else {
                //Note: not printing a warning here because this function is
                // called every frame now


                //warn!("Could not attach controller!");
            }
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

        for event in self.sdl_context.event_pump().unwrap().poll_iter() {
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
                /*Event::JoyDeviceAdded {..} | Event::ControllerDeviceAdded{..} => {
                    self.load_controller_if_none_exist();
                }*/
                Event::JoyDeviceRemoved { which: device_id, .. } |
                Event::ControllerDeviceRemoved { which: device_id, .. } => {
                    let should_remove = if let Some(ref controller) = self.controller {
                        let sr = device_id == controller.instance_id();

                        if sr {
                            info!("Removing controller {}", controller.name());
                        }

                        sr
                    } else {
                        false
                    };

                    if should_remove {
                        self.controller = None;
                    }
                }
                Event::Quit { .. } |
                Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    info!("Program exiting!");
                    if let Some(ref mut debugger) = self.debugger {
                        debugger.die();
                    }
                    self.gameboy.save_ram(self.data_path.clone());
                    std::process::exit(0);
                }
                Event::KeyDown {
                    keycode: Some(keycode),
                    repeat,
                    ..
                } => {
                    if !repeat {
                        match keycode {
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
                Event::KeyUp {
                    keycode: Some(keycode),
                    repeat,
                    ..
                } => {
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
                        if widget.rect.contains_point(click_point) {
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
        match self.canvas.set_scale(scale, scale) {
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

            // sound_system.channel2.wave_duty = self.gameboy.channel2_wave_pattern_duty();
            let channel2_freq = 4194304.0 /
                                (4.0 * 8.0 * (2048.0 - self.gameboy.channel2_frequency() as f32));
            sound_system.channel2.phase_inc = channel2_freq / sound_system.out_freq;

            let channel3_freq = 4194304.0 /
                                (4.0 * 8.0 * (2048.0 - self.gameboy.channel3_frequency() as f32));
            sound_system.channel3.shift_amount = self.gameboy.channel3_shift_amount();
            sound_system.channel3.phase_inc = channel3_freq / sound_system.out_freq;
            sound_system.channel3.wave_ram = self.gameboy.channel3_wave_pattern_ram();

        }

        // 1ms before drawing in terms of CPU time we must throw a vblank interrupt
        // TODO make this variable based on whether it's GB, SGB, etc.

        if (self.cycle_count - self.prev_time) >= CPU_CYCLES_PER_VBLANK {

            //check for new controller every frame
            self.load_controller_if_none_exist();

            if let Some(ref mut dbg) = self.debugger {
                dbg.step(&mut self.gameboy);
            }

            let cycle_count = self.cycle_count;
            self.prev_time = cycle_count;
            self.canvas.set_draw_color(*NICER_COLOR);
            self.canvas.clear();

            // Draw all widgets

            let tc = self.canvas.texture_creator();
            let temp_surface = Surface::new((MEM_DISP_WIDTH as u32) + SCREEN_BUFFER_SIZE_X +
                                            (SCREEN_BUFFER_TILES_X * (TILE_SIZE_PX as u32)),
                                            (MEM_DISP_HEIGHT as u32) + SCREEN_BUFFER_SIZE_Y +
                                            (SCREEN_BUFFER_TILES_Y * (TILE_SIZE_PX as u32)),
                                            PixelFormatEnum::RGBA8888)
                    .unwrap();

            let mut temp_canvas = temp_surface.into_canvas().unwrap();
            for ref mut widget in &mut self.widgets {
                widget.draw(&mut temp_canvas, &mut self.gameboy);
            }
            let mut texture = tc.create_texture_from_surface(&temp_canvas.into_surface())
                .unwrap();

            texture.set_blend_mode(sdl2::render::BlendMode::None);

            self.canvas
                .copy(&texture,
                      None,
                      Some(Rect::new(0, 0, MEM_DISP_WIDTH as u32, MEM_DISP_HEIGHT as u32)))
                .unwrap();



            // TODO add a way to enable/disable this while running
            let record_screen = false;
            if record_screen {
                save_screenshot(&self.canvas,
                                format!("screen{:010}.bmp", self.screenshot_frame_num.0).as_ref());
                self.screenshot_frame_num += Wrapping(1);
            }

            self.canvas.present();

        }
        //for memory visualization
        self.gameboy.remove_old_events();


    }
}
