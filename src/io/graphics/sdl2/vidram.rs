//! Video RAM display

use crate::cpu;
use crate::cpu::*;
use crate::io::constants::*;
use sdl2;

use sdl2::pixels::*;
use sdl2::rect::Point;
use sdl2::rect::Rect;
use sdl2::render::Texture;
use sdl2::surface::Surface;

use super::utility::Drawable;

pub struct VidRamBGDisplay {
    pub tile_data_select: TileDataSelect,
}

const TOP_Y: i32 = 1;

/// Display for backround screen buffer
impl Drawable for VidRamBGDisplay {
    fn get_initial_size(&self) -> (u32, u32) {
        (SCREEN_BUFFER_SIZE_X, SCREEN_BUFFER_SIZE_Y)
    }

    fn draw(&mut self, renderer: &mut sdl2::render::Canvas<Surface>, cpu: &mut Cpu) {
        // TODO add toggle for this also?
        // FIXME pretty sure this is swapped, but for some reason works better
        let tile_map_offset = if cpu.lcdc_bg_tile_map() {
            TILE_MAP_2_START
        } else {
            TILE_MAP_1_START
        };

        let bg_select = &self.tile_data_select;

        let tile_patterns_offset = match *bg_select {
            TileDataSelect::Auto => {
                if cpu.lcdc_bg_win_tile_data() {
                    TILE_PATTERN_TABLE_1_ORIGIN
                } else {
                    TILE_PATTERN_TABLE_2_ORIGIN
                }
            }
            TileDataSelect::Mode1 => TILE_PATTERN_TABLE_1_ORIGIN,
            TileDataSelect::Mode2 => TILE_PATTERN_TABLE_2_ORIGIN,
        };

        draw_background_buffer(
            renderer,
            cpu,
            tile_map_offset,
            tile_patterns_offset,
            MEM_DISP_WIDTH,
        );
        draw_objects(renderer, cpu, cpu.scx() as i32, cpu.scy() as i32);

        if cpu.lcdc_window_on() {
            draw_window_buffer(renderer, cpu, tile_patterns_offset);
        }

        draw_screen_border(renderer, cpu, MEM_DISP_WIDTH, TOP_Y);
    }

    fn click(&mut self, _: sdl2::mouse::MouseButton, _: Point, _: &mut Cpu) {
        self.tile_data_select = match self.tile_data_select {
            TileDataSelect::Auto => TileDataSelect::Mode1,
            TileDataSelect::Mode1 => TileDataSelect::Mode2,
            TileDataSelect::Mode2 => TileDataSelect::Auto,
        };
        debug!("BG buffer tile data: {:?}", self.tile_data_select);
    }
}

pub struct VidRamTileDisplay {
    pub tile_data_select: TileDataSelect,
}

/// Display for tile data. Display tiles in `TILE_COLUMNS` with
/// `BORDER_PX` spacing.
impl Drawable for VidRamTileDisplay {
    fn get_initial_size(&self) -> (u32, u32) {
        let cell_size = TILE_SIZE_PX + BORDER_PX;
        let tile_num = TILE_PATTERN_TABLES_SIZE / TILE_SIZE_BYTES;
        (
            (TILE_COLUMNS * cell_size) as u32,
            ((tile_num / TILE_COLUMNS) * cell_size) as u32,
        )
    }

    fn draw(&mut self, renderer: &mut sdl2::render::Canvas<Surface>, cpu: &mut Cpu) {
        draw_tile_patterns(renderer, cpu);
    }

    fn click(&mut self, button: sdl2::mouse::MouseButton, position: Point, _: &mut Cpu) {
        debug!("Clicked tile display @ {:?} with {:?}", position, button);
    }
}

