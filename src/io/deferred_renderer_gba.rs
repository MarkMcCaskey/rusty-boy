use crate::gba::{self, PpuBgControl};
use crate::io::constants::*;

pub fn deferred_renderer_draw_rotated_bg(
    y: u8,
    gba: &mut gba::GameboyAdvance,
    bg_control: PpuBgControl,
    pa: i16,
    pc: i16,
    bg_x: i32,
    bg_y: i32,
) -> [(u8, u8, u8); GBA_SCREEN_WIDTH] {
    let mut bg_pixels = [(0u8, 0u8, 0u8); GBA_SCREEN_WIDTH];

    // TODO: for bg3 this should be + 0x600
    let map_base_ptr = bg_control.screen_base_block as u32 * 0x800; //+ 0x400;
    let tile_base_ptr = bg_control.character_base_block as u32 * 0x4000;
    for x in 0..GBA_SCREEN_WIDTH {
        let adj_y = (bg_y + (x as i32) * pc as i32) >> 8;
        let adj_x = (bg_x + (x as i32) * pa as i32) >> 8;

        if adj_x < 0
            || adj_x >= GBA_SCREEN_WIDTH as i32
            || adj_y < 0
            || adj_y >= GBA_SCREEN_HEIGHT as i32
        {
            continue;
        }

        let tile_col = (adj_x >> 3) as u32;
        let tile_row = (adj_y >> 3) as u32;
        let idx_into_tile_idx_mem = map_base_ptr + (tile_row * 32 * 2) + (tile_col * 2);
        let tile_idx_lo = gba.vram[idx_into_tile_idx_mem as usize] as u16;
        let tile_idx_hi = gba.vram[idx_into_tile_idx_mem as usize + 1] as u16;
        let tile_num = ((tile_idx_hi & 0x3) << 8) | tile_idx_lo;
        let horizontal_flip = (tile_idx_hi & 0x4) != 0;
        let vertical_flip = (tile_idx_hi & 0x8) != 0;
        //let palette_num = tile_idx_hi >> 4;

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

        let tile_line = nth_line * 8;

        let tile_start = tile_base_ptr as usize + (tile_num as usize * 8 * 8);
        let tile_line_start = tile_start + tile_line as usize;
        let tile_byte_start = tile_line_start + (nth_pixel >> 1) as usize;
        let color_8bit = gba.vram[tile_byte_start];

        let color_lo = gba.obj_palette_ram[color_8bit as usize * 2];
        let color_hi = gba.obj_palette_ram[(color_8bit as usize * 2) + 1];
        let red = color_lo & 0x1F;
        let green = ((color_hi & 0x3) << 3) | (color_lo >> 5);
        let blue = (color_hi >> 2) & 0x1F;

        bg_pixels[x as usize] = (red << 3, green << 3, blue << 3);
    }

    bg_pixels
}

pub fn deferred_renderer_draw_gba_bg4(
    y: u8,
    gba: &mut gba::GameboyAdvance,
) -> [(u8, u8, u8); GBA_SCREEN_WIDTH] {
    let mut bg_pixels = [(0u8, 0u8, 0u8); GBA_SCREEN_WIDTH];
    let bg2_control = gba.ppu_bg2_control();

    let base = if gba.ppu_frame_select() { 0xA000 } else { 0 };
    /*
    for i in 0x4C..=(0x4C + 4) {
        if gba.io_registers[i] != 0 {
            panic!("mode 4 is using mosaic! at {} 0x{:X}", i, gba.io_registers[i])
        }
    }
    */
    /*

    for i in 0x14000..=0x17FFF {
        if gba.vram[i] != 0 {
            panic!("mode 4 is using objects! at {} 0x{:X}", i, gba.vram[i])
        }
    }
    */
    let pa = gba.io_registers.bg2_rotation.pa;
    let pc = gba.io_registers.bg2_rotation.pc;
    let bg_x = gba.io_registers.bg2_rotation.cached_x;
    let bg_y = gba.io_registers.bg2_rotation.cached_y;

    for x in 0..GBA_SCREEN_WIDTH {
        let adj_y = (bg_y + (x as i32) * pc as i32) >> 8;
        let adj_x = (bg_x + (x as i32) * pa as i32) >> 8;

        if adj_x < 0
            || adj_x >= GBA_SCREEN_WIDTH as i32
            || adj_y < 0
            || adj_y >= GBA_SCREEN_HEIGHT as i32
        {
            continue;
        }

        let idx = (adj_y * 240 + adj_x) as usize;
        let palette_idx = gba.vram[base + idx] as usize;
        if palette_idx == 0 {
            continue;
        }
        let color_lo = gba.obj_palette_ram[palette_idx * 2];
        let color_hi = gba.obj_palette_ram[palette_idx * 2 + 1];

        let red = color_lo & 0x1F;
        let green = ((color_hi & 0x3) << 3) | (color_lo >> 5);
        let blue = (color_hi >> 2) & 0x1F;

        bg_pixels[x as usize] = (red << 3, green << 3, blue << 3);
    }

    bg_pixels
}

