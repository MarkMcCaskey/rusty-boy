//! Memory visualization

use sdl2;
use sdl2::mouse::MouseButton;
use sdl2::pixels::*;
use sdl2::rect::{Point, Rect};
use sdl2::surface::Surface;

use std::num::Wrapping;

use super::utility::Drawable;
use crate::cpu::constants::CpuState;
use crate::cpu::constants::MemAddr;
use crate::io::constants::*;
//use cpu::memvis::cpuCOLOR_DEPTH;
use crate::cpu::*;

use crate::cpu::memvis::cpumemvis::*;

use crate::disasm;

/// State for the memory visualization system
pub struct MemVisState {
    pub mem_val_display_enabled: bool,
    //    pub texture: sdl2::render::Texture<'a>,
}

impl MemVisState {
    pub fn new() -> MemVisState {
        MemVisState {
            mem_val_display_enabled: true,
            //texture: texture,
        }
    }

    /// Returns maybe a memory address given the coordinates of the memory visualization
    pub fn screen_coord_to_mem_addr(&self, point: Point) -> Option<MemAddr> {
        let x_scaled = point.x();
        let y_scaled = point.y();
        // FIXME this check is not correct
        if x_scaled < MEM_DISP_WIDTH && y_scaled < MEM_DISP_HEIGHT + 1 {
            Some((x_scaled + y_scaled * MEM_DISP_WIDTH) as u16)
        } else {
            None
        }
    }
}

impl Drawable for MemVisState {
    fn get_initial_size(&self) -> (u32, u32) {
        (MEM_DISP_WIDTH as u32, MEM_DISP_HEIGHT as u32)
    }

    fn draw(&mut self, renderer: &mut sdl2::render::Canvas<Surface>, cpu: &mut Cpu) {
        // draw_memory_access(renderer, cpu);
        // // FIXME make this take immutable cpu arg
        //draw_memory_events(renderer, cpu);
        let dst_rect = Rect::new(0, 0, MEM_DISP_WIDTH as u32, MEM_DISP_HEIGHT as u32);

        if let Some(ref mut logger) = cpu.mem.logger {
            let txt_format = sdl2::pixels::PixelFormatEnum::RGBA8888;

            let texture_creator = renderer.texture_creator();
            let mut texture = texture_creator
                .create_texture_streaming(txt_format, MEM_DISP_WIDTH as u32, MEM_DISP_HEIGHT as u32)
                .unwrap();
            let depth = COLOR_DEPTH;
            let memvis_pitch = MEM_DISP_WIDTH as usize * depth;

            // Draw memory values just by copying them
            texture.set_blend_mode(sdl2::render::BlendMode::None);
            texture
                .update(None, &logger.values[..], memvis_pitch)
                .unwrap();
            renderer.copy(&texture, None, Some(dst_rect)).unwrap();

            // Blend access type on top of values
            texture.set_blend_mode(sdl2::render::BlendMode::Add);
            texture
                .update(None, &logger.access_flags[..], memvis_pitch)
                .unwrap();
            renderer.copy(&texture, None, Some(dst_rect)).unwrap();

            // FIXME This copy is here to please the Borrow Checker
            // God and ideally needs to be removed.
            let mut copy = [0; EVENT_LOGGER_TEXTURE_SIZE];
            copy[..].clone_from_slice(&logger.access_times[..]);

            // Create Surface from values stored in logger
            let mut surface = Surface::from_data(
                &mut copy,
                MEM_DISP_WIDTH as u32,
                MEM_DISP_HEIGHT as u32,
                memvis_pitch as u32,
                txt_format,
            )
            .unwrap();

            // This determines how fast access fades (actual speed
            // will depend on the frame rate).
            const ACCESS_FADE_ALPHA: u8 = 100;

            // Create texture with alpha to do fading effect
            let mut blend =
                Surface::new(MEM_DISP_WIDTH as u32, MEM_DISP_HEIGHT as u32, txt_format).unwrap();
            blend
                .fill_rect(None, Color::RGBA(0, 0, 0, ACCESS_FADE_ALPHA))
                .unwrap();

            // Default blend mode works, whatever it is.
            blend
                .set_blend_mode(sdl2::render::BlendMode::Blend)
                .unwrap();
            surface
                .set_blend_mode(sdl2::render::BlendMode::Add)
                .unwrap();

            // Do the actual fading effect
            blend.blit(None, &mut surface, None).unwrap();

            // Store faded values back into logger
            // NOTE sizes of textures differ from EVENT_LOGGER_TEXTURE_SIZE
            // FIXME there must be a better way to do this without copying
            surface.with_lock(|pixels| {
                logger.access_times[0..pixels.len()].clone_from_slice(&pixels[0..pixels.len()])
            });

            let tc = renderer.texture_creator();
            // Add access_time texture to make recent accesses brigher
            let mut blend_texture = tc.create_texture_from_surface(surface).unwrap();
            blend_texture.set_blend_mode(sdl2::render::BlendMode::Add);
            renderer.copy(&blend_texture, None, Some(dst_rect)).unwrap();

            texture.set_blend_mode(sdl2::render::BlendMode::Add);
            texture
                .update(None, &logger.access_times[..], memvis_pitch)
                .unwrap();
            renderer.copy(&texture, None, Some(dst_rect)).unwrap();

            // Reset blend mode to make other operations faster
            renderer.set_blend_mode(sdl2::render::BlendMode::None);
        }

        // Draw jumps
        draw_memory_events(renderer, cpu);

        // TODO Draw instant pc, again
    }

