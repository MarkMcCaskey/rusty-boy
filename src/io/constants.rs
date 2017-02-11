pub const SCREEN_WIDTH: u32 = 1400;
pub const SCREEN_HEIGHT: u32 = 900;

pub const X_SCALE: f32 = 3.0;
pub const Y_SCALE: f32 = X_SCALE;

// pub const MEM_DISP_WIDTH: i32 = SCREEN_WIDTH as i32 / (X_SCALE as i32);
// Looks nicer when evenly divides mem regions
pub const MEM_DISP_WIDTH: i32 = 0x0100;
pub const MEM_DISP_HEIGHT: i32 = 0xFFFF / MEM_DISP_WIDTH; // TODO check this?

pub const CYCLES_PER_HSYNC: u64 = 114; // FIXME this is probably wrong

pub const CPU_CYCLES_PER_SECOND: u64 = 4194304;
pub const VERT_SYNC_RATE: f32 = 59.73;
pub const CPU_CYCLES_PER_VBLANK: u64 = ((CPU_CYCLES_PER_SECOND as f32) / VERT_SYNC_RATE) as u64;

// How long stuff stays on screen
// TODO: Should depend on num of cpu cycles and frame delay
pub const FADE_DELAY: u64 = CPU_CYCLES_PER_VBLANK * 15;

pub const FRAME_SLEEP: u64 = 1000 / 120;

