use sdl2;
use sdl2::pixels::*;
use cpu::constants::*;

pub const RB_SCREEN_WIDTH: u32 = 1400;
pub const RB_SCREEN_HEIGHT: u32 = 900;

pub const SCALE: f32 = 2.0;

// pub const MEM_DISP_WIDTH: i32 = SCREEN_WIDTH as i32 / (X_SCALE as i32);
// Looks nicer when evenly divides mem regions
pub const MEM_DISP_WIDTH: i32 = 0x0100;
pub const MEM_DISP_HEIGHT: i32 = 0xFFFF / MEM_DISP_WIDTH + 1; // TODO check this?

pub const CYCLES_PER_HSYNC: u64 = 114; // FIXME this is probably wrong

pub const CPU_CYCLES_PER_SECOND: u64 = 4194304;
pub const DIV_TIMER_STEPS_PER_SECOND: u64 = 16384;
pub const VERT_SYNC_RATE: f32 = 59.73;
pub const CPU_CYCLES_PER_VBLANK: u64 = ((CPU_CYCLES_PER_SECOND as f32) / VERT_SYNC_RATE) as u64;
pub const CPU_CYCLES_PER_DIVIDER_STEP: u64 =
    ((CPU_CYCLES_PER_SECOND as f32) / (DIV_TIMER_STEPS_PER_SECOND as f32)) as u64;

// How long stuff stays on screen
// TODO: Should depend on num of cpu cycles and frame delay
pub const FADE_DELAY: u64 = CPU_CYCLES_PER_VBLANK * 10;

pub const FRAME_SLEEP: u64 = 1000 / 120;

// These are selected by $FF40 (LCDC) special register
// Pixel data is stored here
pub const TILE_PATTERN_TABLE_1_START: MemAddr = 0x8000;
pub const TILE_PATTERN_TABLE_1_END: MemAddr = 0x8FFF;
pub const TILE_PATTERN_TABLE_2_START: MemAddr = 0x8800;
pub const TILE_PATTERN_TABLE_2_END: MemAddr = 0x97FF;

// Tile pattern tables have two ways of indexing them: with signed or
// unsigned nums.
pub const TILE_PATTERN_TABLE_1_ORIGIN: MemAddr = 0x8000;
pub const TILE_PATTERN_TABLE_2_ORIGIN: MemAddr = 0x9000;

#[derive(Clone,Debug)]
pub enum TileDataSelect {
    Auto,
    Mode1,
    Mode2,
}

// These are selected by $FF40 (LCDC) special register
// and store indexes into TILE_PATTERN_TABLE
pub const TILE_MAP_1_START: MemAddr = 0x9800;
pub const TILE_MAP_1_END: MemAddr = 0x9BFF;
pub const TILE_MAP_2_START: MemAddr = 0x9C00;
pub const TILE_MAP_2_END: MemAddr = 0x9FFF;


// tables are overlapping
pub const TILE_PATTERN_TABLES_SIZE: MemAddr = TILE_PATTERN_TABLE_2_END - TILE_PATTERN_TABLE_1_START;

pub const TILE_SIZE_BYTES: u16 = 16;
pub const TILE_SIZE_PX: u16 = 8;
pub const BORDER_PX: u16 = 1;
pub const TILE_COLUMNS: u16 = 16;

pub const TILE_PALETTE: [Color; 4] = [Color::RGB(4, 5, 7),
                                      Color::RGB(235, 135, 140),
                                      Color::RGB(156, 146, 244),
                                      Color::RGB(252, 250, 175)];
pub const OBJECT_PALETTE: [Color; 4] = [Color::RGB(184, 248, 24),
                                        Color::RGB(174, 124, 9),
                                        Color::RGB(248, 184, 0),
                                        Color::RGB(248, 240, 240)];

pub const SCREEN_BUFFER_SIZE_X: u32 = 256;
pub const SCREEN_BUFFER_SIZE_Y: u32 = 256;

pub const SCREEN_BUFFER_TILES_X: u32 = 32;
pub const SCREEN_BUFFER_TILES_Y: u32 = 32;

pub const GB_SCREEN_WIDTH: u8 = 160;
pub const GB_SCREEN_HEIGHT: u8 = 144;

pub const NICER_COLOR: sdl2::pixels::Color = sdl2::pixels::Color::RGBA(139, 41, 2, 255);

pub const OBJECT_ATTRIBUTE_START: u16 = 0xFE00;
pub const OBJECT_ATTRIBUTE_END: u16 = 0xFE9F;
pub const OBJECT_ATTRIBUTE_BLOCK_SIZE: u16 = 4;
