//! Video RAM display

use sdl2;
use io::constants::*;
use cpu::*;

use sdl2::rect::Point;


// draw whole background buffer (256x256 px)
// pub fn draw_background_buffer(renderer: &mut sdl2::render::Renderer, gameboy: &Cpu) {}

pub fn draw_tile(renderer: &mut sdl2::render::Renderer,
                 gameboy: &Cpu,
                 mem_offset: u16,
                 tile_idx: u16, // technically when used by GB it's only 0-255
                 screen_offset_x: i32,
                 screen_offset_y: i32) {
    #[inline]
    fn get_bit(n: u8, offset: u8) -> u8 {
        (n >> (7 - offset)) & 1u8
    }
    
        for px in 0..TILE_SIZE_PX {
            for py in 0..TILE_SIZE_PX {
                let col_byte_off = py * 2;
                let offset = mem_offset + (tile_idx * TILE_SIZE_BYTES);
                let col_byte1_v = gameboy.mem[(offset + col_byte_off) as usize];
                let col_byte2_v = gameboy.mem[(offset + col_byte_off + 1) as usize];
                let col_bit_1 = get_bit(col_byte1_v, px as u8);
                let col_bit_2 = get_bit(col_byte2_v, px as u8);
                let px_color = (col_bit_2 << 1) | col_bit_1;

                // let d = 255/4;
                // let px_val = px_color*d;
                // renderer.set_draw_color(Color::RGB(px_val, px_val, px_val));

                let px_pal_col = TILE_PALETTE[px_color as usize];
                renderer.set_draw_color(px_pal_col);

                let point = Point::new(screen_offset_x + px as i32,
                                       screen_offset_y + py as i32);
                match renderer.draw_point(point) {
                    Ok(_) => (),
                    Err(_) => error!("Could not draw point at {:?}", point),
                };
            }
        }
    
}

// This is the dumbest and straightforward code for displaying Tile
// Patterns. It displays both background and sprite "tiles" as they
// overlap in memory.
pub fn draw_tile_patterns(renderer: &mut sdl2::render::Renderer,
                          gameboy: &Cpu,
                          screen_offset_x: i32) {

    for tile_idx in 0..(TILE_PATTERN_TABLES_SIZE / TILE_SIZE_BYTES) + 1 {

        let tile_start_x = (tile_idx % TILE_COLUMNS) * (TILE_SIZE_PX + BORDER_PX);
        let y_pos = tile_idx / TILE_COLUMNS;
        let tile_start_y = (TILE_SIZE_PX + BORDER_PX) * y_pos;

        draw_tile(renderer,
                  gameboy,
                  TILE_PATTERN_TABLE_1_START,
                  tile_idx,
                  tile_start_x as i32 + screen_offset_x,
                  tile_start_y as i32);
    }
}
