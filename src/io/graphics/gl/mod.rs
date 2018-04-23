use sdl2::{self, *, keyboard::Keycode, pixels::PixelFormatEnum, rect::Point, surface::Surface,
           video::GLProfile};

use cpu::Cpu;
use io::{applicationsettings::ApplicationSettings, constants::*,
         graphics::{renderer::Renderer, sdl2::input::*}, sound::*};
use super::renderer::{self, EventResponse};

use gl::{self, *};

static GB_VERT_SHADER_SOURCE: &'static str = include_str!("shaders/gameboy.vert");
static GB_FRAG_SHADER_SOURCE: &'static str = include_str!("shaders/gameboy.frag");

pub struct GlRenderer {
    sdl_context: Sdl, //  sdl_sound: sdl2::audio,
    ctx: sdl2::video::GLContext,
    window: sdl2::video::Window,
    controller: Option<sdl2::controller::GameController>, // storing to keep alive
}

impl GlRenderer {
    pub fn new(app_settings: &ApplicationSettings) -> Result<Self, String> {
        let sdl_context = sdl2::init()?;
        setup_audio(&sdl_context)?;
        let controller = setup_controller_subsystem(&sdl_context);

        // Set up graphics and window
        trace!("Opening window");
        let video_subsystem = sdl_context.video()?;

        let gl_attr = video_subsystem.gl_attr();
        gl_attr.set_context_profile(GLProfile::Core);
        gl_attr.set_context_version(4, 3);

        let window = {
            let (window_width, window_height) = if app_settings.memvis_mode {
                (RB_SCREEN_WIDTH, RB_SCREEN_HEIGHT)
            } else {
                (
                    ((GB_SCREEN_WIDTH as f32) * 2.0) as u32,
                    ((GB_SCREEN_HEIGHT as f32) * 2.0) as u32,
                )
            };

            match video_subsystem
                .window(
                    app_settings.rom_file_name.as_str(),
                    window_width,
                    window_height,
                )
                .position_centered()
                .opengl()
                .build()
            {
                Ok(v) => v,
                Err(e) => panic!("Fatal error: {}", e),
            }
        };
        let ctx = window.gl_create_context()?;
        gl::load_with(|name| video_subsystem.gl_get_proc_address(name) as *const _);

        use std::ffi::CString;
        let vshader_src: CString =
            CString::new(GB_VERT_SHADER_SOURCE).expect("Invalid vertex shader source");
        let fshader_src: CString =
            CString::new(GB_FRAG_SHADER_SOURCE).expect("Invalid fragment shader source");
        let vshader_ptr = (&(&GB_VERT_SHADER_SOURCE).as_ptr() as *const *const u8);
        let fshader_ptr = (&(&GB_FRAG_SHADER_SOURCE).as_ptr() as *const *const u8);
        unsafe {
            gl::ClearColor(0f32, 0f32, 0f32, 1f32);
            let vshader_id = gl::CreateShader(VERTEX_SHADER);
            let fshader_id = gl::CreateShader(FRAGMENT_SHADER);

            gl::ShaderSource(
                vshader_id,
                1,
                vshader_ptr as _,
                (&(GB_VERT_SHADER_SOURCE.len()) as *const usize) as _,
            );
            gl::ShaderSource(
                fshader_id,
                1,
                fshader_ptr as _,
                (&(GB_FRAG_SHADER_SOURCE.len()) as *const usize) as _,
            );

            gl::CompileShader(vshader_id);
            gl::CompileShader(fshader_id);
            let gb_program_id = gl::CreateProgram();
            gl::AttachShader(gb_program_id, vshader_id);
            gl::AttachShader(gb_program_id, fshader_id);
            gl::LinkProgram(gb_program_id);
            gl::UseProgram(gb_program_id);
        }

        Ok(Self {
            sdl_context,
            controller,
            window,
            ctx,
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

impl Renderer for GlRenderer {
    fn draw_gameboy(&mut self, gameboy: &Cpu, app_settings: &ApplicationSettings) {
        //unsafe { gl::DrawElements(TRIANGLES, , UNSIGNED_SHORT, 0u32 as _) }
        self.window.gl_swap_window();
        unsafe {
            gl::Clear(COLOR_BUFFER_BIT);
        }
    }

    fn draw_memory_visualization(&mut self, _gameboy: &Cpu, _app_settings: &ApplicationSettings) {
        unimplemented!();
    }

    fn handle_events(
        &mut self,
        gameboy: &mut Cpu,
        app_settings: &ApplicationSettings,
    ) -> Vec<renderer::EventResponse> {
        let mut ret_vec: Vec<renderer::EventResponse> = vec![];
        for event in self.sdl_context.event_pump().unwrap().poll_iter() {
            use sdl2::event::Event;

            match event {
                Event::ControllerAxisMotion {
                    axis, value: val, ..
                } => {
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
                Event::JoyDeviceRemoved {
                    which: device_id, ..
                }
                | Event::ControllerDeviceRemoved {
                    which: device_id, ..
                } => {
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
                Event::AppTerminating { .. }
                | Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => {
                    ret_vec.push(EventResponse::ProgramTerminated);
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
                Event::MouseButtonDown {
                    x, y, mouse_btn, ..
                } => {
                    // Transform screen coordinates in UI coordinates
                    let click_point = display_coords_to_ui_point(app_settings.ui_scale, x, y);

                    // Find clicked widget
                    /*for widget in &mut self.widgets {
                        if widget.rect.contains_point(click_point) {
                            widget.click(mouse_btn, click_point, gameboy);
                            break;
                        }
                    }*/                }
                Event::MouseWheel { y: _y, .. } => {
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
