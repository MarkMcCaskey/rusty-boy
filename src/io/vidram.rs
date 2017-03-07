//! Video RAM display

use sdl2;
use io::constants::*;
use cpu;
use cpu::*;

use sdl2::rect::Point;
use sdl2::rect::Rect;
use sdl2::pixels::*;

use io::graphics::Drawable;


pub struct VidRamBGDisplay {
    pub tile_data_select: TileDataSelect,
}

/// Display for backround screen buffer
impl Drawable for VidRamBGDisplay {
    fn get_initial_size(&self) -> (u32, u32) {
        (SCREEN_BUFFER_SIZE_X, SCREEN_BUFFER_SIZE_Y)
    }
    
    fn draw(&mut self, renderer: &mut sdl2::render::Renderer, cpu: &mut Cpu) {
        // TODO add toggle for this also?
        let tile_map_offset = TILE_MAP_1_START;
        
        let ref bg_select = self.tile_data_select;

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
        
        draw_background_buffer(renderer, cpu,
                               tile_map_offset,
                               tile_patterns_offset,
                               0);
        draw_objects(renderer, cpu, 0, 0);
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
        ((TILE_COLUMNS * cell_size) as u32,
         ((tile_num / TILE_COLUMNS) * cell_size) as u32)
    }
    
    fn draw(&mut self, renderer: &mut sdl2::render::Renderer, cpu: &mut Cpu) {
        draw_tile_patterns(renderer, cpu);
    }
    
    fn click(&mut self, button: sdl2::mouse::MouseButton, position: Point, _: &mut Cpu) {
        debug!("Clicked tile display @ {:?} with {:?}", position, button);
    }
}


/// Draw single tile at given screen position
pub fn draw_tile(renderer: &mut sdl2::render::Renderer,
                 gameboy: &Cpu,
                 mem_offset: u16,
                 tile_idx: u16, // technically when used by GB it's only 8bit
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

// TODO cache tiles into texture and use blending options (?) to make
// desired color "trasnparent".
/// Draw single tile at given screen position without drawing "0" color.
pub fn draw_tile_transparent(renderer: &mut sdl2::render::Renderer,
                             gameboy: &Cpu,
                             mem_offset: u16,
                             tile_idx: u16, // technically when used by GB it's only 8bit
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

            if true || px_color != 0 {
                let px_pal_col = OBJECT_PALETTE[px_color as usize];
                renderer.set_draw_color(px_pal_col);
                
                let point = Point::new((screen_offset_x as u8).wrapping_add(px as u8) as i32,
                                       (screen_offset_y as u8).wrapping_add(py as u8) as i32);
                renderer.draw_point(point).unwrap();
            }
        }
    }
}

/// This is the dumbest and straightforward code for displaying Tile
/// Patterns. It displays both background and sprite "tiles" as they
/// overlap in memory.
pub fn draw_tile_patterns(renderer: &mut sdl2::render::Renderer,
                          gameboy: &Cpu) {

    for tile_idx in 0..(TILE_PATTERN_TABLES_SIZE / TILE_SIZE_BYTES) + 1 {

        let tile_start_x = (tile_idx % TILE_COLUMNS) * (TILE_SIZE_PX + BORDER_PX);
        let y_pos = tile_idx / TILE_COLUMNS;
        let tile_start_y = (TILE_SIZE_PX + BORDER_PX) * y_pos;

        draw_tile(renderer,
                  gameboy,
                  TILE_PATTERN_TABLE_1_START,
                  tile_idx,
                  tile_start_x as i32,
                  tile_start_y as i32);
    }
}


/// Draw whole background buffer (256x256 px)
pub fn draw_background_buffer(renderer: &mut sdl2::render::Renderer,
                              gameboy: &Cpu,
                              tile_map_offset: cpu::constants::MemAddr,
                              tile_patterns_offset: cpu::constants::MemAddr,
                              screen_offset_x: i32) {

    // TODO implement proper windows/widgets
    const TOP_Y: i32 = 1;
    let screen_offset_y = TOP_Y;

    // FIXME rethink how to do this better
    match tile_patterns_offset {
        TILE_PATTERN_TABLE_1_ORIGIN => {
            for tile in 0..(SCREEN_BUFFER_TILES_X * SCREEN_BUFFER_TILES_Y) {
                let tile_index = gameboy.mem[(tile_map_offset + tile as u16) as usize];

                let tile_x = tile % SCREEN_BUFFER_TILES_X;
                let tile_y = tile / SCREEN_BUFFER_TILES_Y;

                draw_tile(renderer,
                          gameboy,
                          TILE_PATTERN_TABLE_1_START,
                          tile_index as u16, // use index as unsigned 8bit
                          screen_offset_x + (tile_x * TILE_SIZE_PX as u32) as i32,
                          screen_offset_y + (tile_y * TILE_SIZE_PX as u32) as i32);
                
                
            }
        },
        TILE_PATTERN_TABLE_2_ORIGIN => {
            for tile in 0..(SCREEN_BUFFER_TILES_X * SCREEN_BUFFER_TILES_Y) {
                let tile_index = gameboy.mem[(tile_map_offset + tile as u16) as usize];
                
                let tile_x = tile % SCREEN_BUFFER_TILES_X;
                let tile_y = tile / SCREEN_BUFFER_TILES_Y;

                draw_tile(renderer,
                          gameboy,
                          TILE_PATTERN_TABLE_2_START,             // reposition origin
                          add_u16_i8(128u16, (tile_index as i8)), // index is signed 8bit
                          screen_offset_x + (tile_x * TILE_SIZE_PX as u32) as i32,
                          screen_offset_y + (tile_y * TILE_SIZE_PX as u32) as i32);
                
                
            }

        },
        _ => panic!("Wrong tile data select"),
    };

    draw_screen_border(renderer, gameboy, screen_offset_x, TOP_Y);
}