    /// Handle mouse click at pos. Prints some info about clicked
    /// address or jumps to it.
    fn click(&mut self, button: sdl2::mouse::MouseButton, position: Point, cpu: &mut Cpu) {
        match button {
            MouseButton::Left => {
                if let Some(pc) = self.screen_coord_to_mem_addr(position) {
                    print_address_info(pc, cpu);
                }
            }
            MouseButton::Right => {
                if let Some(pc) = self.screen_coord_to_mem_addr(position) {
                    info!("Jumping to ${:04X}", pc);
                    cpu.pc = pc;
                    if cpu.state != CpuState::Normal {
                        info!("CPU state was '{:?}', forcing run.", cpu.state);
                        cpu.state = CpuState::Normal;
                    }
                }
            }
            _ => (),
        }
    }
}

/// Returns point on screen where pixel representing address is drawn.
#[inline]
fn addr_to_point(addr: u16) -> Point {
    let x = (addr as i32) % MEM_DISP_WIDTH;
    let y = (addr as i32) / MEM_DISP_WIDTH;
    Point::new(x, y)
}

/// Clamp i16 value to 0-255 range.
#[inline]
fn clamp_color(v: i16) -> u8 {
    if v < 0 {
        0
    } else if v > 255 {
        255
    } else {
        v as u8
    }
}

/// Simple saturating color addition.
#[inline]
fn mix_color(r1: u8, g1: u8, b1: u8, r2: u8, g2: u8, b2: u8) -> (u8, u8, u8) {
    // FIXME this is just lazy code
    (
        clamp_color(r1 as i16 + r2 as i16),
        clamp_color(g1 as i16 + g2 as i16),
        clamp_color(b1 as i16 + b2 as i16),
    )
}

/// Use u8 value to scale other one.
// FIXME this is just lazy code
#[inline]
fn scale_col(scale: u8, color: u8) -> u8 {
    clamp_color((color as f32 * (scale as f32 / 255f32)) as i16)
}

/// Draw all memory values represented by pixels. Width is determined
/// by `MEM_DISP_WIDTH`.
pub fn draw_memory_values<T>(renderer: &mut sdl2::render::Canvas<T>, gameboy: &Cpu)
where
    T: sdl2::render::RenderTarget,
{
    let mut x = 0;
    let mut y = 0;

    for i in 0..0xFFFF {
        let p = gameboy.mem[i];

        use sdl2::pixels::*;

        // renderer.set_draw_color(Color::RGB(r,g,b));
        // renderer.set_draw_color(Color::RGB(p as u8, p as u8, p as u8));
        renderer.set_draw_color(Color::RGB(0, 0, p));
        // debug!("pixel at {}, {} is {}", x, y, p);

        let point = Point::new(x, y);

        match renderer.draw_point(point) {
            Ok(_) => (),
            Err(_) => error!("Could not draw point at {:?}", point),
        }

        // inc coord
        x = (x + 1) % MEM_DISP_WIDTH;
        if x == 0 {
            y += 1; // % 256; // does this matter?
        }
    }

    // draw current PC
    let pc = gameboy.pc;
    renderer.set_draw_color(Color::RGB(255, 255, 255));
    renderer.draw_point(addr_to_point(pc)).unwrap();
}

/// Draw memory values represented by pixels with colors showing types
/// of access (r/w/x).
pub fn draw_memory_access<T>(renderer: &mut sdl2::render::Canvas<T>, gameboy: &Cpu)
where
    T: sdl2::render::RenderTarget,
{
    // TODO replace this function with parts of MemVisState::draw()
    let mut x = 0;
    let mut y = 0;

    let event_logger = match gameboy.mem.logger {
        Some(ref logger) => logger,
        None => return,
    };

    for &p in event_logger.values.iter() {
        use sdl2::pixels::*;

        // let value = gameboy.mem[addr];
        let value = p;

        // let color = Color::RGB(
        //     clamp_color(v * ((p & 0x2) >> 1) as i16 + v>>2),
        //     clamp_color(v * ((p & 0x1) >> 0) as i16 + v>>2),
        //     clamp_color(v * ((p & 0x4) >> 2) as i16 + v>>2));

        let color = if p == 0 {
            // Was not accessed
            Color::RGB(value, value, value)
        } else {
            // FIXME The color is determined by value in memory, we
            // want to fade max color somewhat (to use bright colors
            // by other stuff), but also show at least something
            // instead of black.
            //
            // It will not overflow normally because input value is
            // 8bit, and "base" is added to 16bit value, and then the
            // value "clamped" so you get "saturating addition(?)"
            let base = 32;
            let value = (value >> 2) as i16;
            let scale = value + base;
            Color::RGB(
                clamp_color(scale * ((p & FLAG_W) >> 1) as i16),
                clamp_color(scale * (p & FLAG_R) as i16),
                clamp_color(255 * ((p & FLAG_X) >> 2) as i16),
            )
        };
        renderer.set_draw_color(color);

        // let r = event_logger.access_flags[(addr * 4)];
        // let g = event_logger.access_flags[(addr * 4) + 1];
        // let b = event_logger.access_flags[(addr * 4) + 2];

        // renderer.set_draw_color(
        //     if r == 0 && g == 0 && b == 0 {
        //         Color::RGB(p,p,p)
        //     } else {
        //         Color::RGB(r,g,b)
        //     });

        let point = Point::new(x, y);

        match renderer.draw_point(point) {
            Ok(_) => (),
            Err(_) => error!("Could not draw point at {:?}", point),
        }

        // inc coord
        x = (x + 1) % MEM_DISP_WIDTH;
        if x == 0 {
            y += 1; // % 256; // does this matter?
        }
    }

    // draw current PC
    let pc = gameboy.pc;
    renderer.set_draw_color(Color::RGB(255, 0, 255));
    renderer.draw_point(addr_to_point(pc)).unwrap();
}