/// Draw single tile at given screen position
pub fn draw_tile(
    renderer: &mut sdl2::render::Canvas<Surface>,
    gameboy: &Cpu,
    mem_offset: u16,
    tile_idx: u16, // technically when used by GB it's only 8bit
    texture: &mut Texture,
    pixel_buffer: &mut [u8; (TILE_SIZE_PX * TILE_SIZE_PX * 4) as usize],
    dst_rect: &Rect,
) {
    #[inline]
    fn get_bit(n: u8, offset: u8) -> u8 {
        (n >> (7 - offset)) & 1u8
    }

    let offset = mem_offset + (tile_idx * TILE_SIZE_BYTES);

    for px in 0..TILE_SIZE_PX {
        for py in 0..TILE_SIZE_PX {
            let col_byte_off = py * 2;
            let col_byte1_v = gameboy.mem[(offset + col_byte_off) as usize];
            let col_byte2_v = gameboy.mem[(offset + col_byte_off + 1) as usize];
            let col_bit_1 = get_bit(col_byte1_v, px as u8);
            let col_bit_2 = get_bit(col_byte2_v, px as u8);
            let px_color = (col_bit_2 << 1) | col_bit_1;

            let (rval, gval, bval) = TILE_PALETTE[px_color as usize];

            let tile_index = ((py * TILE_SIZE_PX) + (px)) * 4;
            // TODO: verify this order is correct outside of Linux...
            /*unsafe {
                    *pixel_buffer.get_unchecked_mut((tile_index + 0) as usize) = 255;
                    *pixel_buffer.get_unchecked_mut((tile_index + 1) as usize) = gval;
                    *pixel_buffer.get_unchecked_mut((tile_index + 2) as usize) = bval;
                    *pixel_buffer.get_unchecked_mut((tile_index + 3) as usize) = rval;
            }*/
            pixel_buffer[tile_index as usize] = 255;
            pixel_buffer[(tile_index + 1) as usize] = gval;
            pixel_buffer[(tile_index + 2) as usize] = bval;
            pixel_buffer[(tile_index + 3) as usize] = rval;
        }
    }
    texture
        .update(None, &pixel_buffer[..], (TILE_SIZE_PX * 4) as usize)
        .unwrap();
    renderer.copy(texture, None, Some(*dst_rect)).unwrap();
}

// TODO cache tiles into texture and use blending options (?) to make
// desired color "trasnparent".
/// Draw single tile at given screen position without drawing "0" color.
pub fn draw_tile_transparent<T>(
    renderer: &mut sdl2::render::Canvas<T>,
    gameboy: &Cpu,
    mem_offset: u16,
    tile_idx: u16, // technically when used by GB it's only 8bit
    _screen_offset_x: i32,
    _screen_offset_y: i32,
    flip_x: bool,
    flip_y: bool,
    texture: &mut Texture,
    pixel_buffer: &mut [u8; (TILE_SIZE_PX * TILE_SIZE_PX * 4) as usize],
    dst_rect: &Rect,
) where
    T: sdl2::render::RenderTarget,
{
    #[inline]
    fn get_bit(n: u8, offset: u8) -> u8 {
        (n >> (7 - offset)) & 1u8
    }

    //  let mut points = [Point::new(0, 0); (TILE_SIZE_PX * TILE_SIZE_PX) as usize];

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

            let realpx = if flip_x { TILE_SIZE_PX - px - 1 } else { px } as u8;
            let realpy = if flip_y { TILE_SIZE_PX - py - 1 } else { py } as u8;

            /*if DEBUG_OAM_BG || px_color != 0 {
                let px_pal_col = OBJECT_PALETTE[px_color as usize];
                renderer.set_draw_color(px_pal_col);

                //let point
                /*points[(TILE_SIZE_PX * py + px) as usize]*/ //=
                renderer.draw_point(
                    Point::new((screen_offset_x as u8).wrapping_add(realpx) as i32,
                               (screen_offset_y as u8).wrapping_add(realpy) as i32)).unwrap();
            }*/

            //if DEBUG_OAM_BG || px_color != 0 {
            let (rval, bval, gval) = OBJECT_PALETTE[px_color as usize];
            let tile_index = ((((realpy as u16) * TILE_SIZE_PX) + (realpx as u16)) * 4) as u8;
            pixel_buffer[tile_index.wrapping_add(0) as usize] = if px_color == 0 { 0 } else { 255 };
            pixel_buffer[(tile_index.wrapping_add(1)) as usize] = gval;
            pixel_buffer[(tile_index.wrapping_add(2)) as usize] = bval;
            pixel_buffer[(tile_index.wrapping_add(3)) as usize] = rval;
            //}
        }
    }
    texture
        .update(None, &pixel_buffer[..], (TILE_SIZE_PX * 4) as usize)
        .unwrap();
    renderer.copy(texture, None, Some(*dst_rect)).unwrap();
}

