use crate::gba;
use crate::io::constants::*;

pub fn deferred_renderer_draw_gba_scanline(
    y: u8,
    gba: &mut gba::GameboyAdvance,
) -> [(u8, u8, u8); GBA_SCREEN_WIDTH] {
    let mut bg_pixels = [(0u8, 0u8, 0u8); GBA_SCREEN_WIDTH];

    let scx = gba.ppu_bg0_x_scroll();
    let scy = gba.ppu_bg0_y_scroll();
    let adj_y = (y as u16).wrapping_add(scy) as u16 & 0x1FF;
    let bg0_control = gba.ppu_bg0_control();
    let map_base_ptr = bg0_control.screen_base_block as u32 * 0x800;
    let tile_base_ptr = bg0_control.character_base_block as u32 * 0x4000;

    let row = (adj_y >> 3) as u32;
    for x in 0..GBA_SCREEN_WIDTH {
        let adj_x = (x as u16).wrapping_add(scx) as u16 & 0x1FF;
        let col = (adj_x >> 3) as u32;
        let idx_into_tile_idx_mem = map_base_ptr + (row << 5) + col;
        let tile_idx_lo = gba.vram[idx_into_tile_idx_mem as usize] as u16;
        let tile_idx_hi = gba.vram[idx_into_tile_idx_mem as usize + 1] as u16;
        let tile_num = ((tile_idx_hi & 0x3) << 8) | tile_idx_lo;
        let horizontal_flip = (tile_idx_hi & 0x4) != 0;
        let vertical_flip = (tile_idx_hi & 0x8) != 0;
        let palette_num = tile_idx_hi >> 4;

        // Lower 3 bits determine which line of the tile we're on
        let mut nth_line = adj_y & 0x7;
        // 8 choices for which pixel on the line we're on, so we take 3 bits here
        let tile_pixel = adj_x & 0x7;
        // pixels go from MSB to LSB within a tile
        let mut nth_pixel = 7 - tile_pixel;
        if vertical_flip {
            nth_line = 7 - nth_line;
        }
        if horizontal_flip {
            nth_pixel = 7 - nth_pixel;
        }

        // 16/16 mode
        let tile_line = nth_line * 4;

        let tile_start = tile_base_ptr as usize + (tile_num as usize * 32);
        let tile_line_start = tile_start + tile_line as usize;
        let tile_byte_start = tile_line_start + (nth_pixel >> 1) as usize;
        let color_4bit = gba.vram[tile_byte_start] >> (nth_pixel & 0x1);

        let palette_start = palette_num as usize * 16;
        let color_lo = gba.obj_palette_ram[palette_start + (color_4bit as usize * 2)];
        let color_hi = gba.obj_palette_ram[palette_start + (color_4bit as usize * 2) + 1];
        let red = color_lo & 0x1F;
        let green = ((color_hi & 0x3) << 3) | (color_lo >> 5);
        let blue = (color_hi >> 2) & 0x1F;
        if (red | green | blue) != 0 {
            panic!("COLOR!");
        }

        bg_pixels[x as usize] = (red << 3, green << 3, blue << 3);
    }

    bg_pixels
}
