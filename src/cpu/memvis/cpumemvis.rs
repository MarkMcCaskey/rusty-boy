use std::collections::VecDeque;

use cpu::constants::*;

pub trait CpuEventLogger {
    fn new(mem: Option<&[u8]>) -> Self;
    fn log_read(&mut self, timestamp: CycleCount, addr: MemAddr);
    fn log_write(&mut self, timestamp: CycleCount, addr: MemAddr, value: byte);
    fn log_exec(&mut self, timestamp: CycleCount, addr: MemAddr);
    fn log_jump(&mut self, timestamp: CycleCount, src: MemAddr, dst: MemAddr);
}

type AccessFlag = u8;
pub const FLAG_R: u8 = 0x1;
pub const FLAG_W: u8 = 0x2;
pub const FLAG_X: u8 = 0x4;

/// Use RGB + Alpha for cpu event visualization
pub const COLOR_DEPTH: usize = 4;

/// Texture size for storing RGBA pixels for every memory address.
/// (+0x100 for some reason).
pub const EVENT_LOGGER_TEXTURE_SIZE: usize = (0xFFFF + 0x100) * COLOR_DEPTH;

/// Structure for storing info about things happening in memory/cpu.
pub struct DeqCpuEventLogger {
    /// Deque for storing events. (currently used only for jump events)
    pub events_deq: VecDeque<EventLogEntry>,
    /// Mirror of addressable memory values (stored as 4 channel texture)
    pub values: Box<[AccessFlag; EVENT_LOGGER_TEXTURE_SIZE]>,
    /// Color coding for address access types (r/w/x)
    pub access_flags: Box<[AccessFlag; EVENT_LOGGER_TEXTURE_SIZE]>,
    /// Store of recent accesses. Used for fading effect.
    pub access_times: Box<[AccessFlag; EVENT_LOGGER_TEXTURE_SIZE]>,
}

/// WARNING: NOT A REAL CLONE, deletes event information
impl Clone for DeqCpuEventLogger {
    fn clone(&self) -> DeqCpuEventLogger {
        DeqCpuEventLogger::new(None)
    }
}

impl Default for DeqCpuEventLogger {
    fn default() -> Self {
        Self::new(None)
    }
}

const EVENT_LOGGER_ACCESS_TYPE_ALPHA: u8 = 76;

impl CpuEventLogger for DeqCpuEventLogger {
    fn new(mem: Option<&[u8]>) -> DeqCpuEventLogger {
        let mut logger = DeqCpuEventLogger {
            events_deq: VecDeque::new(),
            values: Box::new([0; EVENT_LOGGER_TEXTURE_SIZE]),
            access_flags: Box::new([0; EVENT_LOGGER_TEXTURE_SIZE]),
            access_times: Box::new([0; EVENT_LOGGER_TEXTURE_SIZE]),
        };
        // Mirror current memory values and init other textures.
        if let Some(mem) = mem {
            for (i, &v) in mem.iter().enumerate() {
                let p = v;
                let pi = i * COLOR_DEPTH;
                // Base alpha value
                logger.values[pi] = 255;
                // Fade values a little bit to see access better.
                logger.values[pi + 1] = p / 4;
                logger.values[pi + 2] = p / 4;
                logger.values[pi + 3] = p / 4;

                // Initial alpha for access types (should be lower
                // than value used for displaying current access).
                logger.access_flags[pi] = EVENT_LOGGER_ACCESS_TYPE_ALPHA;
                // initial alpha for current access
                logger.access_times[pi] = 255;

            }
        }
        logger
    }

    fn log_read(&mut self, _: CycleCount, addr: MemAddr) {
        let pi = addr as usize * COLOR_DEPTH;
        self.access_flags[pi + 1] = 255;
        self.access_times[pi + 1] = 255;
    }

    fn log_write(&mut self, _: CycleCount, addr: MemAddr, value: byte) {
        let pi = addr as usize * COLOR_DEPTH;
        // Mirror written value into texture
        self.values[pi] = 255; // Alpha
        // Fade values a little bit to see access better.
        self.values[pi + 1] = value / 4; // Blue
        self.values[pi + 2] = value / 4; // Green
        self.values[pi + 3] = value / 4; // Red

        self.access_flags[pi + 3] = 255;
        self.access_times[pi + 3] = 255;
    }

    fn log_exec(&mut self, _: CycleCount, addr: MemAddr) {
        let pi = addr as usize * COLOR_DEPTH;
        self.access_flags[pi + 2] = 255;
        self.access_times[pi + 2] = 255;
    }

    fn log_jump(&mut self, timestamp: CycleCount, src: MemAddr, dst: MemAddr) {
        let log_jumps = true;
        if log_jumps {
            self.events_deq.push_back(EventLogEntry {
                                          timestamp: timestamp,
                                          event: CpuEvent::Jump {
                                              from: src,
                                              to: dst,
                                          },
                                      });
        }
    }
}


/// Types for storing and visualizing various things happening
#[derive(Copy, Clone)]
pub enum EventPlace {
    Addr(MemAddr),
    Register(CpuRegister),
    Register16(CpuRegister16),
}

#[derive(Copy, Clone)]
pub enum CpuEvent {
    Read { from: MemAddr },
    Write { to: MemAddr },
    Execute(MemAddr),
    // TODO: add vis for registers on the side and draw lines ld'ing stuff
    Move { from: EventPlace, to: EventPlace },
    // TODO: draw lines for jumps
    Jump { from: MemAddr, to: MemAddr },
}

pub type CycleCount = u64;

#[derive(Copy, Clone)]
pub struct EventLogEntry {
    pub timestamp: CycleCount,
    pub event: CpuEvent,
}
