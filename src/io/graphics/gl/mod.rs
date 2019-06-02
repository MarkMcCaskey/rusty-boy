pub mod util;

use sdl2::{
    self, keyboard::Keycode, pixels::PixelFormatEnum, rect::Point, surface::Surface,
    video::GLProfile, *,
};

use super::renderer::{self, EventResponse};
use crate::cpu::Cpu;
use crate::io::{
    applicationsettings::ApplicationSettings,
    constants::*,
    graphics::{renderer::Renderer, sdl2::input::*},
    sound::*,
};

use gl::{self, types::*};
use rusty_boy_derive::VertexAttribPointers;
use util::*;

static GB_VERT_SHADER_SOURCE: &'static str = include_str!("shaders/gameboy.vert");
static GB_FRAG_SHADER_SOURCE: &'static str = include_str!("shaders/gameboy.frag");

fn hsv_to_rgb(h: u32, s: u32, v: u32) -> u32 {
    if s == 0 {
        return v | (v << 8) | (v << 16);
    }
    let region = h / 43;
    let remainder = (h - region * 43) * 6;

    let p = (v * (255 - s)) >> 8;
    let q = (v * (255 - ((s * remainder) >> 8))) >> 8;
    let t = (v * (255 - ((s * (255 - remainder)) >> 8))) >> 8;
    match region {
        0 => v << 24 | t << 16 | p << 8,
        1 => q << 24 | v << 16 | p << 8,
        2 => p << 24 | v << 16 | t << 8,
        3 => p << 24 | q << 16 | v << 8,
        4 => t << 24 | p << 16 | v << 8,
        _ => v << 24 | p << 16 | q << 8,
    }
}

pub struct GlRenderer {
    sdl_context: Sdl, //  sdl_sound: sdl2::audio,
    ctx: sdl2::video::GLContext,
    window: sdl2::video::Window,
    gb_program: Program,
    controller: Option<sdl2::controller::GameController>, // storing to keep alive
    num_tiles: i32,
    gb_vao: VertexArray,
    background_texture: Texture,
}

#[derive(VertexAttribPointers, Copy, Clone, Debug)]
#[repr(C, packed)]
struct GbScreenVertex {
    #[location = 0]
    pos: data::f32_f32,
    #[location = 1]
    color: data::f32_f32_f32,
    #[location = 2]
    texcoord: data::f32_f32,
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

