use std::collections::BTreeSet;

use crate::cpu::Cpu;
use crate::io::constants::*;

// window_x can be changed during scanline interrupts
// window_y is read once at the start of drawing and cached
// sprites with smaller x coords are drawn over ones with larger x coords (draw sprites from right to left?)
// when sprites with same x coords overlap, table ordering takes effect (0xFE00 is highest 0xFE04 is one lower)
// special rules for interpreting sprite pattern data in 8x16 mode
// if > 10 sprites per line then lower priority is removed... (so draw left to right but track each line the number of sprites drawn...)
// sprite priority bit: if on then it's drawn on top of Window and Background (easy)
//                      if off then it's then sprite is only drawn over color 0 of background and window (window and bg can't be transparent)

// interrupts:
// v-blank interrupt occurs at the start of the end of drawing
// LCDC status is used on each line?

pub fn deferred_renderer_draw_scanline(
    y: u8,
    cpu: &mut Cpu,
    window_counter: &mut u16,
) -> [u8; GB_SCREEN_WIDTH] {
    let mut bg_pixels = [0u8; GB_SCREEN_WIDTH];
    let mut bg_opacities = [false; GB_SCREEN_WIDTH];

    // invalid y, just return
    if y >= (GB_SCREEN_HEIGHT as u8) {
        return bg_pixels;
    }

    let scy = cpu.scy();
    let scx = cpu.scx();

    let tile_bg_map_addr = if cpu.lcdc_bg_tile_map() {
        TILE_MAP_2_START
    } else {
        TILE_MAP_1_START
    };
    let tile_win_map_addr = if cpu.lcdc_tile_map() {
        TILE_MAP_2_START
    } else {
        TILE_MAP_1_START
    };
    // TODO: could also be implemented with popcnt on a 64bit number
    let mut sprites_seen: BTreeSet<u8> = std::collections::BTreeSet::new();
    let adj_y = y.wrapping_add(scy) as u16;
    let (bg_color1, bg_color2, bg_color3, bg_color4) = cpu.bgp();
    let bg_colors = [bg_color1, bg_color2, bg_color3, bg_color4];
    let (sprite1_color1, sprite1_color2, sprite1_color3, sprite1_color4) = cpu.obp0();
    let (sprite2_color1, sprite2_color2, sprite2_color3, sprite2_color4) = cpu.obp1();
    let sprite_colors1 = [
        sprite1_color1,
        sprite1_color2,
        sprite1_color3,
        sprite1_color4,
    ];
    let sprite_colors2 = [
        sprite2_color1,
        sprite2_color2,
        sprite2_color3,
        sprite2_color4,
    ];

    let mut inc_window_counter = false;

    // (x_coord, obj_idx)
    let mut oam_objects: Vec<(u8, u8)> = (0..=40)
        .map(|obj_idx: u8| {
            let offset = OBJECT_ATTRIBUTE_START + (obj_idx as u16 * OBJECT_ATTRIBUTE_BLOCK_SIZE);
            let sprite_x: u8 = cpu.mem[offset + 1];
            (sprite_x, obj_idx)
        })
        .collect::<Vec<(u8, u8)>>();

    if !cpu.gbc_mode {
        // sort by x for DMG mode
        oam_objects.sort();
    }

    let row = adj_y >> 3;
    for x in 0..GB_SCREEN_WIDTH {
        let adj_x = (x as u8).wrapping_add(scx) as u16;
        if cpu.lcdc_bg_win_display() {
            let col = adj_x >> 3;
            let idx_into_tile_idx_mem = tile_bg_map_addr + (row << 5) + col;
            let tile_idx = cpu.mem[idx_into_tile_idx_mem];
            let tile_start = cpu.get_nth_background_tile(tile_idx as u16);

            // Lower 3 bits determine which line of the tile we're one
            // 1 line = 2 bytes, so we double it
            let tile_line = (adj_y & 0x7) * 2;
            // 8 choices for which pixel on the line we're on, so we take 3 bits here
            let tile_pixel = adj_x & 0x7;
            // pixels go from MSB to LSB within a tile
            let nth_pixel = 7 - tile_pixel;

            let tile_byte_1_bit = (cpu.mem[tile_start + tile_line] >> nth_pixel) & 0x1;
            let tile_byte_2_bit = (cpu.mem[tile_start + (tile_line + 1)] >> nth_pixel) & 0x1;
            let px_color = (tile_byte_2_bit << 1) | tile_byte_1_bit;

            bg_pixels[x] = bg_colors[px_color as usize];
            bg_opacities[x] = bg_opacities[x] || (px_color != 0);

            // window here
            // TODO: techincally win_x = 166 has special behavior but it may be
            // inconsistent due to hardware bugs
            if cpu.lcdc_window_on()
                && ((x as u8).wrapping_add(7) >= cpu.window_x_pos()//.wrapping_sub(7)
                    && (y as u8) >= cpu.window_y_pos())
                && (cpu.window_x_pos() < 166)
            {
                let win_y = (y as u16) - (cpu.window_y_pos() as u16);

                let win_x = cpu.window_x_pos().wrapping_sub(7);
                let win_x = if (adj_x as u8) >= win_x {
                    ((x as u8) - win_x) as u16
                } else {
                    adj_x
                };
                let row = *window_counter >> 3;
                let col = win_x >> 3;
                let idx_into_tile_idx_mem = tile_win_map_addr + (row << 5) + col;
                let tile_idx = cpu.mem[idx_into_tile_idx_mem];
                let tile_start = cpu.get_nth_background_tile(tile_idx as u16);

                // select the correct y pos based on lower 3 bits
                let tile_line = (win_y & 0x7) * 2;
                // 8 choices for which pixel on the line we're on, so we take 3 bits here
                let tile_pixel = adj_x & 0x7;
                // pixels go from MSB to LSB within a tile
                let nth_pixel = 7 - tile_pixel;

                let tile_byte_1_bit = (cpu.mem[tile_start + tile_line] >> nth_pixel) & 0x1;
                let tile_byte_2_bit = (cpu.mem[tile_start + (tile_line + 1)] >> nth_pixel) & 0x1;
                let px_color = (tile_byte_2_bit << 1) | tile_byte_1_bit;
                bg_pixels[x] = bg_colors[px_color as usize];
                bg_opacities[x] = bg_opacities[x] || (px_color != 0);

                inc_window_counter = true;
            }
        }

        if cpu.lcdc_sprite_display() {
            for &(_, obj_idx) in oam_objects.iter() {
                if sprites_seen.len() >= 10 && !sprites_seen.contains(&obj_idx) {
                    continue;
                }
                let offset =
                    OBJECT_ATTRIBUTE_START + (obj_idx as u16 * OBJECT_ATTRIBUTE_BLOCK_SIZE);
                let mut sprite_y: u8 = cpu.mem[offset];
                let mut sprite_x: u8 = cpu.mem[offset + 1];
                // TODO: figure out if condition below with all edge cases later
                let original_sprite_y = sprite_y;
                sprite_y = sprite_y.wrapping_sub(16);

                let sprite_y_size = if cpu.lcdc_sprite_size() { 16 } else { 8 };
                if ((x as u8) < sprite_x && (x as u8).wrapping_add(8) >= sprite_x)
                    && ((y as u8) < sprite_y.wrapping_add(sprite_y_size) && (y as u8) >= sprite_y)
                {
                    sprites_seen.insert(obj_idx);
                } else {
                    continue;
                }
                if (sprite_x == 0 || sprite_x >= 168)
                    && (original_sprite_y == 0 || original_sprite_y >= 160)
                {
                    // sprite is "hidden"
                    continue;
                }
                sprite_x = sprite_x.wrapping_sub(8);

                let tile_index: u8 = cpu.mem[offset + 2];
                // TODO implement flag handling (priority, flips...)
                let flags: u8 = cpu.mem[offset + 3];
                let x_flip = ((flags >> 5) & 1) == 1;
                let y_flip = ((flags >> 6) & 1) == 1;
                let win_bg_over_sprite = ((flags >> 7) & 1) == 1;
                let alt_palette = ((flags >> 4) & 1) == 1;

                // This table is fixed for OAM
                let pattern_table = TILE_PATTERN_TABLE_1_START;

                // a B c d e f g h
                let xth_pixel = (x as u8).wrapping_sub(sprite_x);
                let yth_pixel = (y as u8).wrapping_sub(sprite_y);

                let xth_pixel = if x_flip { 7 - xth_pixel } else { xth_pixel };
                let yth_pixel = if y_flip {
                    (sprite_y_size - 1) - yth_pixel
                } else {
                    yth_pixel
                };

                let tile_index = if cpu.lcdc_sprite_size() {
                    let tile_16 = tile_index & !1;
                    if yth_pixel >= 8 {
                        tile_16 + 1
                    } else {
                        tile_16
                    }
                } else {
                    tile_index
                };

                let tile_line = ((yth_pixel & 0x7) * 2) as u16;
                // 8 choices for which pixel on the line we're on, so we take 3 bits here
                let tile_pixel = xth_pixel & 0x7;
                // pixels go from MSB to LSB within a tile
                let nth_pixel = 7 - tile_pixel;
                let tile_start = pattern_table + (tile_index as u16 * 16);
                let tile_byte_1_bit = (cpu.mem[tile_start + tile_line] >> nth_pixel) & 0x1;
                let tile_byte_2_bit = (cpu.mem[tile_start + (tile_line + 1)] >> nth_pixel) & 0x1;
                let px_color = (tile_byte_2_bit << 1) | tile_byte_1_bit;
                let true_color = if alt_palette {
                    sprite_colors2[px_color as usize]
                } else {
                    sprite_colors1[px_color as usize]
                };

                // transparency
                if px_color != 0
                /*colors[color as usize] != 0*/
                /*color != 0*/
                {
                    if win_bg_over_sprite && bg_opacities[x] {
                        continue;
                    }
                    bg_pixels[x] = true_color;
                    // highest priority pixel value found here, we shouldn't draw anything else
                    break;
                }
            }
        }
    }

    if inc_window_counter {
        *window_counter += 1;
    }
    bg_pixels
}

// FF44(LY) LCDC Y coord
// FF45(LYC) value to compare the above to and set a flag (I think this is an interrupt)
pub fn deferred_renderer(cpu: &mut Cpu) -> [[u8; GB_SCREEN_WIDTH]; GB_SCREEN_HEIGHT] {
    let mut bg_pixels = [[0u8; GB_SCREEN_WIDTH]; GB_SCREEN_HEIGHT];

    let mut window_counter: u16 = 0;
    for y in 0..=(GB_SCREEN_HEIGHT + 9) {
        bg_pixels[y] = deferred_renderer_draw_scanline(y as u8, cpu, &mut window_counter);
    }

    bg_pixels
}
