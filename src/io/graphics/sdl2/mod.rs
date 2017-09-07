pub mod utility;

pub mod memvis;
pub mod vidram;
pub mod input;

use sdl2;
use sdl2::*;
use sdl2::rect::{Point, Rect};
use sdl2::video::GLProfile;
use sdl2::audio::AudioDevice;
use sdl2::keyboard::Keycode;
use sdl2::surface::Surface;
use sdl2::pixels::PixelFormatEnum;

use cpu::Cpu;
use io::graphics::renderer::Renderer;
use io::constants::*;
use self::memvis::MemVisState;
use self::vidram::{VidRamBGDisplay, VidRamTileDisplay};
use self::utility::PositionedFrame;
use self::input::*;
use super::renderer;
use io::sound::*;
use io::applicationsettings::ApplicationSettings;
use super::renderer::EventResponse;

use std;

use self::utility::*;

pub struct Sdl2Renderer {
    sdl_context: Sdl, //  sdl_sound: sdl2::audio,
    canvas: render::Canvas<video::Window>,
    controller: Option<sdl2::controller::GameController>, // storing to keep alive
    widgets: Vec<PositionedFrame>,
}

impl Sdl2Renderer {
    pub fn new(app_settings: &ApplicationSettings) -> Result<Self, String> {
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

        let window = {
            let (window_width, window_height) = if app_settings.memvis_mode {
                (RB_SCREEN_WIDTH, RB_SCREEN_HEIGHT)
            } else {
                (((GB_SCREEN_WIDTH as f32) * 2.0) as u32, ((GB_SCREEN_HEIGHT as f32) * 2.0) as u32)
            };

            match video_subsystem
                      .window(app_settings.rom_file_name.as_str(),
                              window_width,
                              window_height)
                      .position_centered()
                      .opengl()
                      .build() {
                Ok(v) => v,
                Err(e) => panic!("Fatal error: {}", e),
            }
        };

        // video_subsystem.gl_load_library_default();

        let renderer = window
            .into_canvas()
            .accelerated()
            .present_vsync()
            .build()
            .or_else(|_| Err("Could not create SDL2 window"))?;


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
            let (screen_pos_w, screen_pos_h) = if app_settings.memvis_mode {
                (MEM_DISP_WIDTH + 3, 1)
            } else {
                (0, 1)
            };

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
        if app_settings.memvis_mode {
            widgets.push(widget_memvis);
            widgets.push(widget_vidram_tiles);
        }
        widgets.push(widget_vidram_bg);

        Ok(Sdl2Renderer {
               sdl_context,
               canvas: renderer,
               controller,
               widgets,
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
}

impl Renderer for Sdl2Renderer {
    fn draw_gameboy(&mut self, gameboy: &Cpu, app_settings: &ApplicationSettings) {

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


        let scale = app_settings.ui_scale;
        match self.canvas.set_scale(scale, scale) {
            Ok(_) => (),
            Err(_) => error!("Could not set render scale"),
        }

        // Sound is disabled while graphics code is being generalized
            // Sound will be generalized after
/*        let sound_upper_limit =
            ((CPU_CYCLES_PER_SECOND as f32) / gameboy.channel1_sweep_time()) as u64;

        if self.sound_cycles >= sound_upper_limit {
            self.sound_cycles -= sound_upper_limit;

            if gameboy.get_sound1() || self.gameboy.get_sound2() {
                self.sound_system.resume();
            } else {
                self.sound_system.pause();
            }

            let mut sound_system = self.sound_system.lock();
            // TODO move this to channel.update() or something
            sound_system.channel1.wave_duty = gameboy.channel1_wave_pattern_duty();
            let channel1_freq = 4194304.0 /
                                (4.0 * 8.0 * (2048.0 - gameboy.channel1_frequency() as f32));
            sound_system.channel1.phase_inc = channel1_freq / sound_system.out_freq;

            // sound_system.channel2.wave_duty = gameboy.channel2_wave_pattern_duty();
            let channel2_freq = 4194304.0 /
                                (4.0 * 8.0 * (2048.0 - gameboy.channel2_frequency() as f32));
            sound_system.channel2.phase_inc = channel2_freq / sound_system.out_freq;

            let channel3_freq = 4194304.0 /
                                (4.0 * 8.0 * (2048.0 - gameboy.channel3_frequency() as f32));
            sound_system.channel3.shift_amount = gameboy.channel3_shift_amount();
            sound_system.channel3.phase_inc = channel3_freq / sound_system.out_freq;
            sound_system.channel3.wave_ram = gameboy.channel3_wave_pattern_ram();

        }*/

        // 1ms before drawing in terms of CPU time we must throw a vblank interrupt
        // TODO make this variable based on whether it's GB, SGB, etc.


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
        //FIXME:
        let mut gameboy_copy = gameboy.clone();
        for ref mut widget in &mut self.widgets {
            widget.draw(&mut temp_canvas, &mut gameboy_copy);
        }
        let mut texture = tc.create_texture_from_surface(&temp_canvas.into_surface())
            .unwrap();

        texture.set_blend_mode(sdl2::render::BlendMode::None);

        self.canvas
            .copy(&texture,
                  None,
                  Some(Rect::new(0, 0, MEM_DISP_WIDTH as u32, MEM_DISP_HEIGHT as u32)))
            .unwrap();



        // feature disabled while graphics are being generalized
            // TODO add a way to enable/disable this while running
            /*let record_screen = false;
            if record_screen {
                save_screenshot(&self.canvas,
                                format!("screen{:010}.bmp", self.screenshot_frame_num.0).as_ref());
                self.screenshot_frame_num += Wrapping(1);
            }*/

        self.canvas.present();


    }


    fn draw_memory_visualization(&mut self, gameboy: &Cpu, app_settings: &ApplicationSettings) {
        unimplemented!();
    }

    fn handle_events(&mut self,
                     gameboy: &mut Cpu,
                     app_settings: &ApplicationSettings)
                     -> Vec<renderer::EventResponse> {
        let mut ret_vec: Vec<renderer::EventResponse> = vec![];
        for event in self.sdl_context.event_pump().unwrap().poll_iter() {
            use sdl2::event::Event;

            match event {
                Event::ControllerAxisMotion { axis, value: val, .. } => {
                    let deadzone = 10000;
                    trace!("Axis {:?} moved to {}", axis, val);
                    match axis {
                        controller::Axis::LeftX if deadzone < (val as i32).abs() => {
                            if val < 0 {
                                gameboy.press_left();
                                gameboy.unpress_right();
                            } else {
                                gameboy.press_right();
                                gameboy.unpress_left();
                            };
                        }
                        controller::Axis::LeftX => {
                            gameboy.unpress_left();
                            gameboy.unpress_right();
                        }
                        controller::Axis::LeftY if deadzone < (val as i32).abs() => {
                            if val < 0 {
                                gameboy.press_up();
                                gameboy.unpress_down();
                            } else {
                                gameboy.press_down();
                                gameboy.unpress_up();
                            }
                        }
                        controller::Axis::LeftY => {
                            gameboy.unpress_up();
                            gameboy.unpress_down();
                        }
                        _ => {}
                    }

                }
                Event::ControllerButtonDown { button, .. } => {
                    trace!("Button {:?} down", button);
                    match button {
                        controller::Button::A => {
                            gameboy.press_a();
                            // TODO: sound
                            // device.resume();
                        }
                        controller::Button::B => gameboy.press_b(),
                        controller::Button::Back => gameboy.press_select(),
                        controller::Button::Start => gameboy.press_start(),
                        _ => (),
                    }
                }

                Event::ControllerButtonUp { button, .. } => {
                    trace!("Button {:?} up", button);
                    match button {
                        controller::Button::A => {
                            gameboy.unpress_a();
                        }
                        controller::Button::B => gameboy.unpress_b(),
                        controller::Button::Back => gameboy.unpress_select(),
                        controller::Button::Start => gameboy.unpress_start(),
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
                    ret_vec.push(EventResponse::ProgramTerminated);
                    /*    if let Some(ref mut debugger) = self.debugger {
                        debugger.die();
                    }
                    gameboy.save_ram(self.application_settings.data_path.clone());
                    std::process::exit(0);*/
                }
                Event::KeyDown {
                    keycode: Some(keycode),
                    repeat,
                    ..
                } => {
                    if !repeat {
                        match keycode {
                            Keycode::F3 => gameboy.toggle_logger(),
                            Keycode::R => {
                                // Reset/reload emu
                                // TODO Keep previous visualization settings
                                gameboy.reset();
                                ret_vec.push(EventResponse::Reset);
                                //let gbcopy = self.initial_gameboy_state.clone();
                                //gameboy = gbcopy;
                                gameboy.reinit_logger();

                                // // This way makes it possible to edit rom
                                // // with external editor and see changes
                                // // instantly.
                                // gameboy = Cpu::new();
                                // gameboy.load_rom(rom_file);
                            }
                            Keycode::A => gameboy.press_a(),
                            Keycode::S => gameboy.press_b(),
                            Keycode::D => gameboy.press_select(),
                            Keycode::F => gameboy.press_start(),
                            Keycode::Up => gameboy.press_up(),
                            Keycode::Down => gameboy.press_down(),
                            Keycode::Left => gameboy.press_left(),
                            Keycode::Right => gameboy.press_right(),
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
                            Keycode::A => gameboy.unpress_a(),
                            Keycode::S => gameboy.unpress_b(),
                            Keycode::D => gameboy.unpress_select(),
                            Keycode::F => gameboy.unpress_start(),
                            Keycode::Up => gameboy.unpress_up(),
                            Keycode::Down => gameboy.unpress_down(),
                            Keycode::Left => gameboy.unpress_left(),
                            Keycode::Right => gameboy.unpress_right(),

                            _ => (),
                        }
                    }
                }
                Event::MouseButtonDown { x, y, mouse_btn, .. } => {
                    // Transform screen coordinates in UI coordinates
                    let click_point = display_coords_to_ui_point(app_settings.ui_scale, x, y);

                    // Find clicked widget
                    for widget in &mut self.widgets {
                        if widget.rect.contains_point(click_point) {
                            widget.click(mouse_btn, click_point, gameboy);
                            break;
                        }
                    }
                }
                Event::MouseWheel { y, .. } => {
                    //self.ui_scale += y as f32;
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

        return ret_vec;
    }
}

pub fn display_coords_to_ui_point(ui_scale: f32, x: i32, y: i32) -> Point {
    let s_x = (x as f32 / ui_scale) as i32;
    let s_y = (y as f32 / ui_scale) as i32;
    Point::new(s_x, s_y)
}
