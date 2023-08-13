use crate::gba::{self, PpuBgControl};
use crate::io::constants::*;

#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
pub struct Rgb15(u16);

impl Rgb15 {
    pub const fn new(red: u8, green: u8, blue: u8) -> Self {
        let red = red >> 3;
        let green = green >> 3;
        let blue = blue >> 3;
        let color = (blue as u16) << 10 | (green as u16) << 5 | red as u16;
        Rgb15(color)
    }
    pub const fn from_bits(bits: u16) -> Self {
        Rgb15(bits & 0x7FFF)
    }
    pub const fn from_lo_hi(lo: u8, hi: u8) -> Self {
        let color = ((hi as u16) << 8) | lo as u16;
        Rgb15(color & 0x7FFF)
    }
    pub const fn transparent() -> Self {
        Rgb15(0x8000)
    }
    pub const fn black() -> Self {
        Rgb15(0)
    }
    pub const fn white() -> Self {
        Rgb15(0x7FFF)
    }
    pub const fn is_transparent(self) -> bool {
        self.0 & 0x8000 != 0
    }
    pub const fn to_rgb(self) -> (u8, u8, u8) {
        let red = (self.0 & 0x1F) as u8;
        let green = ((self.0 >> 5) & 0x1F) as u8;
        let blue = ((self.0 >> 10) & 0x1F) as u8;
        (red << 3, green << 3, blue << 3)
    }
    // TODO: to_rgba with specific alpha values determined by blending mode
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ObjectMode {
    Normal,
    SemiTransparent,
    ObjectWindow,
    Prohibited,
}

impl ObjectMode {
    pub fn from_bits(bits: u8) -> Self {
        match bits {
            0 => ObjectMode::Normal,
            1 => ObjectMode::SemiTransparent,
            2 => ObjectMode::ObjectWindow,
            3 => ObjectMode::Prohibited,
            _ => panic!("Invalid object mode: {}", bits),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ObjectShape {
    Square,
    Horizontal,
    Vertical,
    Prohibited,
}

impl ObjectShape {
    pub fn from_bits(bits: u8) -> Self {
        match bits {
            0 => ObjectShape::Square,
            1 => ObjectShape::Horizontal,
            2 => ObjectShape::Vertical,
            3 => ObjectShape::Prohibited,
            _ => panic!("Invalid object shape: {}", bits),
        }
    }
}

#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
pub struct Object(u64);

impl Object {
    pub fn from_bits(bits: u64) -> Self {
        Object(bits)
    }
    pub fn from_bytes(bytes: [u8; 8]) -> Self {
        Object(u64::from_le_bytes(bytes))
    }
    pub fn y(self) -> i32 {
        let mut val = (self.0 & 0xFF) as i32;
        if val >= (GBA_SCREEN_HEIGHT as i32) {
            val -= 1 << 8;
        }
        val
    }
    pub fn x(self) -> i32 {
        let mut val = ((self.0 >> 16) & 0x1FF) as i32;
        if val >= (GBA_SCREEN_WIDTH as i32) {
            val -= 1 << 9;
        }
        val
    }
    pub fn rotation_enabled(self) -> bool {
        self.0 & 0x100 != 0
    }
    pub fn double_size(self) -> bool {
        self.0 & 0x200 != 0
    }
    pub fn object_disabled(self) -> bool {
        (self.0 >> 8) & 1 == 0 && (self.0 >> 9) & 1 == 1
    }
    pub fn object_mode(self) -> ObjectMode {
        ObjectMode::from_bits(((self.0 >> 10) & 0x3) as u8)
    }
    pub fn mosaic(self) -> bool {
        (self.0 >> 12) & 1 == 1
    }
    pub fn full_color(self) -> bool {
        (self.0 >> 13) & 1 == 1
    }
    pub fn shape(self) -> ObjectShape {
        ObjectShape::from_bits(((self.0 >> 14) & 0x3) as u8)
    }
    pub fn rotation_scaling_param_selection(self) -> u8 {
        ((self.0 >> (16 + 9)) & 0x1F) as u8
    }
    pub fn horizontal_flip(self) -> bool {
        (self.0 >> (16 + 12)) & 1 == 1
    }
    pub fn vertical_flip(self) -> bool {
        (self.0 >> (16 + 13)) & 1 == 1
    }
    pub fn size(self) -> (u8, u8) {
        let shape = self.shape();
        let size_idx = ((self.0 >> (16 + 14)) & 0x3) as u8;
        match (size_idx, shape) {
            (0, ObjectShape::Square) => (8, 8),
            (0, ObjectShape::Horizontal) => (16, 8),
            (0, ObjectShape::Vertical) => (8, 16),
            (1, ObjectShape::Square) => (16, 16),
            (1, ObjectShape::Horizontal) => (32, 8),
            (1, ObjectShape::Vertical) => (8, 32),
            (2, ObjectShape::Square) => (32, 32),
            (2, ObjectShape::Horizontal) => (32, 16),
            (2, ObjectShape::Vertical) => (16, 32),
            (3, ObjectShape::Square) => (64, 64),
            (3, ObjectShape::Horizontal) => (64, 32),
            (3, ObjectShape::Vertical) => (32, 64),
            (_, ObjectShape::Prohibited) => panic!("Invalid object shape: {:?}", shape),
            _ => unreachable!(),
        }
    }
    pub fn character_name(self) -> u16 {
        ((self.0 >> 32) & 0x3FF) as u16
    }
    pub fn priority(self) -> u8 {
        ((self.0 >> (32 + 10)) & 0x3) as u8
    }
    pub fn palette_number(self) -> u8 {
        ((self.0 >> (32 + 12)) & 0xF) as u8
    }
}

pub struct ObjectIter<'a> {
    gba: &'a gba::GameboyAdvance,
    idx: usize,
}

impl<'a> ObjectIter<'a> {
    pub fn new(gba: &'a gba::GameboyAdvance) -> Self {
        ObjectIter { gba, idx: 0 }
    }
}

impl<'a> std::iter::Iterator for ObjectIter<'a> {
    type Item = Object;

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx >= 128 {
            None
        } else {
            let obj = Object::from_bytes([
                self.gba.oam[(self.idx * 8) + 0],
                self.gba.oam[(self.idx * 8) + 1],
                self.gba.oam[(self.idx * 8) + 2],
                self.gba.oam[(self.idx * 8) + 3],
                self.gba.oam[(self.idx * 8) + 4],
                self.gba.oam[(self.idx * 8) + 5],
                0,
                0,
            ]);
            self.idx += 1;
            Some(obj)
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (128 - self.idx, Some(128 - self.idx))
    }
}

impl<'a> std::iter::ExactSizeIterator for ObjectIter<'a> {}

fn get_object_rotation_data(gba: &gba::GameboyAdvance, index: u8) -> (i32, i32, i32, i32) {
    let index = 0x6 + (index as usize * 32);
    let pa = (((gba.oam[index + 1] as i16) << 8) | gba.oam[index] as i16) as i32;
    let pb = (((gba.oam[index + 1 + 8] as i16) << 8) | gba.oam[index + 8] as i16) as i32;
    let pc = (((gba.oam[index + 1 + 16] as i16) << 8) | gba.oam[index + 16] as i16) as i32;
    let pd = (((gba.oam[index + 1 + 24] as i16) << 8) | gba.oam[index + 24] as i16) as i32;
    (pa, pb, pc, pd)
}

pub fn deferred_renderer_draw_rotated_bg(
    y: u8,
    gba: &mut gba::GameboyAdvance,
    bg_control: PpuBgControl,
    pa: i16,
    pc: i16,
    bg_x: i32,
    bg_y: i32,
) -> [Rgb15; GBA_SCREEN_WIDTH] {
    let mut bg_pixels = [Rgb15::transparent(); GBA_SCREEN_WIDTH];

    // TODO: for bg3 this should be + 0x600
    let map_base_ptr = bg_control.screen_base_block as u32 * 0x800; //+ 0x400;
    let tile_base_ptr = bg_control.character_base_block as u32 * 0x4000;
    for x in 0..GBA_SCREEN_WIDTH {
        // TODO: review use of x/y here
        let adj_y = (bg_y + (x as i32) * pc as i32) >> 8;
        let adj_x = (bg_x + (x as i32) * pa as i32) >> 8;

        if adj_x < 0
            || adj_x >= GBA_SCREEN_WIDTH as i32
            || adj_y < 0
            || adj_y >= GBA_SCREEN_HEIGHT as i32
        //|| y != adj_y as u8
        {
            bg_pixels[x as usize] = Rgb15::transparent();
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

        if color_8bit == 0 {
            bg_pixels[x as usize] = Rgb15::transparent();
            continue;
        }

        let color_lo = gba.obj_palette_ram[color_8bit as usize * 2];
        let color_hi = gba.obj_palette_ram[(color_8bit as usize * 2) + 1];

        bg_pixels[x as usize] = Rgb15::from_lo_hi(color_lo, color_hi);
    }

    bg_pixels
}

pub fn deferred_renderer_draw_gba_bg4(
    y: u8,
    gba: &mut gba::GameboyAdvance,
) -> [Rgb15; GBA_SCREEN_WIDTH] {
    let mut bg_pixels = [Rgb15::transparent(); GBA_SCREEN_WIDTH];
    let bg2_control = gba.ppu_bg2_control();

    let base = if gba.ppu_frame_select() { 0xA000 } else { 0 };
    /*
    for i in 0x4C..=(0x4C + 4) {
        if gba.io_registers[i] != 0 {
            panic!("mode 4 is using mosaic! at {} 0x{:X}", i, gba.io_registers[i])
        }
    }
    */
    let pa = gba.io_registers.bg2_rotation.pa;
    let pc = gba.io_registers.bg2_rotation.pc;
    let bg_x = gba.io_registers.bg2_rotation.cached_x;
    let bg_y = gba.io_registers.bg2_rotation.cached_y;

    /*
    use std::hash::Hasher;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for i in 0..0x02000 {
        hasher.write_u8(gba.vram[i]);
    }
    dbg!(hasher.finish());
    */

    for x in 0..GBA_SCREEN_WIDTH {
        // TODO: review this formula
        // found evidence that adj_y uses x as well here
        let adj_y = (bg_y + (x as i32) * pc as i32) >> 8;
        let adj_x = (bg_x + (x as i32) * pa as i32) >> 8;

        if adj_x < 0
            || adj_x >= GBA_SCREEN_WIDTH as i32
            || adj_y < 0
            || adj_y >= GBA_SCREEN_HEIGHT as i32
        {
            bg_pixels[x as usize] = Rgb15::transparent();
            continue;
        }

        let idx = (adj_y * 240 + adj_x) as usize;

        let palette_idx = gba.vram[base + idx] as usize;
        if palette_idx == 0 {
            bg_pixels[x as usize] = Rgb15::transparent();
            continue;
        }
        let color_lo = gba.obj_palette_ram[palette_idx * 2];
        let color_hi = gba.obj_palette_ram[palette_idx * 2 + 1];

        bg_pixels[x as usize] = Rgb15::from_lo_hi(color_lo, color_hi);
    }

    bg_pixels
}

pub fn deferred_renderer_draw_gba_mode_3(
    y: u8,
    gba: &mut gba::GameboyAdvance,
) -> [Rgb15; GBA_SCREEN_WIDTH] {
    let mut bg_pixels = [Rgb15::transparent(); GBA_SCREEN_WIDTH];
    //let bg2_control = gba.ppu_bg2_control();

    let pa = gba.io_registers.bg2_rotation.pa;
    let pc = gba.io_registers.bg2_rotation.pc;
    let bg_x = gba.io_registers.bg2_rotation.cached_x;
    let bg_y = gba.io_registers.bg2_rotation.cached_y;

    for x in 0..GBA_SCREEN_WIDTH {
        // TODO: review this formula
        let adj_y = (bg_y + (x as i32) * pc as i32) >> 8;
        let adj_x = (bg_x + (x as i32) * pa as i32) >> 8;

        if adj_x < 0
            || adj_x >= GBA_SCREEN_WIDTH as i32
            || adj_y < 0
            || adj_y >= GBA_SCREEN_HEIGHT as i32
        {
            bg_pixels[x as usize] = Rgb15::transparent();
            continue;
        }

        let idx = (adj_y * 240 + adj_x) as usize;
        let color_lo = gba.vram[idx * 2];
        let color_hi = gba.vram[idx * 2 + 1];

        bg_pixels[x as usize] = Rgb15::from_lo_hi(color_lo, color_hi);
    }

    bg_pixels
}

pub fn deferred_renderer_draw_gba_bg3(
    y: u8,
    gba: &mut gba::GameboyAdvance,
) -> [Rgb15; GBA_SCREEN_WIDTH] {
    // bg3 doesn't support transparency, use base color here?
    let mut bg_pixels = [Rgb15::transparent(); GBA_SCREEN_WIDTH];

    let pa = gba.io_registers.bg3_rotation.pa;
    let pc = gba.io_registers.bg3_rotation.pc;
    let bg_x = gba.io_registers.bg3_rotation.cached_x;
    let bg_y = gba.io_registers.bg3_rotation.cached_y;

    for x in 0..GBA_SCREEN_WIDTH {
        let adj_y = (bg_y + (y as i32) * pc as i32) >> 8;
        let adj_x = (bg_x + (x as i32) * pa as i32) >> 8;

        if adj_x < 0
            || adj_x >= GBA_SCREEN_WIDTH as i32
            || adj_y < 0
            || adj_y >= GBA_SCREEN_HEIGHT as i32
        {
            bg_pixels[x as usize] = Rgb15::transparent();
            continue;
        }

        let idx = (adj_y * 240 + adj_x) as usize;
        let color_lo = gba.vram[idx * 2];
        let color_hi = gba.vram[idx * 2 + 1];

        bg_pixels[x as usize] = Rgb15::from_lo_hi(color_lo, color_hi);
    }

    bg_pixels
}

fn get_full_color_pixel(
    gba: &gba::GameboyAdvance,
    base_ptr: usize,
    tile_num: u16,
    (tile_row, tile_col): (u32, u32),
    (nth_line, nth_pixel): (u16, u16),
    tile_array_width: usize,
    obj_palette: bool,
) -> Rgb15 {
    let tile_line = nth_line * 8;
    let tile_size = 64;

    let tile_start = base_ptr + (tile_num as usize * 32);
    let offset = (tile_row as usize * tile_array_width) + tile_col as usize;
    let tile_addr = tile_start + (offset * tile_size);
    let pixel_addr = tile_addr + tile_line as usize + nth_pixel as usize;
    let color_8bit = gba.vram[pixel_addr];

    if color_8bit == 0 {
        return Rgb15::transparent();
    }

    let palette_offset = if obj_palette { 0x200 } else { 0 };

    let color_lo = gba.obj_palette_ram[palette_offset + color_8bit as usize * 2];
    let color_hi = gba.obj_palette_ram[palette_offset + (color_8bit as usize * 2) + 1];

    Rgb15::from_lo_hi(color_lo, color_hi)
}

fn get_4bit_color_pixel(
    gba: &gba::GameboyAdvance,
    base_ptr: usize,
    tile_num: u16,
    (tile_row, tile_col): (u32, u32),
    (nth_line, nth_pixel): (u16, u16),
    tile_array_width: usize,
    palette_num: u8,
    obj_palette: bool,
) -> Rgb15 {
    // 16/16 mode
    // 4bit palette index so 4 bytes per line = 8 palette indices per line
    // 4 bytes per line * 8 lines = 32 bytes per tile
    let tile_line = nth_line * 4;
    let tile_size = 32;

    let tile_start = base_ptr + (tile_num as usize * 32);
    let offset = (tile_row as usize * tile_array_width) + tile_col as usize;
    let tile_addr = tile_start + (offset * tile_size);
    let pixel_addr = tile_addr + tile_line as usize + (nth_pixel as usize >> 1);
    /*
    if pixel_addr >= gba.vram.len() {
        dbg!(pixel_addr, base_ptr, tile_num, tile_row, tile_col, nth_line, nth_pixel);
        return Rgb15::transparent();
    }
    */
    let color_4bit = (gba.vram[pixel_addr] >> ((nth_pixel & 0x1) * 4)) & 0xF;

    if color_4bit == 0 {
        return Rgb15::transparent();
    }

    let palette_offset = if obj_palette { 0x200 } else { 0 };
    // 2 bytes per color, 16 colors per palette
    let palette_start = palette_offset + (palette_num as usize * 16 * 2);

    let color_lo = gba.obj_palette_ram[palette_start + (color_4bit as usize * 2)];
    let color_hi = gba.obj_palette_ram[palette_start + (color_4bit as usize * 2) + 1];

    Rgb15::from_lo_hi(color_lo, color_hi)
}

pub fn deferred_renderer_draw_gba_bg(
    y: u8,
    bg: u8,
    bg_control: PpuBgControl,
    gba: &mut gba::GameboyAdvance,
) -> [Rgb15; GBA_SCREEN_WIDTH] {
    let mut bg_pixels = [Rgb15::transparent(); GBA_SCREEN_WIDTH];

    let (scx, scy) = gba.ppu_bg_scroll(bg);
    let (screen_x_size_mask, screen_y_size_mask) = match bg_control.screen_size {
        0 => (0xFF, 0xFF),
        1 => (0x1FF, 0xFF),
        2 => (0xFF, 0x1FF),
        3 => (0x1FF, 0x1FF),
        _ => unreachable!(),
    };

    /*
            let mut sbb = match (bg_width, bg_height) {
        (256, 256) => 0,
        (512, 256) => bg_x / 256,
        (256, 512) => bg_y / 256,
        (512, 512) => index2d!(u32, bg_x / 256, bg_y / 256, 2),
        _ => unreachable!(),
    } as u32;
     */

    let mosaic_x = gba.ppu_bg_mosaic_h();
    let mosaic_y = gba.ppu_bg_mosaic_v();

    let adj_y = (y as u16).wrapping_add(scy) as u16 & screen_y_size_mask;
    let adj_y = if bg_control.mosaic {
        adj_y - (adj_y % mosaic_y as u16)
    } else {
        adj_y
    };
    // this address is auto-incremented by 2kb for each background
    let tile_base_ptr = bg_control.character_base_block as usize * 0x4000;

    let tile_row = (adj_y >> 3) as u32 & 0x1F;
    for x in 0..GBA_SCREEN_WIDTH {
        let adj_x = (x as u16).wrapping_add(scx) as u16 & screen_x_size_mask;
        let adj_x = if bg_control.mosaic {
            adj_x - (adj_x % mosaic_x as u16)
        } else {
            adj_x
        };

        let screen_base_block = match bg_control.screen_size {
            0 => 0,
            1 => adj_x >> 8,
            2 => adj_y >> 8,
            3 => ((adj_y >> 8) * 2) + (adj_x >> 8),
            _ => unreachable!(),
        };
        let sbb = bg_control.screen_base_block as u32 + screen_base_block as u32;
        // if width is 512, there's an extra edge case here with SBB selection
        let map_base_ptr = sbb * 0x800;

        let tile_col = (adj_x >> 3) as u32 & 0x1F;
        let tile_map_idx = (tile_row * 32) + tile_col;
        let idx_into_tile_idx_mem = map_base_ptr + (tile_map_idx * 2);
        let tile_idx_lo = gba.vram[idx_into_tile_idx_mem as usize] as u16;
        let tile_idx_hi = gba.vram[idx_into_tile_idx_mem as usize + 1] as u16;
        let tile_num = ((tile_idx_hi & 0x3) << 8) | tile_idx_lo;

        let horizontal_flip = (tile_idx_hi & 0x4) != 0;
        let vertical_flip = (tile_idx_hi & 0x8) != 0;
        let palette_num = (tile_idx_hi >> 4) as u8;

        // Lower 3 bits determine which line of the tile we're on
        let mut nth_line = adj_y & 0x7;
        // 8 choices for which pixel on the line we're on, so we take 3 bits here
        let tile_pixel = adj_x & 0x7;
        // pixels go from MSB to LSB within a tile
        //let mut nth_pixel = 7 - tile_pixel;
        let mut nth_pixel = tile_pixel;
        if vertical_flip {
            nth_line = 7 - nth_line;
        }
        if horizontal_flip {
            nth_pixel = 7 - nth_pixel;
        }

        if bg_control.color_mode {
            // TODO: figure out this value
            let tile_array_width = 0;
            let color = get_full_color_pixel(
                gba,
                tile_base_ptr,
                tile_num,
                //(tile_row, tile_col),
                (0, 0),
                (nth_line, nth_pixel),
                tile_array_width,
                false,
            );

            bg_pixels[x] = color;
        } else {
            let tile_ptr = tile_base_ptr + (tile_num as usize * 32);
            //let tile_addr = ((gba.vram[tile_ptr] as u16) | ((gba.vram[tile_ptr + 1] as u16) << 8)) as usize;
            //let tile_addr = ((gba.vram[tile_ptr] as u16) | ((gba.vram[tile_ptr + 1] as u16) << 8)) as usize;
            let pixel_addr = tile_ptr + (nth_line as usize * 4) + (nth_pixel as usize >> 1);
            let color_4bit = (gba.vram[pixel_addr] >> ((nth_pixel & 0x1) * 4)) & 0xF;

            if color_4bit == 0 {
                continue;
            }

            // 2 bytes per color, 16 colors per palette
            let palette_start = palette_num as usize * 16 * 2;

            let color_lo = gba.obj_palette_ram[palette_start + (color_4bit as usize * 2)];
            let color_hi = gba.obj_palette_ram[palette_start + (color_4bit as usize * 2) + 1];

            let color = Rgb15::from_lo_hi(color_lo, color_hi);

            /*
            let tile_array_width = 0;
            let color = get_4bit_color_pixel(
                gba,
                tile_base_ptr,
                tile_num,
                //(tile_row, tile_col),
                (0,0),
                (nth_line, nth_pixel),
                tile_array_width,
                palette_num,
                false,
            );
            */

            bg_pixels[x as usize] = color;
        }
    }

    bg_pixels
}

fn naive_object_layer(
    y: u8,
    gba: &gba::GameboyAdvance,
    obj_base_ptr: usize,
) -> [Rgb15; GBA_SCREEN_WIDTH] {
    let mut pixels = [Rgb15::transparent(); GBA_SCREEN_WIDTH];
    let obj_iter = ObjectIter::new(&gba);
    let obj_order = obj_iter
        .filter(|obj| !obj.object_disabled())
        .filter(|obj| !(obj.x() == 0 || obj.y() == 0))
        .filter(|obj| {
            let (_, y_size) = obj.size();
            if obj.rotation_enabled() {
                let bbox_y = if obj.double_size() {
                    y_size as i32 * 2
                } else {
                    y_size as i32
                };
                (y as i32) >= obj.y() && (y as i32) < obj.y() + bbox_y
            } else {
                obj.y() as u16 <= y as u16 && (y as u16) < (obj.y() as u16) + y_size as u16
            }
        })
        .filter(|obj| obj.object_mode() != ObjectMode::Prohibited)
        .collect::<Vec<_>>();
    // TODO: sort by priority

    for x in 0..GBA_SCREEN_WIDTH {
        for obj in obj_order.iter() {
            let (x_size, y_size) = obj.size();

            let tile_array_width = if gba.ppu_obj_mapping_1d() {
                x_size as usize >> 3
            } else {
                if obj.full_color() {
                    16
                } else {
                    32
                }
            };
            let tile_num = obj.character_name();
            let palette_num = obj.palette_number();

            // TODO: review this
            if 0x10000 + (tile_num as usize * 32) < obj_base_ptr {
                continue;
            }

            if obj.rotation_enabled() {
                let (pa, pb, pc, pd) =
                    get_object_rotation_data(gba, obj.rotation_scaling_param_selection());
                let obj_x = obj.x();
                let obj_y = obj.y();

                let (bbox_x, bbox_y) = if obj.double_size() {
                    (x_size as i32 * 2, y_size as i32 * 2)
                } else {
                    (x_size as i32, y_size as i32)
                };

                if !((x as i32) >= obj_x && (x as i32) < obj_x + bbox_x) {
                    continue;
                }

                let ix = x as i32 - (obj_x + bbox_x / 2);
                let iy = y as i32 - (obj_y + bbox_y / 2);

                let transformed_x = (pa * ix + pb * iy) >> 8;
                let transformed_y = (pc * ix + pd * iy) >> 8;
                let texture_x = transformed_x + x_size as i32 / 2;
                let texture_y = transformed_y + y_size as i32 / 2;

                if !(texture_x >= 0
                    && texture_x < x_size as i32
                    && texture_y >= 0
                    && texture_y < y_size as i32)
                {
                    continue;
                }

                // TODO: review this
                let (texture_x, texture_y) = if obj.mosaic() {
                    let mosaic_x = texture_x - (texture_x % gba.ppu_obj_mosaic_h() as i32);
                    let mosaic_y = texture_y - (texture_y % gba.ppu_obj_mosaic_v() as i32);
                    (mosaic_x, mosaic_y)
                } else {
                    (texture_x, texture_y)
                };

                let tile_col = (texture_x >> 3) as u32;
                let tile_row = (texture_y >> 3) as u32;
                let nth_line = (texture_y & 0x7) as u16;
                let tile_pixel = (texture_x & 0x7) as u16;

                if obj.full_color() {
                    let color = get_full_color_pixel(
                        gba,
                        obj_base_ptr,
                        tile_num,
                        (tile_row, tile_col),
                        (nth_line, tile_pixel),
                        tile_array_width,
                        true,
                    );
                    if color.is_transparent() {
                        continue;
                    }

                    pixels[x] = color;
                    break;
                } else {
                    let color = get_4bit_color_pixel(
                        gba,
                        obj_base_ptr,
                        tile_num,
                        (tile_row, tile_col),
                        (nth_line, tile_pixel),
                        tile_array_width,
                        palette_num,
                        true,
                    );
                    if color.is_transparent() {
                        continue;
                    }

                    pixels[x] = color;
                    break;
                }
            } else {
                if !(obj.x() as usize <= x && x < obj.x() as usize + x_size as usize) {
                    continue;
                }
                let adj_x = x as u16 - obj.x() as u16;
                let adj_y = y as u16 - obj.y() as u16;
                let adj_x = if obj.horizontal_flip() {
                    x_size as u16 - adj_x - 1
                } else {
                    adj_x
                };
                let adj_y = if obj.vertical_flip() {
                    y_size as u16 - adj_y - 1
                } else {
                    adj_y
                };
                let (adj_x, adj_y) = if obj.mosaic() {
                    let mosaic_x = adj_x - (adj_x % gba.ppu_obj_mosaic_h() as u16);
                    let mosaic_y = adj_y - (adj_y % gba.ppu_obj_mosaic_v() as u16);
                    (mosaic_x, mosaic_y)
                } else {
                    (adj_x, adj_y)
                };
                let tile_col = (adj_x >> 3) as u32;
                let tile_row = (adj_y >> 3) as u32;

                // TOOD: handle non 8x8 objects
                // Lower 3 bits determine which line of the tile we're on
                let nth_line = adj_y & 0x7;
                // 8 choices for which pixel on the line we're on, so we take 3 bits here
                let tile_pixel = adj_x & 0x7;
                let nth_pixel = tile_pixel;
                if obj.full_color() {
                    let color = get_full_color_pixel(
                        gba,
                        obj_base_ptr,
                        tile_num,
                        (tile_row, tile_col),
                        (nth_line, nth_pixel),
                        tile_array_width,
                        true,
                    );
                    if color.is_transparent() {
                        continue;
                    }

                    pixels[x] = color;
                    break;
                } else {
                    let color = get_4bit_color_pixel(
                        gba,
                        obj_base_ptr,
                        tile_num,
                        (tile_row, tile_col),
                        (nth_line, nth_pixel),
                        tile_array_width,
                        palette_num,
                        true,
                    );
                    if color.is_transparent() {
                        continue;
                    }

                    pixels[x] = color;
                    break;
                }
            }
        }
    }

    pixels
}

fn layer_order(gba: &gba::GameboyAdvance) -> Vec<u8> {
    let mut bg_order = vec![];
    if gba.ppu_bg0_enabled() {
        let bg0_priority = gba.ppu_bg0_control().priority;
        bg_order.push((0, bg0_priority));
    }
    if gba.ppu_bg1_enabled() {
        let bg1_priority = gba.ppu_bg1_control().priority;
        bg_order.push((1, bg1_priority));
    }
    if gba.ppu_bg2_enabled() {
        let bg2_priority = gba.ppu_bg2_control().priority;
        bg_order.push((2, bg2_priority));
    }
    if gba.ppu_bg3_enabled() {
        let bg3_priority = gba.ppu_bg3_control().priority;
        bg_order.push((3, bg3_priority));
    }
    bg_order.sort_by_key(|(_, priority)| *priority);
    // HACK: always put objects on top for now
    std::iter::once(4)
        .chain(bg_order.into_iter().map(|(bg_num, _)| bg_num))
        .collect()
}

// HACK: just do it here...
// later we probably want to do this on the GPU and pass this data back for debug/visualization
fn scanline_blend(
    gba: &gba::GameboyAdvance,
    pixels: [[Rgb15; GBA_SCREEN_WIDTH]; 5],
) -> [(u8, u8, u8); GBA_SCREEN_WIDTH] {
    let backdrop_color = Rgb15::from_lo_hi(gba.obj_palette_ram[0], gba.obj_palette_ram[1]);
    let mut blended_pixels = [backdrop_color.to_rgb(); GBA_SCREEN_WIDTH];
    let mut pixel_written = [false; GBA_SCREEN_WIDTH];
    let bg_order = layer_order(gba);

    for x in 0..GBA_SCREEN_WIDTH {
        for &layer in bg_order.iter() {
            if !pixel_written[x as usize] && !pixels[layer as usize][x].is_transparent() {
                blended_pixels[x as usize] = pixels[layer as usize][x].to_rgb();
                pixel_written[x as usize] = true;
                break;
            }
        }
    }
    blended_pixels
}

/*
 Mode  Rot/Scal Layers Size               Tiles Colors       Features
 0     No       0123   256x256..512x515   1024  16/16..256/1 SFMABP
 1     Mixed    012-   (BG0,BG1 as above Mode 0, BG2 as below Mode 2)
 2     Yes      --23   128x128..1024x1024 256   256/1        S-MABP
 3     Yes      --2-   240x160            1     32768        --MABP
 4     Yes      --2-   240x160            2     256/1        --MABP
 5     Yes      --2-   160x128            2     32768        --MABP
*/

pub fn deferred_renderer_draw_gba_scanline(
    y: u8,
    gba: &mut gba::GameboyAdvance,
) -> [(u8, u8, u8); GBA_SCREEN_WIDTH] {
    let mut out = [[Rgb15::transparent(); GBA_SCREEN_WIDTH]; 5];
    if gba.ppu_force_blank() {
        return [Rgb15::white().to_rgb(); GBA_SCREEN_WIDTH];
    }
    match gba.ppu_bg_mode() {
        0 => {
            if gba.ppu_bg0_enabled() {
                out[0] = deferred_renderer_draw_gba_bg(y, 0, gba.ppu_bg0_control(), gba);
            }
            if gba.ppu_bg1_enabled() {
                out[1] = deferred_renderer_draw_gba_bg(y, 1, gba.ppu_bg1_control(), gba);
            }
            if gba.ppu_bg2_enabled() {
                out[2] = deferred_renderer_draw_gba_bg(y, 2, gba.ppu_bg2_control(), gba);
            }
            if gba.ppu_bg3_enabled() {
                out[3] = deferred_renderer_draw_gba_bg(y, 3, gba.ppu_bg3_control(), gba);
                //out[3] = deferred_renderer_draw_gba_bg3(y, gba);
            }
        }
        1 => {
            if gba.ppu_bg0_enabled() {
                out[0] = deferred_renderer_draw_gba_bg(y, 0, gba.ppu_bg0_control(), gba);
            }
            if gba.ppu_bg1_enabled() {
                out[1] = deferred_renderer_draw_gba_bg(y, 1, gba.ppu_bg1_control(), gba);
            }
            if gba.ppu_bg2_enabled() {
                let pa = gba.io_registers.bg2_rotation.pa;
                let pc = gba.io_registers.bg2_rotation.pc;
                let bg_x = gba.io_registers.bg2_rotation.cached_x;
                let bg_y = gba.io_registers.bg2_rotation.cached_y;
                out[2] = deferred_renderer_draw_rotated_bg(
                    y,
                    gba,
                    gba.ppu_bg2_control(),
                    pa,
                    pc,
                    bg_x,
                    bg_y,
                );
            }
        }
        2 => {
            if gba.ppu_bg2_enabled() {
                let pa = gba.io_registers.bg2_rotation.pa;
                let pc = gba.io_registers.bg2_rotation.pc;
                let bg_x = gba.io_registers.bg2_rotation.cached_x;
                let bg_y = gba.io_registers.bg2_rotation.cached_y;
                out[2] = deferred_renderer_draw_rotated_bg(
                    y,
                    gba,
                    gba.ppu_bg2_control(),
                    pa,
                    pc,
                    bg_x,
                    bg_y,
                );
            }
            if gba.ppu_bg3_enabled() {
                let pa = gba.io_registers.bg3_rotation.pa;
                let pc = gba.io_registers.bg3_rotation.pc;
                let bg_x = gba.io_registers.bg3_rotation.cached_x;
                let bg_y = gba.io_registers.bg3_rotation.cached_y;
                out[3] = deferred_renderer_draw_rotated_bg(
                    y,
                    gba,
                    gba.ppu_bg3_control(),
                    pa,
                    pc,
                    bg_x,
                    bg_y,
                );
            }
        }
        3 => {
            if gba.ppu_bg3_enabled() {
                out[3] = deferred_renderer_draw_gba_mode_3(y, gba);
            }
        }
        4 => {
            if gba.ppu_bg2_enabled() {
                out[2] = deferred_renderer_draw_gba_bg4(y, gba);
            }
        }
        5 => todo!("bg mode 5"),
        _ => panic!("unknown PPU mode! {}", gba.ppu_bg_mode()),
    }

    if gba.ppu_obj_enabled() {
        let obj_base_ptr = match gba.ppu_bg_mode() {
            0 | 1 | 2 => 0x10000,
            // TODO: review this
            //3 | 4 | 5 => 0x14000,
            3 | 4 | 5 => 0x10000,
            _ => panic!("unknown PPU mode! {}", gba.ppu_bg_mode()),
        };
        out[4] = naive_object_layer(y, gba, obj_base_ptr);
    }

    scanline_blend(gba, out)
}
