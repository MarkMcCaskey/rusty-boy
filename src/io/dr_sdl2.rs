use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum;
use sdl2::rect::{Point, Rect};
use sdl2::surface::Surface;
use sdl2::*;

use crate::cpu::Cpu;
use crate::io::applicationsettings::ApplicationSettings;
use crate::io::constants::*;
use crate::io::graphics::renderer;
use crate::io::graphics::renderer::EventResponse;
use crate::io::graphics::renderer::Renderer;
use crate::io::graphics::sdl2::input::setup_controller_subsystem;
use crate::io::sound::*;

use crate::io::deferred_renderer::deferred_renderer;

pub struct Sdl2Renderer {
    sdl_context: Sdl,
    _sound_system: sdl2::audio::AudioDevice<GBSound>,
    canvas: render::Canvas<video::Window>,
    controller: Option<sdl2::controller::GameController>, // storing to keep alive
    _sound_cycles: u64,
}

impl Sdl2Renderer {
    pub fn new(app_settings: &ApplicationSettings) -> Result<Self, String> {
        let sdl_context = sdl2::init()?;
        let sound_system = setup_audio(&sdl_context)?;
        let controller = setup_controller_subsystem(&sdl_context);

        // Set up graphics and window
        trace!("Opening window");
        let video_subsystem = sdl_context.video()?;

        let window = {
            let (window_width, window_height) = (
                ((GB_SCREEN_WIDTH as f32) * 3.0) as u32,
                ((GB_SCREEN_HEIGHT as f32) * 3.0) as u32,
            );

            match video_subsystem
                .window(
                    app_settings.rom_file_name.as_str(),
                    window_width,
                    window_height,
                )
                .position_centered()
                .build()
            {
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

        Ok(Sdl2Renderer {
            sdl_context,
            _sound_system: sound_system,
            canvas: renderer,
            controller,
            _sound_cycles: 0,
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
    fn draw_frame(&mut self, frame: &[[u8; GB_SCREEN_WIDTH]; GB_SCREEN_HEIGHT]) {
        let scale = 3.0;
        //app_settings.ui_scale;
        match self.canvas.set_scale(scale, scale) {
            Ok(_) => (),
            Err(_) => error!("Could not set render scale"),
        }

        self.canvas.set_draw_color(*NICER_COLOR);
        self.canvas.clear();

        let tc = self.canvas.texture_creator();
        let temp_surface = Surface::new(
            (GB_SCREEN_WIDTH as f32) as u32,
            (GB_SCREEN_HEIGHT as f32) as u32,
            PixelFormatEnum::RGBA8888,
        )
        .unwrap();

        let mut temp_canvas = temp_surface.into_canvas().unwrap();

        for y in 0..GB_SCREEN_HEIGHT {
            for x in 0..GB_SCREEN_WIDTH {
                let px_color = frame[y][x];
                let (r, g, b) = TILE_PALETTE[px_color as usize].rgb();
                let color = sdl2::pixels::Color::RGB(r, g, b);

                temp_canvas.set_draw_color(color);
                temp_canvas
                    .draw_point(Point::new(x as i32, y as i32))
                    .unwrap();
            }
        }

        let mut texture = tc
            .create_texture_from_surface(&temp_canvas.into_surface())
            .unwrap();

        texture.set_blend_mode(sdl2::render::BlendMode::None);

        self.canvas
            .copy(
                &texture,
                None,
                Some(Rect::new(
                    0,
                    0,
                    GB_SCREEN_WIDTH as u32,
                    GB_SCREEN_HEIGHT as u32,
                    //MEM_DISP_WIDTH as u32,
                    //MEM_DISP_HEIGHT as u32,
                )),
            )
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
    fn draw_gameboy(&mut self, gameboy: &mut Cpu, _app_settings: &ApplicationSettings) -> usize {
        let frame = deferred_renderer(gameboy);
        self.draw_frame(&frame);

        0
    }

    fn draw_memory_visualization(&mut self, _gameboy: &Cpu, _app_settings: &ApplicationSettings) {
        unimplemented!();
    }

    fn handle_events(
        &mut self,
        gameboy: &mut Cpu,
        _app_settings: &ApplicationSettings,
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
                    x: _x,
                    y: _y,
                    mouse_btn: _mouse_btn,
                    ..
                } => {
                    // Transform screen coordinates in UI coordinates
                    /* let click_point = display_coords_to_ui_point(app_settings.ui_scale, x, y);

                    // Find clicked widget
                    for widget in &mut self.widgets {
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