/// This is the dumbest and straightforward code for displaying Tile
/// Patterns. It displays both background and sprite "tiles" as they
/// overlap in memory.
pub fn draw_tile_patterns(renderer: &mut sdl2::render::Canvas<Surface>, gameboy: &Cpu) {
    let txt_format = sdl2::pixels::PixelFormatEnum::RGBA8888;
    let texture_creator = renderer.texture_creator();
    let mut texture = texture_creator
        .create_texture_streaming(txt_format, TILE_SIZE_PX as u32, TILE_SIZE_PX as u32)
        .unwrap();
    texture.set_blend_mode(sdl2::render::BlendMode::None);
    let mut pixel_buffer = [0u8; (TILE_SIZE_PX * TILE_SIZE_PX * 4) as usize];
    let mut dst_rect = Rect::new(0, 0, TILE_SIZE_PX as u32, TILE_SIZE_PX as u32);

    for tile_idx in 0..(TILE_PATTERN_TABLES_SIZE / TILE_SIZE_BYTES) + 1 {
        let tile_start_x = (tile_idx % TILE_COLUMNS) * (TILE_SIZE_PX + BORDER_PX);
        let y_pos = tile_idx / TILE_COLUMNS;
        let tile_start_y = (TILE_SIZE_PX + BORDER_PX) * y_pos;

        dst_rect.set_x(tile_start_x as i32);
        dst_rect.set_y(tile_start_y as i32);

        draw_tile(
            renderer,
            gameboy,
            TILE_PATTERN_TABLE_1_START,
            tile_idx,
            &mut texture,
            &mut pixel_buffer,
            &dst_rect,
        );
    }
}

/// Draw whole background buffer (256x256 px)
pub fn draw_background_buffer(
    renderer: &mut sdl2::render::Canvas<Surface>,
    gameboy: &Cpu,
    tile_map_offset: cpu::constants::MemAddr,
    tile_patterns_offset: cpu::constants::MemAddr,
    screen_offset_x: i32,
) {
    // TODO implement proper windows/widgets
    let screen_offset_y = TOP_Y;

    let txt_format = sdl2::pixels::PixelFormatEnum::RGBA8888;
    let texture_creator = renderer.texture_creator();
    let mut texture = texture_creator
        .create_texture_streaming(txt_format, TILE_SIZE_PX as u32, TILE_SIZE_PX as u32)
        .unwrap();
    texture.set_blend_mode(sdl2::render::BlendMode::None);
    let mut pixel_buffer = [0u8; (TILE_SIZE_PX * TILE_SIZE_PX * 4) as usize];
    let mut dst_rect = Rect::new(
        screen_offset_x,
        screen_offset_y,
        TILE_SIZE_PX as u32,
        TILE_SIZE_PX as u32,
    );

    // FIXME rethink how to do this better
    match tile_patterns_offset {
        TILE_PATTERN_TABLE_1_ORIGIN => {
            for tile in 0..(SCREEN_BUFFER_TILES_X * SCREEN_BUFFER_TILES_Y) {
                let tile_index = gameboy.mem[(tile_map_offset + tile as u16) as usize];

                let tile_x = tile % SCREEN_BUFFER_TILES_X;
                let tile_y = tile / SCREEN_BUFFER_TILES_Y;

                dst_rect.set_x((tile_x * TILE_SIZE_PX as u32) as i32);
                dst_rect.set_y(screen_offset_y + (tile_y * TILE_SIZE_PX as u32) as i32);

                draw_tile(
                    renderer,
                    gameboy,
                    TILE_PATTERN_TABLE_1_START,
                    tile_index as u16, // use index as unsigned 8bit
                    //because rendering to its own surface, use 0
                    // value is still needed for background, however
                    &mut texture,
                    &mut pixel_buffer,
                    &dst_rect,
                );
            }
        }
        TILE_PATTERN_TABLE_2_ORIGIN => {
            for tile in 0..(SCREEN_BUFFER_TILES_X * SCREEN_BUFFER_TILES_Y) {
                let tile_index = gameboy.mem[(tile_map_offset + tile as u16) as usize];

                let tile_x = tile % SCREEN_BUFFER_TILES_X;
                let tile_y = tile / SCREEN_BUFFER_TILES_Y;

                dst_rect.set_x((tile_x * TILE_SIZE_PX as u32) as i32);
                dst_rect.set_y(screen_offset_y + (tile_y * TILE_SIZE_PX as u32) as i32);

                draw_tile(
                    renderer,
                    gameboy,
                    TILE_PATTERN_TABLE_2_START, // reposition origin
                    add_u16_i8(128u16, tile_index as i8), // index is signed 8bit
                    //use 0 instead because it's rendering to its
                    // own surface
                    &mut texture,
                    &mut pixel_buffer,
                    &dst_rect,
                );
            }
        }
        _ => panic!("Wrong tile data select"),
    };
}