/// Draw rectangle showing values of SCX and SCY registers,
/// i.e. visible screen area.
fn draw_screen_border(renderer: &mut sdl2::render::Renderer,
                      gameboy: &Cpu,
                      screen_offset_x: i32,
                      screen_offset_y: i32) {
    renderer.set_draw_color(Color::RGB(255, 255, 255));
    let scx: u8 = gameboy.scx();
    let scy: u8 = gameboy.scy();

    renderer.set_clip_rect(Some(Rect::new(screen_offset_x,
                                          screen_offset_y,
                                          SCREEN_BUFFER_SIZE_X,
                                          SCREEN_BUFFER_SIZE_Y)));
    // Draw 9 versions to do wrap around
    // FIXME is this inefficient/dumb and there is a better way? probably.
    for x in -1..2 {
        for y in -1..2 {
            let offset_x = screen_offset_x.wrapping_add(x*SCREEN_BUFFER_SIZE_X as i32);
            let offset_y = screen_offset_y.wrapping_add(y*SCREEN_BUFFER_SIZE_X as i32);
            renderer.draw_rect(Rect::new(offset_x + scx as i32 - 1,
                                         offset_y + scy as i32 - 1,
                                         GB_SCREEN_WIDTH as u32 + 2,
                                         GB_SCREEN_HEIGHT as u32 + 2)).unwrap();
        }
    }
    renderer.set_clip_rect(None);
}


/// Draw "sprites" (something gameboy calls "Objects").
pub fn draw_objects(renderer: &mut sdl2::render::Renderer,
                    gameboy: &Cpu,
                    screen_offset_x: i32,
                    screen_offset_y: i32) {

    // TODO sprites should use palletes
    // TODO sprites can have transparent color
    // TODO sprites are sorted by x/y/idx
    
    for obj_idx in 0..40+1 {
        let offset = OBJECT_ATTRIBUTE_START + obj_idx * OBJECT_ATTRIBUTE_BLOCK_SIZE;
        let sprite_y: u8 = gameboy.mem[offset as usize];
        let sprite_x: u8 = gameboy.mem[offset as usize + 1];
        let tile_index: i8 = gameboy.mem[offset as usize + 2] as i8;
        // TODO implement flag handling (priority, flips...)
        let flags: u8 = gameboy.mem[offset as usize + 3];
        if sprite_x == 0 && sprite_y == 0 {
            // sprite is "hidden"
            continue
        }
        let screen_x = sprite_x.wrapping_sub(8);
        let screen_y = sprite_y.wrapping_sub(16);
        
        if gameboy.lcdc_sprite_size() {
            // "Tall 8x16 sprites" mode
            // FIXME Not sure this is how 8x16 sprites work
            let tile_16 = tile_index & !1;
            draw_tile_transparent(renderer,
                                  gameboy,
                                  TILE_PATTERN_TABLE_2_START,
                                  add_u16_i8(128u16, tile_16), // index is signed 8bit
                                  screen_offset_x + screen_x as i32,
                                  screen_offset_y + screen_y as i32);
            draw_tile_transparent(renderer,
                                  gameboy,
                                  TILE_PATTERN_TABLE_2_START,
                                  add_u16_i8(128u16, (tile_16 + 1)), // index is signed 8bit
                                  screen_offset_x + screen_x as i32,
                                  screen_offset_y + screen_y as i32);
        } else {
            // 8x8 sprites mode
            draw_tile_transparent(renderer,
                                  gameboy,
                                  TILE_PATTERN_TABLE_2_START,
                                  add_u16_i8(128u16, (tile_index as i8)), // index is signed 8bit
                                  screen_offset_x + screen_x as i32,
                                  screen_offset_y + screen_y as i32);

        }
        
    }
       
}