/// Draw all `CpuEvents` that fade depending on current cpu time. When
/// age of event is more that `FADE_DELAY`, event is removed.
pub fn draw_memory_events<T>(renderer: &mut sdl2::render::Canvas<T>, gameboy: &Cpu)
where
    T: sdl2::render::RenderTarget,
{
    // TODO: can be used to do partial "smart" redraw, and speed thing up.
    // But event logging itself is extremely slow

    renderer.set_blend_mode(sdl2::render::BlendMode::Add);

    let event_logger = match gameboy.mem.logger {
        Some(ref logger) => logger,
        None => return,
    };

    // Draw current events with color determined by age
    for entry in &event_logger.events_deq {
        let timestamp = entry.timestamp;
        let event = &entry.event;
        {
            let time_diff = (Wrapping(gameboy.cycles) - Wrapping(timestamp)).0;
            if time_diff < FADE_DELAY {
                let time_norm = 1.0 - (time_diff as f32) / (FADE_DELAY as f32);
                let colval = (time_norm * 255.0) as u8;
                match *event {
                    CpuEvent::Read { from: addr } => {
                        let val = gameboy.mem[addr as usize];
                        let (r, g, b) = mix_color(0, colval, 0, scale_col(colval, val / 2), 0, val);
                        renderer.set_draw_color(Color::RGB(r, g, b));
                        match renderer.draw_point(addr_to_point(addr)) {
                            Ok(_) => (),
                            Err(_) => error!("Cannot draw point at {:?}", addr_to_point(addr)),
                        }
                    }
                    CpuEvent::Write { to: addr } => {
                        let val = gameboy.mem[addr as usize];
                        let (r, g, b) = mix_color(colval, 0, 0, 0, scale_col(colval, val / 2), val);
                        renderer.set_draw_color(Color::RGB(r, g, b));
                        match renderer.draw_point(addr_to_point(addr)) {
                            Ok(_) => (),
                            Err(_) => error!("Cannot draw point at {:?}", addr_to_point(addr)),
                        }
                    }
                    CpuEvent::Execute(addr) => {
                        let val = gameboy.mem[addr as usize];
                        let (r, g, b) = mix_color(colval, colval, scale_col(colval, val), 0, 0, 0);
                        renderer.set_draw_color(Color::RGB(r, g, b));
                        match renderer.draw_point(addr_to_point(addr)) {
                            Ok(_) => (),
                            Err(_) => error!("Cannot draw point at {:?}", addr_to_point(addr)),
                        }
                    }
                    CpuEvent::Jump { from: src, to: dst } => {
                        renderer.set_draw_color(Color::RGBA(200, 200, 0, colval));
                        let src_point = addr_to_point(src);
                        let dst_point = addr_to_point(dst);
                        // Horizontal lines are drawn with scaling but diagonal
                        // lines ignore it for some reason, which allows us to
                        // draw lines thinner than memory cells.
                        if src_point.y() != dst_point.y() {
                            match renderer.draw_line(src_point, dst_point) {
                                Ok(_) => (),
                                Err(_) => error!(
                                    "Cannot draw line from {:?} to {:?}",
                                    src_point, dst_point
                                ),
                            }
                        }
                    }
                    _ => (),
                }
            } else {
                break;
            }
        }
    }
    renderer.set_blend_mode(sdl2::render::BlendMode::None);
}

fn print_address_info(pc: MemAddr, cpu: &Cpu) {
    let pc = pc as usize;
    let mem = &cpu.mem;
    let b1 = mem[pc + 1];
    let b2 = mem[pc + 2];
    let (mnem, _) = disasm::pp_opcode(mem[pc], b1, b2, pc as u16);
    let nn = byte_to_u16(b1, b2);
    println!(
        "${:04X} {:16} 0x{:02X} 0x{:02X} 0x{:02X} 0x{:04X}",
        pc, mnem, mem[pc], b1, b2, nn
    );
}