/// Draw rectangle showing values of SCX and SCY registers,
/// i.e. visible screen area.
fn draw_screen_border<T>(
    renderer: &mut sdl2::render::Canvas<T>,
    gameboy: &Cpu,
    screen_offset_x: i32,
    screen_offset_y: i32,
) where
    T: sdl2::render::RenderTarget,
{
    renderer.set_draw_color(Color::RGB(0xFF, 0xFF, 0x00));
    let scx: u8 = gameboy.scx();
    let scy: u8 = gameboy.scy();

    // Draw 9 versions to do wrap around
    // FIXME is this inefficient/dumb and there is a better way? probably.
    for x in -1..2 {
        for y in -1..2 {
            let offset_x = screen_offset_x.wrapping_add(x * SCREEN_BUFFER_SIZE_X as i32);
            let offset_y = screen_offset_y.wrapping_add(y * SCREEN_BUFFER_SIZE_X as i32);
            renderer
                .draw_rect(Rect::new(
                    offset_x + scx as i32 - 1,
                    offset_y + scy as i32 - 1,
                    GB_SCREEN_WIDTH as u32 + 2,
                    GB_SCREEN_HEIGHT as u32 + 2,
                ))
                .unwrap();
        }
    }
    renderer.set_clip_rect(None);
}

/*fn draw_screen_wrapped<T>(
    renderer: &mut sdl2::render::Canvas<T>,
    gameboy: &Cpu,
    screen_offset_x: i32,
    screen_offset_y: i32,
) where
    T: sdl2::render::RenderTarget,
{
    renderer.surface().blit
}*/

