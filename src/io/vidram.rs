//! Video RAM display

use sdl2;
use io::constants::*;
use cpu::constants::MemAddr;
use cpu::*;

use sdl2::render::Renderer;
use sdl2::rect::Point;
use sdl2::rect::Rect;
use sdl2::pixels::*;


// draw whole background buffer (256x256 px)
// pub fn draw_background_buffer(renderer: &mut sdl2::render::Renderer, gameboy: &Cpu) {}

const TILE_PATTERN_TABLE_1_START: MemAddr = 0x8000;
const TILE_PATTERN_TABLE_1_END: MemAddr = 0x8FFF;
const TILE_PATTERN_TABLE_2_START: MemAddr = 0x8800;
const TILE_PATTERN_TABLE_2_END: MemAddr = 0x97FF;

// tables are overlapping
const TILE_PATTERN_TABLES_SIZE: MemAddr = TILE_PATTERN_TABLE_2_END - TILE_PATTERN_TABLE_1_START;

const TILE_SIZE_BYTES: u16 = 16;
const TILE_SIZE_PX: u16 = 8;
const BORDER_PX: u16 = 1;
const TILE_COLUMNS: u16 = 16;

const TILE_PALETTE: [Color; 4] = [Color::RGB(4, 5, 7),
                                  Color::RGB(235, 135, 140),
                                  Color::RGB(156, 146, 244),
                                  Color::RGB(252, 250, 175)];


// This is the dumbest and straightforward code for displaying Tile
// Patters. It displays both background and sprite "tiles", as they
// overlap.
pub fn draw_tile_patterns(renderer: &mut sdl2::render::Renderer,
                          gameboy: &Cpu,
                          screen_offset_x: i32) {
    #[inline]
    fn get_bit(n: u8, offset: u8) -> u8 {
        (n >> (7 - offset)) & 1u8
    }

    for tile in 0..(TILE_PATTERN_TABLES_SIZE / TILE_SIZE_BYTES) + 1 {

        let tile_start_x = (tile % TILE_COLUMNS) * (TILE_SIZE_PX + BORDER_PX);
        let y_pos = tile / TILE_COLUMNS;
        let tile_start_y = (TILE_SIZE_PX + BORDER_PX) * y_pos;

        for px in 0..TILE_SIZE_PX {
            for py in 0..TILE_SIZE_PX {
                let col_byte_off = py * 2;
                let offset = TILE_PATTERN_TABLE_1_START + (tile * TILE_SIZE_BYTES);
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

                let point = Point::new((tile_start_x + px) as i32 + screen_offset_x,
                                       (tile_start_y + py) as i32);
                renderer.draw_point(point);
            }
        }
    }
}