pub fn deferred_renderer_draw_gba_scanline(
    y: u8,
    gba: &mut gba::GameboyAdvance,
) -> [(u8, u8, u8); GBA_SCREEN_WIDTH] {
    match gba.ppu_bg_mode() {
        2 => {
            let pa = gba.io_registers.bg2_rotation.pa;
            let pc = gba.io_registers.bg2_rotation.pc;
            let bg_x = gba.io_registers.bg2_rotation.cached_x;
            let bg_y = gba.io_registers.bg2_rotation.cached_y;
            // TODO: also blend with bg 3
            return deferred_renderer_draw_rotated_bg(
                y,
                gba,
                gba.ppu_bg2_control(),
                pa,
                pc,
                bg_x,
                bg_y,
            );
        }
        3 => todo!("bg mode 3"),
        4 => return deferred_renderer_draw_gba_bg4(y, gba),
        5 => todo!("bg mode 5"),
        _ => (),
    }
    let mut bg_pixels = [(0u8, 0u8, 0u8); GBA_SCREEN_WIDTH];

    let scx = gba.ppu_bg0_x_scroll();
    let scy = gba.ppu_bg0_y_scroll();
    let bg0_control = gba.ppu_bg0_control();
    let (screen_x_size_mask, screen_y_size_mask) =
    // ASSUMES TEXT MODE DURING EARLY DEVELOPMENT
    // TOOD: add support for other mode here
        match bg0_control.screen_size {
            0 => (0xFF, 0xFF),
            1 => (0x1FF, 0xFF),
            2 => (0xFF, 0x1FF),
            3 => (0x1FF, 0x1FF),
            _ => unreachable!(),
        };

    //dbg!(gba.ppu_bg0_control());
    //dbg!(gba.ppu_bg1_control());
    //dbg!(gba.ppu_bg2_control());
    //dbg!(gba.ppu_bg3_control());
    if bg0_control.mosaic {
        todo!("Mosaic mode!");
    }
    let adj_y = (y as u16).wrapping_add(scy) as u16 & screen_y_size_mask;
    // this address is auto-incremented by 2kb for each background
    let map_base_ptr = bg0_control.screen_base_block as u32 * 0x800;
    let tile_base_ptr = bg0_control.character_base_block as u32 * 0x4000;

    let tile_row = (adj_y >> 3) as u32;
    for x in 0..GBA_SCREEN_WIDTH {
        let adj_x = (x as u16).wrapping_add(scx) as u16 & screen_x_size_mask;
        let tile_col = (adj_x >> 3) as u32;
        let idx_into_tile_idx_mem = map_base_ptr + (tile_row * 32 * 2) + (tile_col * 2);
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

        /*
        if tile_num != 0 || palette_num != 0 {
            dbg!(adj_x, adj_y, tile_num, palette_num);
            panic!("found!");
        }
        */

        if bg0_control.color_mode {
            todo!()
        } else {
            // 16/16 mode
            // 4bit palette index so 4 bytes per line = 8 palette indices per line
            // 4 bytes per line * 8 lines = 32 bytes per tile
            let tile_line = nth_line * 4;

            let tile_start = tile_base_ptr as usize + (tile_num as usize * 32);
            let tile_line_start = tile_start + tile_line as usize;
            let tile_byte_start = tile_line_start + (nth_pixel >> 1) as usize;
            let color_4bit = (gba.vram[tile_byte_start] >> ((nth_pixel & 0x1) * 4)) & 0xF;
            // HACK: hello world wants this
            //let color_4bit = color_4bit + 1;

            // 2 bytes per color, 16 colors per palette
            let palette_start = palette_num as usize * 16 * 2;
            /*
            let mut found = false;
            for i in (palette_start)..(0x400 -palette_start) {
                if gba.obj_palette_ram[i] != 0 {
                    dbg!(i, gba.obj_palette_ram[i], color_4bit);
                    found = true;
                }
            }
            if found { panic!("found "); }
            */
            let color_lo = gba.obj_palette_ram[palette_start + (color_4bit as usize * 2)];
            let color_hi = gba.obj_palette_ram[palette_start + (color_4bit as usize * 2) + 1];
            let red = color_lo & 0x1F;
            let green = ((color_hi & 0x3) << 3) | (color_lo >> 5);
            let blue = (color_hi >> 2) & 0x1F;
            if red | green | blue != 0 {
                //panic!("COLOR!");
            }

            bg_pixels[x as usize] = (red << 3, green << 3, blue << 3);
        }
    }

    bg_pixels
}