/// Draw "sprites" (something gameboy calls "Objects").
pub fn draw_objects(
    renderer: &mut sdl2::render::Canvas<Surface>,
    gameboy: &Cpu,
    screen_offset_x: i32,
    screen_offset_y: i32,
) {
    let txt_format = sdl2::pixels::PixelFormatEnum::RGBA8888;
    let texture_creator = renderer.texture_creator();
    let mut texture = texture_creator
        .create_texture_streaming(txt_format, TILE_SIZE_PX as u32, TILE_SIZE_PX as u32)
        .unwrap();
    texture.set_blend_mode(sdl2::render::BlendMode::Blend);
    let mut pixel_buffer = [0u8; (TILE_SIZE_PX * TILE_SIZE_PX * 4) as usize];
    let mut dst_rect = Rect::new(0, 0, TILE_SIZE_PX as u32, TILE_SIZE_PX as u32);

    // TODO sprites should use palletes
    // TODO sprites can have transparent color
    // TODO sprites are sorted by x/y/idx

    for obj_idx in 0..40 + 1 {
        let offset = OBJECT_ATTRIBUTE_START + obj_idx * OBJECT_ATTRIBUTE_BLOCK_SIZE;
        let sprite_y: u8 = gameboy.mem[offset as usize];
        let sprite_x: u8 = gameboy.mem[offset as usize + 1];
        let tile_index: u8 = gameboy.mem[offset as usize + 2];
        // TODO implement flag handling (priority, flips...)
        let flags: u8 = gameboy.mem[offset as usize + 3];
        let x_flip = ((flags >> 5) & 1) == 1;
        let y_flip = ((flags >> 6) & 1) == 1;

        if sprite_x == 0 && sprite_y == 0 {
            // sprite is "hidden"
            continue;
        }
        let screen_x = sprite_x.wrapping_sub(8);
        let screen_y = sprite_y.wrapping_sub(16);

        // This table is fixed for OAM
        let pattern_table = TILE_PATTERN_TABLE_1_START;

        if gameboy.lcdc_sprite_size() {
            // "Tall 8x16 sprites" mode
            // FIXME Not sure this is how 8x16 sprites work
            let tile_16 = tile_index & !1;
            dst_rect.set_x(screen_offset_x + screen_x as i32);
            dst_rect.set_y(screen_offset_y + screen_y as i32);

            draw_tile_transparent(
                renderer,
                gameboy,
                pattern_table,
                tile_16 as u16,
                screen_offset_x + screen_x as i32,
                screen_offset_y + screen_y as i32,
                x_flip,
                y_flip,
                &mut texture,
                &mut pixel_buffer,
                &dst_rect,
            );
            // Draw second sprite below the first one
            dst_rect.set_x(screen_offset_x + screen_x as i32);
            dst_rect.set_y(screen_offset_y + screen_y.wrapping_add(8) as i32);

            draw_tile_transparent(
                renderer,
                gameboy,
                pattern_table,
                tile_16 as u16 + 1,
                screen_offset_x + screen_x as i32,
                screen_offset_y + screen_y.wrapping_add(8) as i32,
                x_flip,
                y_flip,
                &mut texture,
                &mut pixel_buffer,
                &dst_rect,
            );
        } else {
            // 8x8 sprites mode
            dst_rect.set_x(screen_offset_x + screen_x as i32);
            dst_rect.set_y(screen_offset_y + screen_y as i32);
            draw_tile_transparent(
                renderer,
                gameboy,
                pattern_table,
                tile_index as u16,
                screen_offset_x + screen_x as i32,
                screen_offset_y + screen_y as i32,
                x_flip,
                y_flip,
                &mut texture,
                &mut pixel_buffer,
                &dst_rect,
            );
        }
    }
}

/// Draw "sprites" (something gameboy calls "Objects").
pub fn draw_window_buffer(
    renderer: &mut sdl2::render::Canvas<Surface>,
    gameboy: &Cpu,
    tile_patterns_offset: u16,
) {
    let x = gameboy.window_x_pos();
    let y = gameboy.window_y_pos();
    let window_tile_data_start = if gameboy.lcdc_tile_map() {
        0x9C00
    } else {
        0x9800
    };

    let txt_format = sdl2::pixels::PixelFormatEnum::RGBA8888;
    let texture_creator = renderer.texture_creator();
    let mut texture = texture_creator
        .create_texture_streaming(txt_format, TILE_SIZE_PX as u32, TILE_SIZE_PX as u32)
        .unwrap();
    texture.set_blend_mode(sdl2::render::BlendMode::None);
    let mut pixel_buffer = [0u8; (TILE_SIZE_PX * TILE_SIZE_PX * 4) as usize];
    let mut dst_rect = Rect::new(x as i32, y as i32, TILE_SIZE_PX as u32, TILE_SIZE_PX as u32);

    for i in 0..32 {
        for j in 0..32 {
            let screen_x = x as i32 + (i * TILE_SIZE_PX as i32);
            let screen_y = y as i32 + (j * TILE_SIZE_PX as i32);

            dst_rect.set_x(screen_x);
            dst_rect.set_y(screen_y);
            let tile_data =
                gameboy.mem[(window_tile_data_start + (j * TILE_SIZE_PX as i32) + i) as usize];

            draw_tile_transparent(
                renderer,
                gameboy,
                tile_patterns_offset,
                tile_data as u16,
                screen_x,
                screen_y,
                false,
                false,
                &mut texture,
                &mut pixel_buffer,
                &dst_rect,
            );
        }
    }
}
