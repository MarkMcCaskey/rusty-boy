pub mod utility;

pub mod input;
pub mod memvis;
pub mod vidram;

use sdl2;
use sdl2::keyboard::Keycode;
use sdl2::rect::{Point, Rect};
use sdl2::*;

use self::input::*;
use self::memvis::MemVisState;
use self::utility::PositionedFrame;
use self::vidram::{VidRamBGDisplay, VidRamTileDisplay};
use super::renderer;
use super::renderer::EventResponse;
use crate::io::applicationsettings::ApplicationSettings;
use crate::io::constants::*;
use crate::io::graphics::renderer::{Button, InputReceiver, Renderer};
use crate::io::sound::*;

use self::utility::*;

pub struct Sdl2Renderer {
    sdl_context: Sdl,
    _sound_system: sdl2::audio::AudioDevice<GBSound>,
    _canvas: render::Canvas<video::Window>,
    controller: Option<sdl2::controller::GameController>, // storing to keep alive
    widgets: Vec<PositionedFrame>,
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
            .map_err(|_| "Could not create SDL2 window")?;

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
            let vis = VidRamBGDisplay {
                tile_data_select: TileDataSelect::Auto,
            };
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
            let vis = VidRamTileDisplay {
                tile_data_select: TileDataSelect::Auto,
            };
            let (w, h) = vis.get_initial_size();
            PositionedFrame {
                rect: Rect::new((MEM_DISP_WIDTH + SCREEN_BUFFER_SIZE_X as i32) + 5, 0, w, h),
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
            _sound_system: sound_system,
            _canvas: renderer,
            controller,
            widgets,
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
    fn draw_frame(&mut self, _frame: &[[(u8, u8, u8); GB_SCREEN_WIDTH]; GB_SCREEN_HEIGHT]) {
        todo!("do this later if we care")
    }

    fn handle_events(&mut self, ir: &mut dyn InputReceiver) -> Vec<renderer::EventResponse> {
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
                                ir.press(Button::Left);
                                ir.unpress(Button::Right);
                            } else {
                                ir.press(Button::Right);
                                ir.unpress(Button::Left);
                            };
                        }
                        controller::Axis::LeftX => {
                            ir.unpress(Button::Left);
                            ir.press(Button::Right);
                        }
                        controller::Axis::LeftY if deadzone < (val as i32).abs() => {
                            if val < 0 {
                                ir.press(Button::Up);
                                ir.unpress(Button::Down);
                            } else {
                                ir.press(Button::Down);
                                ir.unpress(Button::Up);
                            }
                        }
                        controller::Axis::LeftY => {
                            ir.unpress(Button::Up);
                            ir.unpress(Button::Down);
                        }
                        _ => {}
                    }
                }
                Event::ControllerButtonDown { button, .. } => {
                    trace!("Button {:?} down", button);
                    match button {
                        controller::Button::A => {
                            ir.press(Button::A);
                            // TODO: sound
                            // device.resume();
                        }
                        controller::Button::B => ir.press(Button::B),
                        controller::Button::Back => ir.press(Button::Select),
                        controller::Button::Start => ir.press(Button::Start),
                        _ => (),
                    }
                }

                Event::ControllerButtonUp { button, .. } => {
                    trace!("Button {:?} up", button);
                    match button {
                        controller::Button::A => {
                            ir.unpress(Button::A);
                        }
                        controller::Button::B => ir.unpress(Button::B),
                        controller::Button::Back => ir.unpress(Button::Select),
                        controller::Button::Start => ir.unpress(Button::Start),
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
                            Keycode::F3 => ir.toggle_logger(),
                            Keycode::R => {
                                // Reset/reload emu
                                // TODO Keep previous visualization settings
                                ir.reset();
                                ret_vec.push(EventResponse::Reset);
                                //let gbcopy = self.initial_gameboy_state.clone();
                                //gameboy = gbcopy;
                                ir.reinit_logger();

                                // // This way makes it possible to edit rom
                                // // with external editor and see changes
                                // // instantly.
                                // gameboy = Cpu::new();
                                // gameboy.load_rom(rom_file);
                            }
                            Keycode::A => ir.press(Button::A),
                            Keycode::S => ir.press(Button::B),
                            Keycode::D => ir.press(Button::Select),
                            Keycode::F => ir.press(Button::Start),
                            Keycode::Up => ir.press(Button::Up),
                            Keycode::Down => ir.press(Button::Down),
                            Keycode::Left => ir.press(Button::Left),
                            Keycode::Right => ir.press(Button::Right),
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
                            Keycode::A => ir.unpress(Button::A),
                            Keycode::S => ir.unpress(Button::B),
                            Keycode::D => ir.unpress(Button::Select),
                            Keycode::F => ir.unpress(Button::Start),
                            Keycode::Up => ir.unpress(Button::Up),
                            Keycode::Down => ir.unpress(Button::Down),
                            Keycode::Left => ir.unpress(Button::Left),
                            Keycode::Right => ir.unpress(Button::Right),
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

pub fn display_coords_to_ui_point(ui_scale: f32, x: i32, y: i32) -> Point {
    let s_x = (x as f32 / ui_scale) as i32;
    let s_y = (y as f32 / ui_scale) as i32;
    Point::new(s_x, s_y)
}