        let (window_width, window_height) = if app_settings.memvis_mode {
            (RB_SCREEN_WIDTH, RB_SCREEN_HEIGHT)
        } else {
            (
                ((GB_SCREEN_WIDTH as f32) * 2.0) as u32,
                ((GB_SCREEN_HEIGHT as f32) * 2.0) as u32,
            )
        };
        let window = {
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
        window.gl_set_context_to_current().unwrap();
        unsafe {
            gl::Enable(gl::DEBUG_OUTPUT);
            gl::DebugMessageCallback(dbg_msg, std::ptr::null_mut());
        }

        info!("Here");
        use std::ffi::CString;
        let vshader_src: CString =
            CString::new(GB_VERT_SHADER_SOURCE).expect("Invalid vertex shader source");
        let fshader_src: CString =
            CString::new(GB_FRAG_SHADER_SOURCE).expect("Invalid fragment shader source");
        let gb_program = unsafe {
            gl::Viewport(0, 0, window_width as i32, window_height as i32);
            gl::ClearColor(0f32, 0f32, 0f32, 1f32);

            let gb_program = Program::from_shaders(&[
                Shader::from_vert_source(&vshader_src)?,
                Shader::from_frag_source(&fshader_src)?,
            ])?;
            gl::UseProgram(gb_program.id());
            gb_program
        };

        let mut verts: Vec<GbScreenVertex> = vec![];
        let mut indices: Vec<u16> = vec![];
        let num_tiles = 32;
        let num_tilesf = num_tiles as f32;
        let mut counter = 0;
        for i in 0..num_tiles {
            for j in 0..num_tiles {
                let denormal_br_x = ((i as f32) / num_tilesf);
                let denormal_br_y = ((j as f32) / num_tilesf);
                let bot_right_x = (denormal_br_x - 0.5) * 2.;
                let bot_right_y = (denormal_br_y - 0.5) * 2.;

                verts.push(GbScreenVertex {
                    pos: (bot_right_x, bot_right_y).into(),
                    color: (1., 0., 0.).into(),
                    texcoord: (denormal_br_x, denormal_br_y).into(),
                });
                verts.push(GbScreenVertex {
                    pos: (bot_right_x + (1.0 / num_tilesf * 2.), bot_right_y).into(),
                    color: (0., 1., 0.).into(),
                    texcoord: (denormal_br_x + (1.0 / num_tilesf), denormal_br_y).into(),
                });
                verts.push(GbScreenVertex {
                    pos: (
                        bot_right_x + (1.0 / num_tilesf * 2.),
                        bot_right_y + (1.0 / num_tilesf * 2.),
                    )
                        .into(),
                    color: (0., 0., 1.).into(),
                    texcoord: (
                        denormal_br_x + (1.0 / num_tilesf),
                        denormal_br_y + (1.0 / num_tilesf),
                    )
                        .into(),
                });
                verts.push(GbScreenVertex {
                    pos: (bot_right_x, bot_right_y + (1.0 / num_tilesf * 2.)).into(),
                    color: (0., 1., 1.).into(),
                    texcoord: (denormal_br_x, denormal_br_y + (1.0 / num_tilesf)).into(),
                });

                indices.push(counter);
                indices.push(counter + 1);
                indices.push(counter + 2);
                indices.push(counter);
                indices.push(counter + 2);
                indices.push(counter + 3);
                counter += 4;
            }
        }
        let vbo = ArrayBuffer::new();
        vbo.bind();
        vbo.static_draw_data(&verts);
        vbo.unbind();
        let vao = VertexArray::new();
        vao.bind();
        vbo.bind();
        GbScreenVertex::vertex_attrib_pointers();
        let ibo = ElementArrayBuffer::new();
        ibo.bind();
        ibo.static_draw_data(&indices);
        vbo.unbind();
        vao.unbind();
        const TEX_X: i32 = 256;
        const TEX_Y: i32 = 256;
        let texture = Texture::new(TEX_X, TEX_Y);
        texture.bind();
        /*let mut pixels = [0u32; (TEX_X * TEX_Y) as usize];
        for i in 0..TEX_X {
            for j in 0..TEX_Y {
                pixels[((i * TEX_Y) + j) as usize] = hsv_to_rgb(i as u32, j as u32, 255); //0x00A00000 | (i as u32) | ((j as u32) << 8); //((i as f32) / 255.) / ((j as f32) / 255.);
            }
        }*/
        let img = bmp::open(concat!(env!("CARGO_MANIFEST_DIR"), "/test.bmp")).unwrap();
        let mut pixels = [0u32; (TEX_X * TEX_Y) as usize];

        for i in 0..TEX_X {
            for j in 0..TEX_Y {
                let pix = img.get_pixel(j as u32, i as u32);
                let val = (pix.r as u32) << 24 | (pix.g as u32) << 16 | (pix.b as u32) << 8 | 0xFF;
                pixels[((i * TEX_Y) + j) as usize] = val;
            }
        }

        texture.upload_pixels(&pixels);
        texture.unbind();

        Ok(Self {
            sdl_context,
            controller,
            window,
            gb_program,
            ctx,
            num_tiles,
            gb_vao: vao,
            background_texture: texture,
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
    fn draw_gameboy(&mut self, _gameboy: &Cpu, _app_settings: &ApplicationSettings) {
        info!("In draw");
        // Use shader program
        self.gb_program.use_program();
        self.gb_vao.bind();
        self.background_texture.bind();
        unsafe {
            gl::Clear(gl::COLOR_BUFFER_BIT);
            gl::DrawElements(
                gl::TRIANGLES,
                self.num_tiles * self.num_tiles * 6,
                gl::UNSIGNED_SHORT,
                std::ptr::null(),
            );
            //gl::DrawArrays(gl::TRIANGLE_STRIP, 0, self.num_tiles * self.num_tiles * 4);
        }
        self.gb_vao.unbind();
        self.background_texture.unbind();
        self.window.gl_swap_window();
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
                    x, y, mouse_btn: _, ..
                } => {
                    // Transform screen coordinates in UI coordinates
                    let _click_point = display_coords_to_ui_point(app_settings.ui_scale, x, y);

                    // Find clicked widget
                    /*for widget in &mut self.widgets {
                        if widget.rect.contains_point(click_point) {
                            widget.click(mouse_btn, click_point, gameboy);
                            break;
                        }
                    }*/
                }
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

extern "system" fn dbg_msg(
    src: u32,
    ty: u32,
    id: GLuint,
    sev: GLuint,
    len: GLsizei,
    msg: *const i8,
    user_param: *mut std::ffi::c_void,
) {
    unsafe {
        println!(
            "{} {} {} {} {} {}",
            src,
            ty,
            id,
            sev,
            len,
            std::ffi::CStr::from_ptr(msg).to_str().unwrap().to_string()
        );
    }
}
