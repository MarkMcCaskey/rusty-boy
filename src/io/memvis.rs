//! Memory visualization

use sdl2;
use sdl2::rect::{Rect, Point};
use sdl2::pixels::*;
use sdl2::mouse::MouseButton;

use std::num::Wrapping;

use io::constants::*;
use io::graphics::Toggle;
use io::graphics::Drawable;
use cpu::constants::MemAddr;
use cpu::constants::CpuState;
use cpu::*;

use disasm;

/// State for the memory visualization system
pub struct MemVisState {
    pub tile_data_mode_button: Toggle<TileDataSelect>,
    pub mem_val_display_enabled: bool,
}

impl MemVisState {
    pub fn new() -> MemVisState {
        MemVisState {
            tile_data_mode_button: Toggle::new(Rect::new(MEM_DISP_WIDTH, MEM_DISP_HEIGHT, 24, 12),
                                               vec![TileDataSelect::Auto,
                                                    TileDataSelect::Mode1,
                                                    TileDataSelect::Mode2]),
            mem_val_display_enabled: true,
        }
    }

    /// Returns maybe a memory address given the coordinates of the memory visualization
    pub fn screen_coord_to_mem_addr(&self, point: Point) -> Option<MemAddr> {
        let x_scaled = point.x() as i32;
        let y_scaled = point.y() as i32;
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
    
    fn draw(&self, renderer: &mut sdl2::render::Renderer, cpu: &Cpu) {
        draw_memory_access(renderer, cpu);
        // // FIXME make this take imutable cpu arg
        // draw_memory_events(renderer, cpu);
    }
    
    /// Handle mouse click at pos. Prints some info about clicked
    /// address or jumps to it.
    fn click(&mut self, button: sdl2::mouse::MouseButton, position: Point, cpu: &mut Cpu) {
        match button {
            MouseButton::Left => {
                if let Some(pc) = self.screen_coord_to_mem_addr(position) {
                    print_address_info(pc, cpu);
                }
            },
            MouseButton::Right => {
                if let Some(pc) = self.screen_coord_to_mem_addr(position) {
                    info!("Jumping to ${:04X}", pc);
                    cpu.pc = pc;
                    if cpu.state != CpuState::Normal {
                        info!("CPU state was '{:?}', forcing run.", cpu.state);
                        cpu.state = CpuState::Normal;
                    }
                }
            },
            _ => (),
        }
    }
}



/// Returns point on screen where pixel representing address is drawn.
#[inline]
fn addr_to_point(addr: u16) -> Point {
    let x = (addr as i32) % MEM_DISP_WIDTH;
    let y = (addr as i32) / MEM_DISP_WIDTH;
    Point::new(x as i32, y as i32)
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
    (clamp_color(r1 as i16 + r2 as i16),
     clamp_color(g1 as i16 + g2 as i16),
     clamp_color(b1 as i16 + b2 as i16))
}


/// Use u8 value to scale other one.
// FIXME this is just lazy code
#[inline]
fn scale_col(scale: u8, color: u8) -> u8 {
    clamp_color((color as f32 * (scale as f32 / 255f32)) as i16)
}


/// Draw all memory values represented by pixels. Width is determined
/// by `MEM_DISP_WIDTH`.
pub fn draw_memory_values(renderer: &mut sdl2::render::Renderer, gameboy: &Cpu) {
    let mut x = 0;
    let mut y = 0;

    for &p in gameboy.mem.iter() {

        use sdl2::pixels::*;

        // renderer.set_draw_color(Color::RGB(r,g,b));
        // renderer.set_draw_color(Color::RGB(p as u8, p as u8, p as u8));
        renderer.set_draw_color(Color::RGB(0 as u8, 0 as u8, p as u8));
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
pub fn draw_memory_access(renderer: &mut sdl2::render::Renderer, gameboy: &Cpu) {
    let mut x = 0;
    let mut y = 0;

    let event_logger = match gameboy.event_logger {
        Some(ref logger) => logger,
        None => return,
    };


    for (addr, &p) in event_logger.access_flags.iter().enumerate() {

        use sdl2::pixels::*;

        let value = gameboy.mem[addr];

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
            Color::RGB(clamp_color(scale * ((p & FLAG_W) >> 1) as i16),
                       clamp_color(scale * (p & FLAG_R) as i16),
                       clamp_color(255 * ((p & FLAG_X) >> 2) as i16))
        };

        renderer.set_draw_color(color);


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
pub fn draw_memory_events(renderer: &mut sdl2::render::Renderer, gameboy: &mut Cpu) {
    // TODO: can be used to do partial "smart" redraw, and speed thing up.
    // But event logging itself is extremely slow

    let mut event_logger = match gameboy.event_logger {
        Some(ref mut logger) => logger,
        None => return,
    };

    // Remove events that are too old
    while !event_logger.events_deq.is_empty() {
        let timestamp = event_logger.events_deq.front().unwrap().timestamp;
        if (Wrapping(gameboy.cycles) - Wrapping(timestamp)).0 >= FADE_DELAY {
            event_logger.events_deq.pop_front();
        } else {
            break;
        }
    }

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
                        let val = gameboy.mem[addr as usize] as u8;
                        let (r, g, b) = mix_color(0, colval, 0, scale_col(colval, val / 2), 0, val);
                        renderer.set_draw_color(Color::RGB(r, g, b));
                        match renderer.draw_point(addr_to_point(addr)) {
                            Ok(_) => (),
                            Err(_) => error!("Cannot draw point at {:?}", addr_to_point(addr)),
                        }
                    }
                    CpuEvent::Write { to: addr } => {
                        let val = gameboy.mem[addr as usize] as u8;
                        let (r, g, b) = mix_color(colval, 0, 0, 0, scale_col(colval, val / 2), val);
                        renderer.set_draw_color(Color::RGB(r, g, b));
                        match renderer.draw_point(addr_to_point(addr)) {
                            Ok(_) => (),
                            Err(_) => error!("Cannot draw point at {:?}", addr_to_point(addr)),
                        }
                    }
                    CpuEvent::Execute(addr) => {
                        let val = gameboy.mem[addr as usize] as u8;
                        let (r, g, b) = mix_color(colval, colval, scale_col(colval, val), 0, 0, 0);
                        renderer.set_draw_color(Color::RGB(r, g, b));
                        match renderer.draw_point(addr_to_point(addr)) {
                            Ok(_) => (),
                            Err(_) => error!("Cannot draw point at {:?}", addr_to_point(addr)),
                        }
                    }
                    CpuEvent::Jump { from: src, to: dst } => {
                        renderer.set_draw_color(Color::RGB(colval, colval, 0));
                        let src_point = addr_to_point(src);
                        let dst_point = addr_to_point(dst);
                        // Horizontal lines are drawn with scaling but diagonal
                        // lines ignore it for some reason, which allows us to
                        // draw lines thinner than memory cells.
                        if src_point.y() != dst_point.y() {
                            match renderer.draw_line(src_point, dst_point) {
                                Ok(_) => (),
                                Err(_) => {
                                    error!("Cannot draw line from {:?} to {:?}",
                                           src_point,
                                           dst_point)
                                }
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
}

fn print_address_info(pc: MemAddr, cpu: &Cpu) {
    let pc = pc as usize;
    let mem = cpu.mem;
    let b1 = mem[pc + 1];
    let b2 = mem[pc + 2];
    let (mnem, _) = disasm::pp_opcode(mem[pc] as u8, b1 as u8, b2 as u8, pc as u16);
    let nn = byte_to_u16(b1, b2);
    println!("${:04X} {:16} 0x{:02X} 0x{:02X} 0x{:02X} 0x{:04X}",
             pc,
             mnem,
             mem[pc],
             b1,
             b2,
             nn);

}
