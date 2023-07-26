//! Experimental GBA support

pub struct GameboyAdvance {
    r: Registers,
    // TODO: move this later
    entire_rom: Vec<u8>,
    bios: [u8; 0x4000],
    iw_ram: [u8; 0x8000],
    wram: [u8; 0x40000],
    pub io_registers: IoRegisters,
    pub obj_palette_ram: [u8; 0x400],
    pub vram: [u8; 0x18000],
    pub oam: [u8; 0x400],
    sram: [u8; 0x10000],
    // used for "break points" for counting loops, etc while I debug the basics
    debug_counter: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct PpuBgControl {
    /// 2 bit value
    pub priority: u8,
    /// 2 bit value, in units of 16kb
    pub character_base_block: u8,
    pub mosaic: bool,
    /// false = 16/16, true = 256/1
    pub color_mode: bool,
    /// 5 bit value in units of 2kb
    pub screen_base_block: u8,
    /// only used in BG2 and Bg3
    pub display_area_overflow: bool,
    /*
      Internal Screen Size (dots) and size of BG Map (bytes):

    Value  Text Mode      Rotation/Scaling Mode
    0      256x256 (2K)   128x128   (256 bytes)
    1      512x256 (4K)   256x256   (1K)
    2      256x512 (4K)   512x512   (4K)
    3      512x512 (8K)   1024x1024 (16K)

       */
    /// 2 bit value
    pub screen_size: u8,
}

impl PpuBgControl {
    pub fn from_bits(bits: u16) -> Self {
        Self {
            priority: (bits & 0b11) as u8,
            character_base_block: ((bits >> 2) & 0b11) as u8,
            mosaic: (bits & 0x40) != 0,
            color_mode: (bits & 0x80) != 0,
            screen_base_block: ((bits >> 8) & 0b11111) as u8,
            display_area_overflow: (bits & 0x2000) != 0,
            screen_size: ((bits >> 14) & 0b11) as u8,
        }
    }
}

/// Bit shifting rotation mechanism in the ARM7 CPU.
/// In ARM mode these are bundled into some instructions.
/// In Thumb mode this can be accessed with separate instructions.
pub struct BarrelShifter;

impl BarrelShifter {
    pub fn lsl(val: u32, shift_amt: u32, registers: Option<&mut Registers>) -> u32 {
        if shift_amt == 0 {
            return val;
        }
        if shift_amt > 32 {
            registers.map(|r| r.cpsr_set_carry_flag(false));
            return 0;
        }
        if shift_amt == 32 {
            registers.map(|r| r.cpsr_set_carry_flag(val & 1 == 1));
            return 0;
        }
        registers.map(|r| r.cpsr_set_carry_flag((val >> (32 - shift_amt)) & 1 == 1));
        val << shift_amt
    }
    pub fn lsr(val: u32, shift_amt: u32, registers: Option<&mut Registers>) -> u32 {
        if shift_amt == 0 {
            registers.map(|r| r.cpsr_set_carry_flag((val >> 31) & 1 == 1));
            return 0;
        }
        if shift_amt > 32 {
            registers.map(|r| r.cpsr_set_carry_flag(false));
            return 0;
        }
        if shift_amt == 32 {
            registers.map(|r| r.cpsr_set_carry_flag((val >> 31) & 1 == 1));
            return 0;
        }
        registers.map(|r| r.cpsr_set_carry_flag((val >> (shift_amt - 1)) & 1 == 1));
        val >> shift_amt
    }
    pub fn asr(val: u32, shift_amt: u32, registers: Option<&mut Registers>) -> u32 {
        if shift_amt == 0 {
            registers.map(|r| r.cpsr_set_carry_flag((val >> 31) & 1 == 1));
            return ((val as i32) >> 31) as u32;
        }
        if shift_amt >= 32 {
            registers.map(|r| r.cpsr_set_carry_flag((val >> 31) & 1 == 1));
            return ((val as i32) >> 31) as u32;
        }
        registers.map(|r| r.cpsr_set_carry_flag((val >> (shift_amt - 1)) & 1 == 1));
        ((val as i32) >> shift_amt) as u32
    }
    pub fn ror(val: u32, shift_amt: u32, registers: Option<&mut Registers>) -> u32 {
        if shift_amt == 0 {
            return val;
        }
        let out = val.rotate_right(shift_amt);
        registers.map(|r| r.cpsr_set_carry_flag((out >> 31) & 1 == 1));
        out
    }
    // acts like rrx1, do we need a general RRX?
    pub fn rrx(val: u32, registers: Option<&mut Registers>, carry: bool) -> u32 {
        let out = (val >> 1) | ((carry as u32) << 31);
        registers.map(|r| r.cpsr_set_carry_flag(val & 1 == 1));
        out
    }
}

#[derive(Debug, Clone, Copy)]
pub enum DmaAddrControl {
    Increment,
    Decrement,
    Fixed,
    IncrementReload,
}

impl DmaAddrControl {
    pub fn from_bits(v: u8) -> Self {
        match v {
            0 => Self::Increment,
            1 => Self::Decrement,
            2 => Self::Fixed,
            3 => Self::IncrementReload,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum DmaStartTiming {
    Immediately,
    VBlank,
    HBlank,
    Special,
}

impl DmaStartTiming {
    pub fn from_bits(v: u8) -> Self {
        match v {
            0 => Self::Immediately,
            1 => Self::VBlank,
            2 => Self::HBlank,
            3 => Self::Special,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DmaControl {
    pub dest_addr_control: DmaAddrControl,
    pub source_addr_control: DmaAddrControl,
    pub repeat: bool,
    /// false = 16bit, true = 32bit
    pub transfer_type: bool,
    /// DMA 3 only
    pub game_pak_drq: bool,
    pub start_timing: DmaStartTiming,
    pub irq_at_end: bool,
    pub enabled: bool,
}

impl DmaControl {
    pub fn from_bits(bits: u16) -> Self {
        Self {
            dest_addr_control: DmaAddrControl::from_bits(((bits >> 5) & 0b11) as u8),
            source_addr_control: DmaAddrControl::from_bits(((bits >> 7) & 0b11) as u8),
            repeat: (bits >> 9) & 1 == 1,
            transfer_type: (bits >> 10) & 1 == 1,
            game_pak_drq: (bits >> 11) & 1 == 1,
            start_timing: DmaStartTiming::from_bits(((bits >> 12) & 0b11) as u8),
            irq_at_end: (bits >> 14) & 1 == 1,
            enabled: (bits >> 15) & 1 == 1,
        }
    }
}

pub struct IoRegisters {
    io_registers: [u8; 0x400],
    dma0_enabled: bool,
    dma1_enabled: bool,
    dma2_enabled: bool,
    dma3_enabled: bool,
    timer0: u16,
    timer1: u16,
    timer2: u16,
    timer3: u16,
}

impl IoRegisters {
    pub fn new() -> Self {
        let mut io_registers = [0; 0x400];
        io_registers[0x130] = 0xFF;
        io_registers[0x131] = 0x3;
        IoRegisters {
            io_registers,
            dma0_enabled: false,
            dma1_enabled: false,
            dma2_enabled: false,
            dma3_enabled: false,
            timer0: 0,
            timer1: 0,
            timer2: 0,
            timer3: 0,
        }
    }

    pub fn dma_waiting(&self) -> bool {
        self.dma0_enabled || self.dma1_enabled || self.dma2_enabled || self.dma3_enabled
    }

    pub fn dma0(&self) -> DmaControl {
        DmaControl::from_bits(
            self.io_registers[0xBA] as u16 | ((self.io_registers[0xBB] as u16) << 8),
        )
    }
    pub fn dma1(&self) -> DmaControl {
        DmaControl::from_bits(
            self.io_registers[0xC6] as u16 | ((self.io_registers[0xC7] as u16) << 8),
        )
    }
    pub fn dma2(&self) -> DmaControl {
        DmaControl::from_bits(
            self.io_registers[0xD2] as u16 | ((self.io_registers[0xD3] as u16) << 8),
        )
    }
    pub fn dma3(&self) -> DmaControl {
        DmaControl::from_bits(
            self.io_registers[0xDE] as u16 | ((self.io_registers[0xDF] as u16) << 8),
        )
    }
    pub fn dma0_source_addr(&self) -> u32 {
        self.io_registers[0xB0] as u32
            | ((self.io_registers[0xB1] as u32) << 8)
            | ((self.io_registers[0xB2] as u32) << 16)
            | ((self.io_registers[0xB3] as u32) << 24)
    }
    pub fn dma0_dest_addr(&self) -> u32 {
        self.io_registers[0xB4] as u32
            | ((self.io_registers[0xB5] as u32) << 8)
            | ((self.io_registers[0xB6] as u32) << 16)
            | ((self.io_registers[0xB7] as u32) << 24)
    }
    pub fn dma0_word_count(&self) -> u16 {
        self.io_registers[0xB8] as u16 | ((self.io_registers[0xB9] as u16) << 8)
    }
    pub fn dma1_source_addr(&self) -> u32 {
        self.io_registers[0xBC] as u32
            | ((self.io_registers[0xBD] as u32) << 8)
            | ((self.io_registers[0xBE] as u32) << 16)
            | ((self.io_registers[0xBF] as u32) << 24)
    }
    pub fn dma1_dest_addr(&self) -> u32 {
        self.io_registers[0xC2] as u32
            | ((self.io_registers[0xC3] as u32) << 8)
            | ((self.io_registers[0xC4] as u32) << 16)
            | ((self.io_registers[0xC5] as u32) << 24)
    }
    pub fn dma1_word_count(&self) -> u16 {
        self.io_registers[0xC4] as u16 | ((self.io_registers[0xC5] as u16) << 8)
    }
    pub fn dma2_source_addr(&self) -> u32 {
        self.io_registers[0xC8] as u32
            | ((self.io_registers[0xC9] as u32) << 8)
            | ((self.io_registers[0xCA] as u32) << 16)
            | ((self.io_registers[0xCB] as u32) << 24)
    }
    pub fn dma2_dest_addr(&self) -> u32 {
        self.io_registers[0xCC] as u32
            | ((self.io_registers[0xCD] as u32) << 8)
            | ((self.io_registers[0xCE] as u32) << 16)
            | ((self.io_registers[0xCF] as u32) << 24)
    }
    pub fn dma2_word_count(&self) -> u16 {
        self.io_registers[0xD0] as u16 | ((self.io_registers[0xD1] as u16) << 8)
    }
    pub fn dma3_source_addr(&self) -> u32 {
        self.io_registers[0xD4] as u32
            | ((self.io_registers[0xD5] as u32) << 8)
            | ((self.io_registers[0xD6] as u32) << 16)
            | ((self.io_registers[0xD7] as u32) << 24)
    }
    pub fn dma3_dest_addr(&self) -> u32 {
        self.io_registers[0xD8] as u32
            | ((self.io_registers[0xD9] as u32) << 8)
            | ((self.io_registers[0xDA] as u32) << 16)
            | ((self.io_registers[0xDB] as u32) << 24)
    }
    pub fn dma3_word_count(&self) -> u16 {
        self.io_registers[0xDC] as u16 | ((self.io_registers[0xDD] as u16) << 8)
    }
    pub fn disable_dma0(&mut self) {
        self.io_registers[0xBB] &= !0x80;
    }
    pub fn disable_dma1(&mut self) {
        self.io_registers[0xC7] &= !0x80;
    }
    pub fn disable_dma2(&mut self) {
        self.io_registers[0xD3] &= !0x80;
    }
    pub fn disable_dma3(&mut self) {
        self.io_registers[0xDF] &= !0x80;
    }
    pub fn timer0_running(&self) -> bool {
        (self.io_registers[0x102] >> 7) == 1
    }
    pub fn timer1_running(&self) -> bool {
        (self.io_registers[0x106] >> 7) == 1
    }
    pub fn timer2_running(&self) -> bool {
        (self.io_registers[0x10A] >> 7) == 1
    }
    pub fn timer3_running(&self) -> bool {
        (self.io_registers[0x10E] >> 7) == 1
    }
    pub fn timer0_irq_enabled(&self) -> bool {
        (self.io_registers[0x102] >> 6) & 1 == 1
    }
    pub fn timer1_irq_enabled(&self) -> bool {
        (self.io_registers[0x106] >> 6) & 1 == 1
    }
    pub fn timer2_irq_enabled(&self) -> bool {
        (self.io_registers[0x10A] >> 6) & 1 == 1
    }
    pub fn timer3_irq_enabled(&self) -> bool {
        (self.io_registers[0x10E] >> 6) & 1 == 1
    }
    pub fn timer_irq_enabled(&self, timer: u8) -> bool {
        match timer {
            0 => self.timer0_irq_enabled(),
            1 => self.timer1_irq_enabled(),
            2 => self.timer2_irq_enabled(),
            3 => self.timer3_irq_enabled(),
            _ => unreachable!(),
        }
    }
    pub fn timer_enabled(&self, timer: u8) -> bool {
        match timer {
            0 => self.timer0_running(),
            1 => self.timer1_running(),
            2 => self.timer2_running(),
            3 => self.timer3_running(),
            _ => unreachable!(),
        }
    }
    pub fn increment_timer(&mut self, timer: u8) -> bool {
        match timer {
            0 => {
                if let Some(v) = self.timer0.checked_add(1) {
                    self.timer0 = v;
                } else {
                    self.timer0 = self.get_mem16(0x4000100);
                    return true;
                }
            }
            1 => {
                if let Some(v) = self.timer1.checked_add(1) {
                    self.timer1 = v;
                } else {
                    self.timer1 = self.get_mem16(0x4000104);
                    return true;
                }
            }
            2 => {
                if let Some(v) = self.timer2.checked_add(1) {
                    self.timer2 = v;
                } else {
                    self.timer2 = self.get_mem16(0x4000108);
                    return true;
                }
            }
            3 => {
                if let Some(v) = self.timer3.checked_add(1) {
                    self.timer3 = v;
                } else {
                    self.timer3 = self.get_mem16(0x400010C);
                    return true;
                }
            }
            _ => unreachable!(),
        }
        false
    }
    // TODO: bit 2 count-up mode
    pub fn timer0_prescaler(&self) -> u16 {
        match self.io_registers[0x102] & 3 {
            0b00 => 1,
            0b01 => 64,
            0b10 => 256,
            0b11 => 1024,
            _ => unreachable!(),
        }
    }
    pub fn timer1_prescaler(&self) -> u16 {
        match self.io_registers[0x106] & 3 {
            0b00 => 1,
            0b01 => 64,
            0b10 => 256,
            0b11 => 1024,
            _ => unreachable!(),
        }
    }
    pub fn timer2_prescaler(&self) -> u16 {
        match self.io_registers[0x10A] & 3 {
            0b00 => 1,
            0b01 => 64,
            0b10 => 256,
            0b11 => 1024,
            _ => unreachable!(),
        }
    }
    pub fn timer3_prescaler(&self) -> u16 {
        match self.io_registers[0x10E] & 3 {
            0b00 => 1,
            0b01 => 64,
            0b10 => 256,
            0b11 => 1024,
            _ => unreachable!(),
        }
    }

    pub fn set_mem8(&mut self, addr: u32, val: u8) {
        debug_assert!((0x4000000..=0x4FFFFFF).contains(&addr));
        let addr = addr & 0x3FF;
        match addr {
            // IF: interrupt request flags
            // writes to this register clear set bits to aknowledge interrupts
            0x200..=0x203 => {
                self.io_registers[addr as usize] &= !val;
            }
            0x0B0..=0x0E0 => {
                match addr {
                    0xBB if val & 0x80 != 0 => {
                        // DMA 0 start
                        self.dma0_enabled = true;
                    }
                    0xC7 if val & 0x80 != 0 => {
                        // DMA 1 start
                        self.dma1_enabled = true;
                    }
                    0xD3 if val & 0x80 != 0 => {
                        // DMA 2 start
                        self.dma2_enabled = true;
                    }
                    0xDF if val & 0x80 != 0 => {
                        // DMA3 start
                        self.dma3_enabled = true;
                    }
                    _ => (),
                }
                //println!("DMA: {:X} = {:X}", addr, val);
                self.io_registers[addr as usize] = val;
            }
            0x102 => {
                if (self.io_registers[0x102] >> 7) == 0 && (val >> 7) == 1 {
                    self.timer0 = self.get_mem16(0x100);
                }
                self.io_registers[addr as usize] = val;
            }
            0x106 => {
                if (self.io_registers[0x106] >> 7) == 0 && (val >> 7) == 1 {
                    self.timer1 = self.get_mem16(0x104);
                }
                self.io_registers[addr as usize] = val;
            }
            0x10A => {
                if (self.io_registers[0x10A] >> 7) == 0 && (val >> 7) == 1 {
                    self.timer2 = self.get_mem16(0x108);
                }
                self.io_registers[addr as usize] = val;
            }
            0x10E => {
                if (self.io_registers[0x10E] >> 7) == 0 && (val >> 7) == 1 {
                    self.timer3 = self.get_mem16(0x10C);
                }
                self.io_registers[addr as usize] = val;
            }
            0x100..=0x110 => {
                self.io_registers[addr as usize] = val;
            }
            // interupt clearing
            0x214..=0x217 => {
                self.io_registers[addr as usize] &= !val;
            }
            // TODO:  4000204h - WAITCNT - Waitstate Control (R/W)
            _ => {
                self.io_registers[addr as usize] = val;
            }
        }
    }
    pub fn set_mem16(&mut self, addr: u32, val: u16) {
        debug_assert!((0x4000000..=0x4FFFFFF).contains(&addr));
        let lo_addr = addr & !1;
        let hi_addr = addr | 1;

        let lo_byte = val as u8;
        let hi_byte = (val >> 8) as u8;
        self.set_mem8(lo_addr, lo_byte);
        self.set_mem8(hi_addr, hi_byte);
    }
    pub fn set_mem32(&mut self, addr: u32, val: u32) {
        debug_assert!((0x4000000..=0x4FFFFFF).contains(&addr));
        // REVIEW: make sure this is correct
        let lo_addr = addr & !3;
        let hi_addr = addr | 2;

        let lo_half_word = val as u16;
        let hi_half_word = (val >> 16) as u16;
        self.set_mem16(lo_addr, lo_half_word);
        self.set_mem16(hi_addr, hi_half_word);
    }
    pub fn get_mem8(&self, addr: u32) -> u8 {
        debug_assert!((0x4000000..=0x4FFFFFF).contains(&addr));
        let addr = addr & 0x3FF;
        match addr {
            0x100 => (self.timer0 & 0xFF) as u8,
            0x101 => (self.timer0 >> 8) as u8,
            0x104 => (self.timer1 & 0xFF) as u8,
            0x105 => (self.timer1 >> 8) as u8,
            0x108 => (self.timer2 & 0xFF) as u8,
            0x109 => (self.timer2 >> 8) as u8,
            0x10C => (self.timer3 & 0xFF) as u8,
            0x10D => (self.timer3 >> 8) as u8,
            _ => self.io_registers[addr as usize],
        }
    }
    pub fn get_mem16(&self, addr: u32) -> u16 {
        debug_assert!((0x4000000..=0x4FFFFFF).contains(&addr));
        let lo_addr = addr & !1;
        let hi_addr = addr | 1;

        (self.get_mem8(lo_addr) as u16) | ((self.get_mem8(hi_addr) as u16) << 8)
    }
    pub fn get_mem32(&self, addr: u32) -> u32 {
        debug_assert!((0x4000000..=0x4FFFFFF).contains(&addr));
        let lo_addr = addr & !3;
        let hi_addr = addr | 2;

        (self.get_mem16(lo_addr) as u32) | ((self.get_mem16(hi_addr) as u32) << 16)
    }
}

impl std::ops::Index<usize> for IoRegisters {
    type Output = u8;

    fn index(&self, index: usize) -> &Self::Output {
        &self.io_registers[index]
    }
}

impl std::ops::IndexMut<usize> for IoRegisters {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.io_registers[index]
    }
}

pub struct Registers {
    pub r0: u32,
    pub r1: u32,
    pub r2: u32,
    pub r3: u32,
    pub r4: u32,
    pub r5: u32,
    pub r6: u32,
    pub r7: u32,
    pub r8: u32,
    pub r9: u32,
    pub r10: u32,
    pub r11: u32,
    pub r12: u32,
    /// r13
    pub sp: u32,
    /// r14
    pub lr: u32,
    /// r15
    pub pc: u32,
    pub r8_fiq: u32,
    pub r9_fiq: u32,
    pub r10_fiq: u32,
    pub r11_fiq: u32,
    pub r12_fiq: u32,
    /// fiq stack pointer
    pub r13_fiq: u32,
    pub r14_fiq: u32,
    /// svc stack pointer
    pub r13_svc: u32,
    pub r14_svc: u32,
    /// abt stack pointer
    pub r13_abt: u32,
    pub r14_abt: u32,
    /// irq stack pointer
    pub r13_irq: u32,
    pub r14_irq: u32,
    /// und stack pointer
    pub r13_und: u32,
    pub r14_und: u32,

    /// Flags, etc
    pub cpsr: u32,

    pub spsr_fiq: u32,
    pub spsr_svc: u32,
    pub spsr_abt: u32,
    pub spsr_irq: u32,
    pub spsr_und: u32,
}

impl std::ops::Index<u8> for Registers {
    type Output = u32;

    fn index(&self, index: u8) -> &Self::Output {
        // TODO: is user or undefined a better default?
        let mode = self.register_mode().unwrap_or(RegisterMode::Undefined);
        match index {
            0 => &self.r0,
            1 => &self.r1,
            2 => &self.r2,
            3 => &self.r3,
            4 => &self.r4,
            5 => &self.r5,
            6 => &self.r6,
            7 => &self.r7,
            8 => {
                if mode == RegisterMode::FIQ {
                    &self.r8_fiq
                } else {
                    &self.r8
                }
            }
            9 => {
                if mode == RegisterMode::FIQ {
                    &self.r9_fiq
                } else {
                    &self.r9
                }
            }
            10 => {
                if mode == RegisterMode::FIQ {
                    &self.r10_fiq
                } else {
                    &self.r10
                }
            }
            11 => {
                if mode == RegisterMode::FIQ {
                    &self.r11_fiq
                } else {
                    &self.r11
                }
            }
            12 => {
                if mode == RegisterMode::FIQ {
                    &self.r12_fiq
                } else {
                    &self.r12
                }
            }
            13 => match mode {
                RegisterMode::FIQ => &self.r13_fiq,
                RegisterMode::Supervisor => &self.r13_svc,
                RegisterMode::Abort => &self.r13_abt,
                RegisterMode::IRQ => &self.r13_irq,
                RegisterMode::Undefined => &self.r13_und,
                RegisterMode::User => &self.sp,
                RegisterMode::System => &self.sp,
            },
            14 => match mode {
                RegisterMode::FIQ => &self.r14_fiq,
                RegisterMode::Supervisor => &self.r14_svc,
                RegisterMode::Abort => &self.r14_abt,
                RegisterMode::IRQ => &self.r14_irq,
                RegisterMode::Undefined => &self.r14_und,
                RegisterMode::User => &self.lr,
                RegisterMode::System => &self.lr,
            },
            15 => &self.pc,
            _ => unimplemented!("invalid register read"),
        }
    }
}

impl std::ops::IndexMut<u8> for Registers {
    fn index_mut(&mut self, index: u8) -> &mut Self::Output {
        // TODO: is user or undefined a better default?
        let mode = self.register_mode().unwrap_or(RegisterMode::Undefined);
        match index {
            0 => &mut self.r0,
            1 => &mut self.r1,
            2 => &mut self.r2,
            3 => &mut self.r3,
            4 => &mut self.r4,
            5 => &mut self.r5,
            6 => &mut self.r6,
            7 => &mut self.r7,
            8 => {
                if mode == RegisterMode::FIQ {
                    &mut self.r8_fiq
                } else {
                    &mut self.r8
                }
            }
            9 => {
                if mode == RegisterMode::FIQ {
                    &mut self.r9_fiq
                } else {
                    &mut self.r9
                }
            }
            10 => {
                if mode == RegisterMode::FIQ {
                    &mut self.r10_fiq
                } else {
                    &mut self.r10
                }
            }
            11 => {
                if mode == RegisterMode::FIQ {
                    &mut self.r11_fiq
                } else {
                    &mut self.r11
                }
            }
            12 => {
                if mode == RegisterMode::FIQ {
                    &mut self.r12_fiq
                } else {
                    &mut self.r12
                }
            }
            13 => match mode {
                RegisterMode::FIQ => &mut self.r13_fiq,
                RegisterMode::Supervisor => &mut self.r13_svc,
                RegisterMode::Abort => &mut self.r13_abt,
                RegisterMode::IRQ => &mut self.r13_irq,
                RegisterMode::Undefined => &mut self.r13_und,
                RegisterMode::User => &mut self.sp,
                RegisterMode::System => &mut self.sp,
            },
            14 => match mode {
                RegisterMode::FIQ => &mut self.r14_fiq,
                RegisterMode::Supervisor => &mut self.r14_svc,
                RegisterMode::Abort => &mut self.r14_abt,
                RegisterMode::IRQ => &mut self.r14_irq,
                RegisterMode::Undefined => &mut self.r14_und,
                RegisterMode::User => &mut self.lr,
                RegisterMode::System => &mut self.lr,
            },
            15 => &mut self.pc,
            _ => unimplemented!("invalid register write"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    User,
    FIQ,
    IRQ,
    Supervisor,
}

impl Mode {
    pub fn from_u8(val: u8) -> Self {
        match val {
            0b00 => Mode::User,
            0b01 => Mode::FIQ,
            0b10 => Mode::IRQ,
            0b11 => Mode::Supervisor,
            _ => unimplemented!(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegisterMode {
    User,
    FIQ,
    IRQ,
    Supervisor,
    Abort,
    Undefined,
    /// Not described in GBA docs, but this is a real ARMv7 mode...
    System,
}

impl RegisterMode {
    pub fn from_u8(val: u8) -> Option<Self> {
        match val {
            // non-privileged user
            0b10000 => Some(Self::User),
            0b10001 => Some(Self::FIQ),
            0b10010 => Some(Self::IRQ),
            0b10011 => Some(Self::Supervisor),
            0b10111 => Some(Self::Abort),
            0b11011 => Some(Self::Undefined),
            // privileged user
            0b11111 => Some(Self::System),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cond {
    Eq,
    Ne,
    CsHs,
    CcLo,
    Mi,
    Pl,
    Vs,
    Vc,
    Hi,
    Ls,
    Ge,
    Lt,
    Gt,
    Le,
    Al,
    Nv,
}

impl Cond {
    fn from_u8(val: u8) -> Self {
        match val {
            0b0000 => Cond::Eq,
            0b0001 => Cond::Ne,
            0b0010 => Cond::CsHs,
            0b0011 => Cond::CcLo,
            0b0100 => Cond::Mi,
            0b0101 => Cond::Pl,
            0b0110 => Cond::Vs,
            0b0111 => Cond::Vc,
            0b1000 => Cond::Hi,
            0b1001 => Cond::Ls,
            0b1010 => Cond::Ge,
            0b1011 => Cond::Lt,
            0b1100 => Cond::Gt,
            0b1101 => Cond::Le,
            0b1110 => Cond::Al,
            0b1111 => Cond::Nv,
            _ => unreachable!(),
        }
    }
}

impl Registers {
    pub fn new() -> Self {
        Self {
            r0: 0,
            r1: 0,
            r2: 0,
            r3: 0,
            r4: 0,
            r5: 0,
            r6: 0,
            r7: 0,
            r8: 0,
            r9: 0,
            r10: 0,
            r11: 0,
            r12: 0,
            sp: 0x03007F00,
            lr: 0,
            pc: 0x08000000,
            r8_fiq: 0,
            r9_fiq: 0,
            r10_fiq: 0,
            r11_fiq: 0,
            r12_fiq: 0,
            r13_fiq: 0,
            r14_fiq: 0,
            r13_svc: 0x03007FE0,
            r14_svc: 0,
            r13_abt: 0,
            r14_abt: 0,
            r13_irq: 0x03007FA0,
            r14_irq: 0,
            r13_und: 0,
            r14_und: 0,
            // Disable IRQ, FIQ, and set supervisor mode
            cpsr: 0b11010011,
            spsr_fiq: 0,
            spsr_svc: 0,
            spsr_abt: 0,
            spsr_irq: 0,
            spsr_und: 0,
        }
    }

    pub fn cpsr_sign_flag(&self) -> bool {
        (self.cpsr >> 31) & 1 != 0
    }
    pub fn cpsr_set_sign_flag(&mut self, v: bool) {
        self.cpsr &= !(1 << 31);
        self.cpsr |= (v as u32) << 31;
    }
    pub fn cpsr_zero_flag(&self) -> bool {
        (self.cpsr >> 30) & 1 != 0
    }
    pub fn cpsr_set_zero_flag(&mut self, v: bool) {
        self.cpsr &= !(1 << 30);
        self.cpsr |= (v as u32) << 30;
    }
    pub fn cpsr_carry_flag(&self) -> bool {
        (self.cpsr >> 29) & 1 != 0
    }
    pub fn cpsr_set_carry_flag(&mut self, v: bool) {
        self.cpsr &= !(1 << 29);
        self.cpsr |= (v as u32) << 29;
    }
    pub fn cpsr_overflow_flag(&self) -> bool {
        (self.cpsr >> 28) & 1 != 0
    }
    pub fn cpsr_set_overflow_flag(&mut self, v: bool) {
        self.cpsr &= !(1 << 28);
        self.cpsr |= (v as u32) << 28;
    }
    pub fn irq_disabled(&self) -> bool {
        (self.cpsr >> 7) & 1 != 0
    }
    pub fn fiq_disabled(&self) -> bool {
        (self.cpsr >> 6) & 1 != 0
    }
    pub fn thumb_enabled(&self) -> bool {
        (self.cpsr >> 5) & 1 != 0
    }
    pub fn set_thumb(&mut self, enabled: bool) {
        self.cpsr &= !(1 << 5);
        self.cpsr |= (enabled as u32) << 5;
    }
    pub fn cpsr_disable_irq(&mut self) {
        self.cpsr |= 1 << 7;
    }
    // bit 27 is sticky overflow, not relevant on GBA CPU I think
    pub fn mode_bits(&self) -> u8 {
        (self.cpsr & 0x1F) as u8
    }
    pub fn register_mode(&self) -> Option<RegisterMode> {
        RegisterMode::from_u8(self.mode_bits())
    }
    // TODO: review this later, all details fuzzy
    pub fn get_spsr(&self) -> u32 {
        let mode = self.register_mode().unwrap();
        match mode {
            RegisterMode::User | RegisterMode::System => unimplemented!("Is this possible?"),
            RegisterMode::FIQ => self.spsr_fiq,
            RegisterMode::Supervisor => self.spsr_svc,
            RegisterMode::Abort => self.spsr_abt,
            RegisterMode::IRQ => self.spsr_irq,
            RegisterMode::Undefined => self.spsr_und,
        }
    }
    pub fn get_spsr_mut(&mut self) -> &mut u32 {
        let mode = self.register_mode().unwrap();
        match mode {
            RegisterMode::User | RegisterMode::System => unimplemented!("Is this possible?"),
            RegisterMode::FIQ => &mut self.spsr_fiq,
            RegisterMode::Supervisor => &mut self.spsr_svc,
            RegisterMode::Abort => &mut self.spsr_abt,
            RegisterMode::IRQ => &mut self.spsr_irq,
            RegisterMode::Undefined => &mut self.spsr_und,
        }
    }
    pub fn set_svc_mode(&mut self) {
        self.cpsr &= !0x1F;
        self.cpsr |= 0b10011;
    }
    pub fn set_irq_mode(&mut self) {
        self.cpsr &= !0x1F;
        self.cpsr |= 0b10010;
    }
    pub fn lr(&self) -> u32 {
        let mode = self.register_mode().unwrap();
        match mode {
            RegisterMode::User => self.lr,
            RegisterMode::FIQ => self.r14_fiq,
            RegisterMode::Supervisor => self.r14_svc,
            RegisterMode::Abort => self.r14_abt,
            RegisterMode::IRQ => self.r14_irq,
            RegisterMode::Undefined => self.r14_und,
            RegisterMode::System => self.lr,
        }
    }
    pub fn lr_mut(&mut self) -> &mut u32 {
        let mode = self.register_mode().unwrap();
        match mode {
            RegisterMode::User => &mut self.lr,
            RegisterMode::FIQ => &mut self.r14_fiq,
            RegisterMode::Supervisor => &mut self.r14_svc,
            RegisterMode::Abort => &mut self.r14_abt,
            RegisterMode::IRQ => &mut self.r14_irq,
            RegisterMode::Undefined => &mut self.r14_und,
            RegisterMode::System => &mut self.lr,
        }
    }
    pub fn sp(&self) -> u32 {
        let mode = self.register_mode().unwrap();
        match mode {
            RegisterMode::User => self.sp,
            RegisterMode::FIQ => self.r13_fiq,
            RegisterMode::Supervisor => self.r13_svc,
            RegisterMode::Abort => self.r13_abt,
            RegisterMode::IRQ => self.r13_irq,
            RegisterMode::Undefined => self.r13_und,
            RegisterMode::System => self.sp,
        }
    }
    pub fn sp_mut(&mut self) -> &mut u32 {
        let mode = self.register_mode().unwrap();
        match mode {
            RegisterMode::User => &mut self.sp,
            RegisterMode::FIQ => &mut self.r13_fiq,
            RegisterMode::Supervisor => &mut self.r13_svc,
            RegisterMode::Abort => &mut self.r13_abt,
            RegisterMode::IRQ => &mut self.r13_irq,
            RegisterMode::Undefined => &mut self.r13_und,
            RegisterMode::System => &mut self.sp,
        }
    }
    pub fn split_pc(&self) -> (u8, (bool, bool), u32, Mode) {
        let mode = Mode::from_u8((self.pc & 0x3) as u8);
        let pc = (self.pc >> 2) & 0xFF_FFFF;
        // TODO: proper type
        let flags = (self.pc >> 28) & 0xF;
        let irq_disabled = (self.pc >> 27) & 1 == 1;
        let fiq_disabled = (self.pc >> 26) & 1 == 1;

        (flags as u8, (irq_disabled, fiq_disabled), pc, mode)
    }
}

impl GameboyAdvance {
    pub fn new() -> GameboyAdvance {
        GameboyAdvance {
            r: Registers::new(),
            entire_rom: vec![],
            bios: [0u8; 0x4000],
            iw_ram: [0u8; 0x8000],
            wram: [0u8; 0x40000],
            io_registers: IoRegisters::new(),
            obj_palette_ram: [0u8; 0x400],
            vram: [0u8; 0x18000],
            oam: [0u8; 0x400],
            sram: [0u8; 0x10000],
            debug_counter: 0,
        }
    }

    /// Very useful link:
    /// https://problemkaputt.de/gbatek.htm#gbacartridgeheader
    pub fn load_rom(&mut self, bytes: Vec<u8>) {
        let mut title = String::new();
        bytes[0xA0..(0xA0 + 12)].iter().for_each(|b| {
            if *b != 0 {
                title.push(*b as char);
            }
        });

        let mut game_code = String::new();
        bytes[0xAC..(0xAC + 4)].iter().for_each(|b| {
            if *b != 0 {
                game_code.push(*b as char);
            }
        });

        info!("Found GBA ROM: `{}` ({})", title, game_code);

        let fixed_value = bytes[0xB2];
        if fixed_value != 0x96 {
            error!("GBA ROM Header 0xB2 must be 0x96, found 0x{:X}. This may not be a GBA ROM or it may be corrupt", fixed_value);
        }

        let main_unit_code = bytes[0xB3];
        if main_unit_code != 0 {
            warn!("Non-zero main unit code: {} found!", main_unit_code);
        }

        // TODO: 0xBD is the complement check, check header for validity
        let first_opcode = bytes[0];
        self.entire_rom = bytes;
    }

    pub fn load_bios(&mut self, data: &[u8]) {
        self.bios.copy_from_slice(data);
    }

    pub fn cond_should_execute(&self, cond: Cond) -> bool {
        match cond {
            Cond::Eq => self.r.cpsr_zero_flag(),
            Cond::Ne => !self.r.cpsr_zero_flag(),
            Cond::CsHs => self.r.cpsr_carry_flag(),
            Cond::CcLo => !self.r.cpsr_carry_flag(),
            Cond::Mi => self.r.cpsr_sign_flag(),
            Cond::Pl => !self.r.cpsr_sign_flag(),
            Cond::Vs => self.r.cpsr_overflow_flag(),
            Cond::Vc => !self.r.cpsr_overflow_flag(),
            Cond::Hi => self.r.cpsr_carry_flag() && !self.r.cpsr_zero_flag(),
            Cond::Ls => !self.r.cpsr_carry_flag() || self.r.cpsr_zero_flag(),
            Cond::Ge => self.r.cpsr_sign_flag() == self.r.cpsr_overflow_flag(),
            Cond::Lt => self.r.cpsr_sign_flag() != self.r.cpsr_overflow_flag(),
            Cond::Gt => {
                !self.r.cpsr_zero_flag() && (self.r.cpsr_sign_flag() == self.r.cpsr_overflow_flag())
            }
            Cond::Le => {
                self.r.cpsr_zero_flag() || (self.r.cpsr_sign_flag() != self.r.cpsr_overflow_flag())
            }
            Cond::Al => true,
            Cond::Nv => false,
        }
    }

    // for modes 4 and 5
    pub fn ppu_frame_select(&self) -> bool {
        (self.io_registers[0] >> 4) & 1 == 1
    }
    pub fn ppu_bg0_x_scroll(&self) -> u16 {
        ((self.io_registers[0x10] as u16) << 8) | (self.io_registers[0x11] as u16) & 0x1F
    }
    pub fn ppu_bg0_y_scroll(&self) -> u16 {
        ((self.io_registers[0x12] as u16) << 8) | (self.io_registers[0x13] as u16) & 0x1F
    }
    pub fn ppu_bg1_x_scroll(&self) -> u16 {
        ((self.io_registers[0x14] as u16) << 8) | (self.io_registers[0x15] as u16) & 0x1F
    }
    pub fn ppu_bg1_y_scroll(&self) -> u16 {
        ((self.io_registers[0x16] as u16) << 8) | (self.io_registers[0x17] as u16) & 0x1F
    }
    pub fn ppu_bg2_x_scroll(&self) -> u16 {
        ((self.io_registers[0x18] as u16) << 8) | (self.io_registers[0x19] as u16) & 0x1F
    }
    pub fn ppu_bg2_y_scroll(&self) -> u16 {
        ((self.io_registers[0x1A] as u16) << 8) | (self.io_registers[0x1B] as u16) & 0x1F
    }
    pub fn ppu_bg3_x_scroll(&self) -> u16 {
        ((self.io_registers[0x1C] as u16) << 8) | (self.io_registers[0x1D] as u16) & 0x1F
    }
    pub fn ppu_bg3_y_scroll(&self) -> u16 {
        ((self.io_registers[0x1E] as u16) << 8) | (self.io_registers[0x1F] as u16) & 0x1F
    }
    pub fn ppu_bg0_control(&self) -> PpuBgControl {
        let bits = ((self.io_registers[0x8] as u16) << 8) | (self.io_registers[0x9] as u16);
        PpuBgControl::from_bits(bits)
    }
    pub fn ppu_bg1_control(&self) -> PpuBgControl {
        let bits = ((self.io_registers[0xA] as u16) << 8) | (self.io_registers[0xB] as u16);
        PpuBgControl::from_bits(bits)
    }
    pub fn ppu_bg2_control(&self) -> PpuBgControl {
        let bits = ((self.io_registers[0xC] as u16) << 8) | (self.io_registers[0xD] as u16);
        PpuBgControl::from_bits(bits)
    }
    pub fn ppu_bg3_control(&self) -> PpuBgControl {
        let bits = ((self.io_registers[0xE] as u16) << 8) | (self.io_registers[0xF] as u16);
        PpuBgControl::from_bits(bits)
    }
    pub fn ppu_bg_mode(&self) -> u8 {
        self.io_registers[0x0] & 0x7
    }
    pub fn ppu_bg0_enabled(&self) -> bool {
        self.io_registers[0x1] & 1 == 1
    }
    pub fn ppu_bg1_enabled(&self) -> bool {
        (self.io_registers[0x1] >> 1) & 1 == 1
    }
    pub fn ppu_bg2_enabled(&self) -> bool {
        (self.io_registers[0x1] >> 2) & 1 == 1
    }
    pub fn ppu_bg3_enabled(&self) -> bool {
        (self.io_registers[0x1] >> 3) & 1 == 1
    }
    pub fn ppu_obj_enabled(&self) -> bool {
        (self.io_registers[0x1] >> 4) & 1 == 1
    }
    pub fn ppu_win0_enabled(&self) -> bool {
        (self.io_registers[0x1] >> 5) & 1 == 1
    }
    pub fn ppu_win1_enabled(&self) -> bool {
        (self.io_registers[0x1] >> 6) & 1 == 1
    }
    pub fn ppu_obj_win_enabled(&self) -> bool {
        (self.io_registers[0x1] >> 7) & 1 == 1
    }

    pub fn ppu_set_vblank(&mut self, v: bool) {
        self.io_registers[0x4] &= !1;
        self.io_registers[0x4] |= v as u8;
    }
    pub fn ppu_set_hblank(&mut self, v: bool) {
        self.io_registers[0x4] &= !0b10;
        self.io_registers[0x4] |= (v as u8) << 1;
    }
    pub fn ppu_set_vcounter(&mut self, v: bool) {
        self.io_registers[0x4] &= !0b100;
        self.io_registers[0x4] |= (v as u8) << 2;
    }
    pub fn ppu_hblank_irq_enabled(&self) -> bool {
        (self.io_registers[0x4] >> 3) & 1 == 1
    }
    pub fn ppu_vblank_irq_enabled(&self) -> bool {
        (self.io_registers[0x4] >> 4) & 1 == 1
    }
    pub fn ppu_vcounter_irq_enabled(&self) -> bool {
        (self.io_registers[0x4] >> 5) & 1 == 1
    }
    pub fn ppu_vcounter_setting(&self) -> u8 {
        self.io_registers[0x5]
    }
    pub fn ppu_set_readonly_vcounter(&mut self, ly: u8) {
        self.io_registers[0x6] = ly as u8;
    }

    pub fn master_interrupts_enabled(&self) -> bool {
        self.io_registers[0x208] & 1 == 1
    }

    pub fn lcdc_vblank_interrupt_enabled(&self) -> bool {
        self.io_registers[0x200] & 1 == 1
    }
    pub fn lcdc_hblank_interrupt_enabled(&self) -> bool {
        (self.io_registers[0x200] >> 1) & 1 == 1
    }
    pub fn lcdc_vcounter_interrupt_enabled(&self) -> bool {
        (self.io_registers[0x200] >> 2) & 1 == 1
    }
    pub fn timer0_interrupt_enabled(&self) -> bool {
        (self.io_registers[0x200] >> 3) & 1 == 1
    }
    pub fn timer1_interrupt_enabled(&self) -> bool {
        (self.io_registers[0x200] >> 4) & 1 == 1
    }
    pub fn timer2_interrupt_enabled(&self) -> bool {
        (self.io_registers[0x200] >> 5) & 1 == 1
    }
    pub fn timer3_interrupt_enabled(&self) -> bool {
        (self.io_registers[0x200] >> 6) & 1 == 1
    }
    pub fn serial_interrupt_enabled(&self) -> bool {
        (self.io_registers[0x200] >> 7) & 1 == 1
    }
    pub fn dma0_interrupt_enabled(&self) -> bool {
        self.io_registers[0x201] & 1 == 1
    }
    pub fn dma1_interrupt_enabled(&self) -> bool {
        (self.io_registers[0x201] >> 1) & 1 == 1
    }
    pub fn dma2_interrupt_enabled(&self) -> bool {
        (self.io_registers[0x201] >> 2) & 1 == 1
    }
    pub fn dma3_interrupt_enabled(&self) -> bool {
        (self.io_registers[0x201] >> 3) & 1 == 1
    }
    pub fn keypad_interrupt_enabled(&self) -> bool {
        (self.io_registers[0x201] >> 4) & 1 == 1
    }
    pub fn game_pak_interrupt_enabled(&self) -> bool {
        (self.io_registers[0x201] >> 5) & 1 == 1
    }

    pub fn lcdc_vblank_interrupt_requested(&self) -> bool {
        self.io_registers[0x202] & 1 == 1
    }
    pub fn lcdc_hblank_interrupt_requested(&self) -> bool {
        (self.io_registers[0x202] >> 1) & 1 == 1
    }
    pub fn lcdc_vcounter_interrupt_requested(&self) -> bool {
        (self.io_registers[0x202] >> 2) & 1 == 1
    }
    pub fn timer0_interrupt_requested(&self) -> bool {
        (self.io_registers[0x202] >> 3) & 1 == 1
    }
    pub fn timer1_interrupt_requested(&self) -> bool {
        (self.io_registers[0x202] >> 4) & 1 == 1
    }
    pub fn timer2_interrupt_requested(&self) -> bool {
        (self.io_registers[0x202] >> 5) & 1 == 1
    }
    pub fn timer3_interrupt_requested(&self) -> bool {
        (self.io_registers[0x202] >> 6) & 1 == 1
    }
    pub fn serial_interrupt_requested(&self) -> bool {
        (self.io_registers[0x202] >> 7) & 1 == 1
    }
    pub fn dma0_interrupt_requested(&self) -> bool {
        self.io_registers[0x203] & 1 == 1
    }
    pub fn dma1_interrupt_requested(&self) -> bool {
        (self.io_registers[0x203] >> 1) & 1 == 1
    }
    pub fn dma2_interrupt_requested(&self) -> bool {
        (self.io_registers[0x203] >> 2) & 1 == 1
    }
    pub fn dma3_interrupt_requested(&self) -> bool {
        (self.io_registers[0x203] >> 3) & 1 == 1
    }
    pub fn keypad_interrupt_requested(&self) -> bool {
        (self.io_registers[0x203] >> 4) & 1 == 1
    }
    pub fn game_pak_interrupt_requested(&self) -> bool {
        (self.io_registers[0x203] >> 5) & 1 == 1
    }

    pub fn set_lcdc_vblank_interrupt(&mut self, value: bool) {
        self.io_registers[0x202] &= !1;
        self.io_registers[0x202] |= value as u8;
    }
    pub fn set_lcdc_hblank_interrupt(&mut self, value: bool) {
        self.io_registers[0x202] &= !(1 << 1);
        self.io_registers[0x202] |= (value as u8) << 1;
    }
    pub fn set_lcdc_vcounter_interrupt(&mut self, value: bool) {
        self.io_registers[0x202] &= !(1 << 2);
        self.io_registers[0x202] |= (value as u8) << 2;
    }
    pub fn set_timer0_interrupt(&mut self, value: bool) {
        self.io_registers[0x202] &= !(1 << 3);
        self.io_registers[0x202] |= (value as u8) << 3;
    }
    pub fn set_timer1_interrupt(&mut self, value: bool) {
        self.io_registers[0x202] &= !(1 << 4);
        self.io_registers[0x202] |= (value as u8) << 4;
    }
    pub fn set_timer2_interrupt(&mut self, value: bool) {
        self.io_registers[0x202] &= !(1 << 5);
        self.io_registers[0x202] |= (value as u8) << 5;
    }
    pub fn set_timer3_interrupt(&mut self, value: bool) {
        self.io_registers[0x202] &= !(1 << 6);
        self.io_registers[0x202] |= (value as u8) << 6;
    }
    pub fn set_timer_interrupt(&mut self, timer: u8, value: bool) {
        match timer {
            0 => self.set_timer0_interrupt(value),
            1 => self.set_timer1_interrupt(value),
            2 => self.set_timer2_interrupt(value),
            3 => self.set_timer3_interrupt(value),
            _ => panic!("Invalid timer number"),
        }
    }
    pub fn set_serial_interrupt(&mut self, value: bool) {
        self.io_registers[0x202] &= !(1 << 7);
        self.io_registers[0x202] |= (value as u8) << 7;
    }
    pub fn set_dma0_interrupt(&mut self, value: bool) {
        self.io_registers[0x203] &= !1;
        self.io_registers[0x203] |= value as u8;
    }
    pub fn set_dma1_interrupt(&mut self, value: bool) {
        self.io_registers[0x203] &= !(1 << 1);
        self.io_registers[0x203] |= (value as u8) << 1;
    }
    pub fn set_dma2_interrupt(&mut self, value: bool) {
        self.io_registers[0x203] &= !(1 << 2);
        self.io_registers[0x203] |= (value as u8) << 2;
    }
    pub fn set_dma3_interrupt(&mut self, value: bool) {
        self.io_registers[0x203] &= !(1 << 3);
        self.io_registers[0x203] |= (value as u8) << 3;
    }
    pub fn set_keypad_interrupt(&mut self, value: bool) {
        self.io_registers[0x203] &= !(1 << 4);
        self.io_registers[0x203] |= (value as u8) << 4;
    }
    pub fn set_game_pak_interrupt(&mut self, value: bool) {
        self.io_registers[0x203] &= !(1 << 5);
        self.io_registers[0x203] |= (value as u8) << 5;
    }

    pub fn dma3_source_address(&self) -> u32 {
        (self.io_registers[0x0D7] as u32) << 24
            | (self.io_registers[0x0D6] as u32) << 16
            | (self.io_registers[0x0D5] as u32) << 8
            | (self.io_registers[0x0D4] as u32)
    }
    pub fn dma3_dest_address(&self) -> u32 {
        (self.io_registers[0x0DE] as u32) << 24
            | (self.io_registers[0x0DC] as u32) << 16
            | (self.io_registers[0x0DB] as u32) << 8
            | (self.io_registers[0x0DA] as u32)
    }
    pub fn dma3_count(&self) -> u16 {
        (self.io_registers[0x0DD] as u16) << 8 | (self.io_registers[0x0DC] as u16)
    }

    pub fn get_mem8(&self, address: u32) -> (u8, u8) {
        match address {
            //0x00000000..=0x00003FFF => {
            0x00000000..=0x01FFFFFF => {
                // bios system ROM
                // todo!("bios system ROM")
                // HACK:
                (0, 0)
            }
            //0x02000000..=0x0203FFFF => {
            0x02000000..=0x02FFFFFF => {
                // on-board work ram
                (self.wram[(address & 0x3FFFF) as usize], 3)
            }
            //0x03000000..=0x03007FFF => (self.iw_ram[(address & 0x7FFF) as usize], 1),
            0x03000000..=0x03FFFFFF => (self.iw_ram[(address & 0x7FFF) as usize], 1),
            //0x04000000..=0x040003FE => (self.io_registers[(address & 0x3FE) as usize], 1),
            0x04000000..=0x04FFFFFF => (self.io_registers.get_mem8(address), 1),
            //0x05000000..=0x050003FF => (self.obj_palette_ram[(address & 0x3FF) as usize], 1),
            0x05000000..=0x05FFFFFF => (self.obj_palette_ram[(address & 0x3FF) as usize], 1),
            //0x06000000..=0x06017FFF => (self.vram[(address & 0x17FFF) as usize], 1),
            0x06000000..=0x06FFFFFF => (self.vram[(address & 0x17FFF) as usize], 1),
            //0x07000000..=0x070003FF => (self.oam[(address & 0x3FF) as usize], 1),
            0x07000000..=0x07FFFFFF => (self.oam[(address & 0x3FF) as usize], 1),
            0x08000000..=0x09FFFFFF => {
                if (address - 0x0800_0000) > self.entire_rom.len() as u32 {
                    return (0, 5);
                }
                (self.entire_rom[(address & 0x1FFFFFF) as usize], 5)
            }
            0x0A000000..=0x0BFFFFFF => todo!("Game Pak ROM/FlashROM (max 32MB) - Wait State 1"),
            0x0C000000..=0x0DFFFFFF => todo!("Game Pak ROM/FlashROM (max 32MB) - Wait State 2"),
            //0x0E000000..=0x0E00FFFF => todo!("Game Pak SRAM    (max 64 KBytes) - 8bit Bus width"),
            0x0E000000..=0x0FFFFFFF => (self.sram[(address & 0xFFFF) as usize], 5),
            _ => (0, 0),
        }
    }
    pub fn get_mem16(&self, address: u32) -> (u16, u8) {
        let lo_bit_idx = address & !0x1;
        let hi_bit_idx = address | 0x1;
        match address {
            0x00000000..=0x01FFFFFF => {
                //0x00000000..=0x00003FFF => {
                // bios system ROM
                // TODO: separate opcoed reading logic from these getters
                let lo_bit = self.bios[(lo_bit_idx & 0x3FFF) as usize] as u16;
                let hi_bit = self.bios[(hi_bit_idx & 0x3FFF) as usize] as u16;
                ((hi_bit << 8) | lo_bit, 1)
            }
            0x02000000..=0x02FFFFFF => {
                //0x02000000..=0x0203FFFF => {
                // on-board work ram
                let lo_bit = self.wram[(lo_bit_idx & 0x3FFFF) as usize] as u16;
                let hi_bit = self.wram[(hi_bit_idx & 0x3FFFF) as usize] as u16;
                ((hi_bit << 8) | lo_bit, 3)
            }
            0x03000000..=0x03FFFFFF => {
                //0x03000000..=0x03007FFF => {
                let lo_bit = self.iw_ram[(lo_bit_idx & 0x7FFF) as usize] as u16;
                let hi_bit = self.iw_ram[(hi_bit_idx & 0x7FFF) as usize] as u16;
                ((hi_bit << 8) | lo_bit, 1)
            }
            0x04000000..=0x04FFFFFF => {
                //0x04000000..=0x040003FF => {
                (self.io_registers.get_mem16(address), 1)
            }
            0x05000000..=0x05FFFFFF => {
                //0x05000000..=0x050003FF => {
                let lo_bit = self.obj_palette_ram[(lo_bit_idx & 0x3FF) as usize] as u16;
                let hi_bit = self.obj_palette_ram[(hi_bit_idx & 0x3FF) as usize] as u16;
                ((hi_bit << 8) | lo_bit, 1)
            }
            0x06000000..=0x06FFFFFF => {
                //0x06000000..=0x06017FFF => {
                let lo_bit = self.vram[(lo_bit_idx & 0x17FFF) as usize] as u16;
                let hi_bit = self.vram[(hi_bit_idx & 0x17FFF) as usize] as u16;
                ((hi_bit << 8) | lo_bit, 1)
            }
            0x07000000..=0x07FFFFFF => {
                //0x07000000..=0x070003FF => {
                let lo_bit = self.oam[(lo_bit_idx & 0x3FF) as usize] as u16;
                let hi_bit = self.oam[(hi_bit_idx & 0x3FF) as usize] as u16;
                ((hi_bit << 8) | lo_bit, 1)
            }
            0x08000000..=0x09FFFFFF => {
                if (hi_bit_idx - 0x0800_0000) > self.entire_rom.len() as u32 {
                    return (0, 5);
                }
                let lo_bit = self.entire_rom[(lo_bit_idx & 0x1FFFFFF) as usize] as u16;
                let hi_bit = self.entire_rom[(hi_bit_idx & 0x1FFFFFF) as usize] as u16;
                ((hi_bit << 8) | lo_bit, 5)
            }
            0x0A000000..=0x0BFFFFFF => {
                if (hi_bit_idx - 0x0A00_0000) > self.entire_rom.len() as u32 {
                    return (0, 5);
                }
                let lo_bit = self.entire_rom[(lo_bit_idx & 0x1FFFFFF) as usize] as u16;
                let hi_bit = self.entire_rom[(hi_bit_idx & 0x1FFFFFF) as usize] as u16;
                ((hi_bit << 8) | lo_bit, 5)
            }
            0x0C000000..=0x0DFFFFFF => {
                if (hi_bit_idx - 0x0C00_0000) > self.entire_rom.len() as u32 {
                    return (0, 5);
                }
                let lo_bit = self.entire_rom[(lo_bit_idx & 0x1FFFFFF) as usize] as u16;
                let hi_bit = self.entire_rom[(hi_bit_idx & 0x1FFFFFF) as usize] as u16;
                ((hi_bit << 8) | lo_bit, 5)
            }
            //0x0E000000..=0x0E00FFFF => todo!("Game Pak SRAM    (max 64 KBytes) - 8bit Bus width"),
            0x0E000000..=0x0FFFFFFF => todo!("Game Pak SRAM    (max 64 KBytes) - 8bit Bus width"),
            _ => (0, 0),
        }
    }
    pub fn get_mem32(&self, address: u32) -> (u32, u8) {
        //address = address & 0x0FFF_FFFF;
        let bit1_idx = address & !0x3;
        let bit2_idx = (address & !0x3) | 0b01;
        let bit3_idx = (address & !0x3) | 0b10;
        let bit4_idx = (address & !0x3) | 0b11;
        match address {
            0x00000000..=0x01FFFFFF => {
                //0x00000000..=0x00003FFF => {
                // bios system ROM
                let bit1 = self.bios[(bit1_idx & 0x3FFF) as usize] as u32;
                let bit2 = self.bios[(bit2_idx & 0x3FFF) as usize] as u32;
                let bit3 = self.bios[(bit3_idx & 0x3FFF) as usize] as u32;
                let bit4 = self.bios[(bit4_idx & 0x3FFF) as usize] as u32;
                let out = (bit4 << 24) | (bit3 << 16) | (bit2 << 8) | bit1;
                (out, 1)
            }
            0x02000000..=0x02FFFFFF => {
                //0x02000000..=0x0203FFFF => {
                // on-board work ram
                let bit1 = self.wram[(bit1_idx & 0x3FFFF) as usize] as u32;
                let bit2 = self.wram[(bit2_idx & 0x3FFFF) as usize] as u32;
                let bit3 = self.wram[(bit3_idx & 0x3FFFF) as usize] as u32;
                let bit4 = self.wram[(bit4_idx & 0x3FFFF) as usize] as u32;
                let out = (bit4 << 24) | (bit3 << 16) | (bit2 << 8) | bit1;
                (out, 6)
            }
            0x03000000..=0x03FFFFFF => {
                //0x03000000..=0x03007FFF => {
                let bit1 = self.iw_ram[(bit1_idx & 0x7FFF) as usize] as u32;
                let bit2 = self.iw_ram[(bit2_idx & 0x7FFF) as usize] as u32;
                let bit3 = self.iw_ram[(bit3_idx & 0x7FFF) as usize] as u32;
                let bit4 = self.iw_ram[(bit4_idx & 0x7FFF) as usize] as u32;
                let out = (bit4 << 24) | (bit3 << 16) | (bit2 << 8) | bit1;
                (out, 1)
            }
            0x04000000..=0x04FFFFFE => {
                //0x04000000..=0x040003FE => {
                (self.io_registers.get_mem32(address), 1)
            }
            0x05000000..=0x05FFFFFF => {
                //0x05000000..=0x050003FF => {
                let bit1 = self.obj_palette_ram[(bit1_idx & 0x3FF) as usize] as u32;
                let bit2 = self.obj_palette_ram[(bit2_idx & 0x3FF) as usize] as u32;
                let bit3 = self.obj_palette_ram[(bit3_idx & 0x3FF) as usize] as u32;
                let bit4 = self.obj_palette_ram[(bit4_idx & 0x3FF) as usize] as u32;
                let out = (bit4 << 24) | (bit3 << 16) | (bit2 << 8) | bit1;
                (out, 2)
            }
            0x06000000..=0x06FFFFFF => {
                //0x06000000..=0x06017FFF => {
                let bit1 = self.vram[(bit1_idx & 0x17FFF) as usize] as u32;
                let bit2 = self.vram[(bit2_idx & 0x17FFF) as usize] as u32;
                let bit3 = self.vram[(bit3_idx & 0x17FFF) as usize] as u32;
                let bit4 = self.vram[(bit4_idx & 0x17FFF) as usize] as u32;
                let out = (bit4 << 24) | (bit3 << 16) | (bit2 << 8) | bit1;
                (out, 2)
            }
            0x07000000..=0x07FFFFFF => {
                //0x07000000..=0x070003FF => {
                let bit1 = self.oam[(bit1_idx & 0x3FF) as usize] as u32;
                let bit2 = self.oam[(bit2_idx & 0x3FF) as usize] as u32;
                let bit3 = self.oam[(bit3_idx & 0x3FF) as usize] as u32;
                let bit4 = self.oam[(bit4_idx & 0x3FF) as usize] as u32;
                let out = (bit4 << 24) | (bit3 << 16) | (bit2 << 8) | bit1;
                (out, 1)
            }
            0x0A000000..=0x0BFFFFFF | 0x0C000000..=0x0DFFFFFF | 0x08000000..=0x09FFFFFF => {
                // TODO: properly handle later bytes overflowing too
                if bit1_idx & 0x1FFFFFF >= self.entire_rom.len() as u32 {
                    return (0, 8);
                }
                let bit1 = self.entire_rom[(bit1_idx & 0x1FFFFFF) as usize] as u32;
                let bit2 = self.entire_rom[(bit2_idx & 0x1FFFFFF) as usize] as u32;
                let bit3 = self.entire_rom[(bit3_idx & 0x1FFFFFF) as usize] as u32;
                let bit4 = self.entire_rom[(bit4_idx & 0x1FFFFFF) as usize] as u32;
                let out = (bit4 << 24) | (bit3 << 16) | (bit2 << 8) | bit1;
                (out, 8)
            }
            /*
            0x0A000000..=0x0BFFFFFF => todo!("Game Pak ROM/FlashROM (max 32MB) - Wait State 1"),
            0x0C000000..=0x0DFFFFFF => todo!("Game Pak ROM/FlashROM (max 32MB) - Wait State 2"),
            //0x0E000000..=0x0E00FFFF => todo!("Game Pak SRAM    (max 64 KBytes) - 8bit Bus width"),
            */
            0x0E000000..=0x0FFFFFFF => todo!("Game Pak SRAM    (max 64 KBytes) - 8bit Bus width"),
            _ => (0, 0),
        }
    }

    pub fn set_mem8(&mut self, address: u32, val: u8) -> u8 {
        match address {
            0x00000000..=0x00003FFF => {
                // bios system ROM
                //todo!("bios system ROM")
                1
            }
            0x02000000..=0x02FFFFFF => {
                //0x02000000..=0x0203FFFF => {
                // on-board work ram
                self.wram[(address & 0x3FFFF) as usize] = val;
                3
            }
            0x03000000..=0x03FFFFFF => {
                //0x03000000..=0x03007FFF => {
                self.iw_ram[(address & 0x7FFF) as usize] = val;
                1
            }
            0x04000000..=0x04FFFFFF => {
                //0x04000000..=0x040003FE => {
                self.io_registers.set_mem8(address, val);
                1
            }
            0x05000000..=0x050003FF => {
                todo!("OBJ Pallete ram")
            }
            //0x06000000..=0x06017FFF => {
            0x06000000..=0x06FFFFFF => {
                // INACCUARY: GBA can't do 8 bit writes here
                self.vram[(address & 0x17FFF) as usize] = val;
                1
            }
            0x07000000..=0x070003FF => {
                todo!("OAM")
            }
            0x08000000..=0x09FFFFFF => todo!("Game Pak ROM/FlashROM (max 32MB) - Wait State 0"),
            0x0A000000..=0x0BFFFFFF => todo!("Game Pak ROM/FlashROM (max 32MB) - Wait State 1"),
            0x0C000000..=0x0DFFFFFF => todo!("Game Pak ROM/FlashROM (max 32MB) - Wait State 2"),
            //0x0E000000..=0x0E00FFFF => todo!("Game Pak SRAM    (max 64 KBytes) - 8bit Bus width"),
            0x0E000000..=0x0FFFFFFF => {
                self.sram[(address & 0xFFFF) as usize] = val;
                5
            }
            _ => 0,
        }
    }

    pub fn set_mem16(&mut self, address: u32, val: u16) -> u8 {
        let lo_bit_idx = address & !0x1;
        let hi_bit_idx = address | 0x1;
        let lo_val = (val & 0xFF) as u8;
        let hi_val = ((val >> 8) & 0xFF) as u8;
        match address {
            0x00000000..=0x00003FFF => {
                // bios system ROM
                // HACK: disable
                //todo!("bios system ROM")
                0
            }
            0x02000000..=0x02FFFFFF => {
                //0x02000000..=0x0203FFFF => {
                // on-board work ram
                self.wram[(lo_bit_idx & 0x3FFFF) as usize] = lo_val;
                self.wram[(hi_bit_idx & 0x3FFFF) as usize] = hi_val;
                3
            }
            0x03000000..=0x03FFFFFF => {
                //0x03000000..=0x03007FFF => {
                self.iw_ram[(lo_bit_idx & 0x7FFF) as usize] = lo_val;
                self.iw_ram[(hi_bit_idx & 0x7FFF) as usize] = hi_val;
                1
            }
            0x04000000..=0x04FFFFFE => {
                //0x04000000..=0x040003FE => {
                self.io_registers.set_mem16(address, val);
                1
            }
            0x05000000..=0x05FFFFFF => {
                //0x05000000..=0x050003FF => {
                self.obj_palette_ram[(lo_bit_idx & 0x3FF) as usize] = lo_val;
                self.obj_palette_ram[(hi_bit_idx & 0x3FF) as usize] = hi_val;
                1
            }
            0x06000000..=0x06FFFFFF => {
                //0x06000000..=0x06017FFF => {
                self.vram[(lo_bit_idx & 0x17FFF) as usize] = lo_val;
                self.vram[(hi_bit_idx & 0x17FFF) as usize] = hi_val;
                1
            }
            0x07000000..=0x07FFFFFF => {
                //0x07000000..=0x070003FF => {
                self.oam[(lo_bit_idx & 0x3FF) as usize] = lo_val;
                self.oam[(hi_bit_idx & 0x3FF) as usize] = hi_val;
                1
            }
            0x08000000..=0x09FFFFFF => 0, //todo!("Game Pak ROM/FlashROM (max 32MB) - Wait State 0"),
            0x0A000000..=0x0BFFFFFF => todo!("Game Pak ROM/FlashROM (max 32MB) - Wait State 1"),
            0x0C000000..=0x0DFFFFFF => todo!("Game Pak ROM/FlashROM (max 32MB) - Wait State 2"),
            //0x0E000000..=0x0E00FFFF => todo!("Game Pak SRAM    (max 64 KBytes) - 8bit Bus width"),
            0x0E000000..=0x0FFFFFFF => todo!("Game Pak SRAM    (max 64 KBytes) - 8bit Bus width"),
            _ => 0,
        }
    }

    pub fn set_mem32(&mut self, address: u32, val: u32) -> u8 {
        let bit1_idx = address & !0x3;
        let bit2_idx = (address & !0x3) | 0b01;
        let bit3_idx = (address & !0x3) | 0b10;
        let bit4_idx = (address & !0x3) | 0b11;
        let val1 = (val & 0xFF) as u8;
        let val2 = ((val >> 8) & 0xFF) as u8;
        let val3 = ((val >> 16) & 0xFF) as u8;
        let val4 = ((val >> 24) & 0xFF) as u8;
        match address {
            0x00000000..=0x00003FFF => {
                // bios system ROM
                // HACK: disable this for now, we don't want to write but something is trying to
                //todo!("bios system ROM")
                0
            }
            0x02000000..=0x02FFFFFF => {
                //0x02000000..=0x0203FFFF => {
                // on-board work ram
                self.wram[(bit1_idx & 0x3FFFF) as usize] = val1;
                self.wram[(bit2_idx & 0x3FFFF) as usize] = val2;
                self.wram[(bit3_idx & 0x3FFFF) as usize] = val3;
                self.wram[(bit4_idx & 0x3FFFF) as usize] = val4;
                6
            }
            0x03000000..=0x03FFFFFF => {
                //0x03000000..=0x03007FFF => {
                self.iw_ram[(bit1_idx & 0x7FFF) as usize] = val1;
                self.iw_ram[(bit2_idx & 0x7FFF) as usize] = val2;
                self.iw_ram[(bit3_idx & 0x7FFF) as usize] = val3;
                self.iw_ram[(bit4_idx & 0x7FFF) as usize] = val4;
                1
            }
            0x04000000..=0x04FFFFFE => {
                //0x04000000..=0x040003FE => {
                self.io_registers.set_mem32(address, val);
                1
            }
            0x05000000..=0x05FFFFFF => {
                //0x05000000..=0x050003FF => {
                self.obj_palette_ram[(bit1_idx & 0x3FF) as usize] = val1;
                self.obj_palette_ram[(bit2_idx & 0x3FF) as usize] = val2;
                self.obj_palette_ram[(bit3_idx & 0x3FF) as usize] = val3;
                self.obj_palette_ram[(bit4_idx & 0x3FF) as usize] = val4;
                2
            }
            0x06000000..=0x06FFFFFF => {
                //0x06000000..=0x06017FFF => {
                self.vram[(bit1_idx & 0x17FFF) as usize] = val1;
                self.vram[(bit2_idx & 0x17FFF) as usize] = val2;
                self.vram[(bit3_idx & 0x17FFF) as usize] = val3;
                self.vram[(bit4_idx & 0x17FFF) as usize] = val4;
                2
            }
            0x07000000..=0x07FFFFFF => {
                //0x07000000..=0x070003FF => {
                self.oam[(bit1_idx & 0x3FF) as usize] = val1;
                self.oam[(bit2_idx & 0x3FF) as usize] = val2;
                self.oam[(bit3_idx & 0x3FF) as usize] = val3;
                self.oam[(bit4_idx & 0x3FF) as usize] = val4;
                1
            }
            // TODO:  4000204h - WAITCNT - Waitstate Control (R/W)
            // HACK: test ROM is writing here, can't find clear docs
            0x08000000..=0x09FFFFFF => 0, /*todo!(
            "Game Pak ROM/FlashROM (max 32MB) - Wait State 0: 0x{:X}",
            address
            ),*/
            0x0A000000..=0x0BFFFFFF => todo!("Game Pak ROM/FlashROM (max 32MB) - Wait State 1"),
            0x0C000000..=0x0DFFFFFF => todo!("Game Pak ROM/FlashROM (max 32MB) - Wait State 2"),
            //0x0E000000..=0x0E00FFFF => todo!("Game Pak SRAM    (max 64 KBytes) - 8bit Bus width"),
            0x0E000000..=0x0FFFFFFF => todo!("Game Pak SRAM    (max 64 KBytes) - 8bit Bus width"),
            _ => 0,
        }
    }

    fn irq_interrupt(&mut self) {
        self.r.set_irq_mode();
        // Docs say `SUBS PC,R14,4   ;both PC=R14_irq-4, and CPSR=SPSR_irq'
        // to return from IRQ, so we add 4
        self.r[14] = self.r.pc + 4;
        *self.r.get_spsr_mut() = self.r.cpsr;
        self.r.set_thumb(false);
        self.r.cpsr_disable_irq();
        self.r.pc = 0x18;
    }

    pub fn maybe_handle_interrupts(&mut self) {
        // TODO: when this function handles more than just IRQ, this logic must be changed
        if self.r.irq_disabled() {
            return;
        }
        if self.lcdc_vblank_interrupt_requested() && self.lcdc_hblank_interrupt_enabled() {
            debug!("VBLANK interrupt started");
            self.irq_interrupt();
        } else if self.lcdc_hblank_interrupt_requested() && self.lcdc_hblank_interrupt_enabled() {
            debug!("HBLANK interrupt started");
            self.irq_interrupt();
        } else if self.lcdc_vcounter_interrupt_requested() && self.lcdc_vcounter_interrupt_enabled()
        {
            debug!("VCOUNTER interrupt started");
            self.irq_interrupt();
        } else if self.timer0_interrupt_requested() && self.timer0_interrupt_enabled() {
            debug!("TIMER0 interrupt started");
            self.irq_interrupt();
        } else if self.timer1_interrupt_requested() && self.timer1_interrupt_enabled() {
            debug!("TIMER1 interrupt started");
            self.irq_interrupt();
        } else if self.timer2_interrupt_requested() && self.timer2_interrupt_enabled() {
            debug!("TIMER2 interrupt started");
            self.irq_interrupt();
        } else if self.timer3_interrupt_requested() && self.timer3_interrupt_enabled() {
            debug!("TIMER3 interrupt started");
            self.irq_interrupt();
        } else if self.serial_interrupt_requested() && self.serial_interrupt_enabled() {
            debug!("SERIAL interrupt started");
            self.irq_interrupt();
        } else if self.dma0_interrupt_requested() && self.dma0_interrupt_enabled() {
            debug!("DMA0 interrupt started");
            self.irq_interrupt();
        } else if self.dma1_interrupt_requested() && self.dma1_interrupt_enabled() {
            debug!("DMA1 interrupt started");
            self.irq_interrupt();
        } else if self.dma2_interrupt_requested() && self.dma2_interrupt_enabled() {
            debug!("DMA2 interrupt started");
            self.irq_interrupt();
        } else if self.dma3_interrupt_requested() && self.dma3_interrupt_enabled() {
            debug!("DMA3 interrupt started");
            self.irq_interrupt();
        } else if self.keypad_interrupt_requested() && self.keypad_interrupt_enabled() {
            debug!("KEYPAD interrupt started");
            self.irq_interrupt();
        } else if self.game_pak_interrupt_requested() && self.game_pak_interrupt_enabled() {
            debug!("GAMEPAK interrupt started");
            self.irq_interrupt();
        }
    }

    pub fn run_dma(&mut self, control: DmaControl, src: u32, dest: u32, count: u32) -> u32 {
        let mut cycles = 0;
        match control.start_timing {
            DmaStartTiming::Immediately => {
                println!(
                    "Transfering data from 0x{:X} to 0x{:X}: 0x{:X} bytes",
                    src, dest, count
                );
                if control.transfer_type {
                    for i in 0..count {
                        let val = self.get_mem32(src + (i * 4) as u32);
                        let o = self.set_mem32(dest + (i * 4) as u32, val.0);
                        cycles += val.1 as u32 + o as u32;
                    }
                } else {
                    for i in 0..count {
                        let val = self.get_mem16(src + (i * 2) as u32);
                        let o = self.set_mem16(dest + (i * 2) as u32, val.0);
                        cycles += val.1 as u32 + o as u32;
                    }
                }
                if control.repeat {
                    todo!("repeat dma");
                }
            }
            DmaStartTiming::VBlank => {
                todo!("VBlank DMA timing not implemented")
            }
            DmaStartTiming::HBlank => {
                todo!("HBlank DMA timing not implemented")
            }
            DmaStartTiming::Special => {
                todo!("Special DMA timing not implemented");
            }
        }
        cycles
    }

    pub fn handle_dma(&mut self) -> u32 {
        if self.io_registers.dma0_enabled {
            self.io_registers.dma0_enabled = false;
            let dma = self.io_registers.dma0();
            let src = self.io_registers.dma0_source_addr();
            let dest = self.io_registers.dma0_dest_addr();
            let mut count = self.io_registers.dma0_word_count();
            if count == 0 {
                count = 0x4000;
            }
            let out = self.run_dma(dma, src, dest, count as u32);
            if dma.irq_at_end && self.dma0_interrupt_enabled() {
                self.set_dma0_interrupt(true);
            }
            self.io_registers.disable_dma0();
            out
        } else if self.io_registers.dma1_enabled {
            self.io_registers.dma1_enabled = false;
            let dma = self.io_registers.dma1();
            let src = self.io_registers.dma1_source_addr();
            let dest = self.io_registers.dma1_dest_addr();
            let mut count = self.io_registers.dma1_word_count();
            if count == 0 {
                count = 0x4000;
            }
            let out = self.run_dma(dma, src, dest, count as u32);
            if dma.irq_at_end && self.dma1_interrupt_enabled() {
                self.set_dma1_interrupt(true);
            }
            self.io_registers.disable_dma1();
            out
        } else if self.io_registers.dma2_enabled {
            self.io_registers.dma2_enabled = false;
            let dma = self.io_registers.dma2();
            let src = self.io_registers.dma2_source_addr();
            let dest = self.io_registers.dma2_dest_addr();
            let mut count = self.io_registers.dma2_word_count();
            if count == 0 {
                count = 0x4000;
            }
            let out = self.run_dma(dma, src, dest, count as u32);
            if dma.irq_at_end && self.dma2_interrupt_enabled() {
                self.set_dma2_interrupt(true);
            }
            self.io_registers.disable_dma2();
            out
        } else if self.io_registers.dma3_enabled {
            self.io_registers.dma3_enabled = false;
            let dma = self.io_registers.dma3();
            let src = self.io_registers.dma3_source_addr();
            let dest = self.io_registers.dma3_dest_addr();
            let mut count = self.io_registers.dma3_word_count() as u32;
            if count == 0 {
                count = 0x10000;
            }
            let out = self.run_dma(dma, src, dest, count);
            if dma.irq_at_end && self.dma3_interrupt_enabled() {
                self.set_dma3_interrupt(true);
            }
            self.io_registers.disable_dma3();
            out
        } else {
            0
        }
    }

    // TODO: aligned reads
    pub fn get_opcode(&self) -> u32 {
        // TODO: the timing of reading from mem for PC should be handled?
        self.get_mem32(self.r.pc).0
    }

    pub fn get_thumb_opcode(&self) -> u16 {
        // TODO: the timing of reading from mem for PC should be handled?
        self.get_mem16(self.r.pc).0
    }

    pub fn dispatch(&mut self) -> u32 {
        if self.io_registers.dma_waiting() {
            return self.handle_dma();
        }
        if self.get_mem16(0x4000202).0 != 0 && self.master_interrupts_enabled() {
            self.maybe_handle_interrupts();
        }
        if self.r.thumb_enabled() {
            return self.dispatch_thumb() as u32;
        }
        let opcode = self.get_opcode();
        let opcode_idx = (opcode >> 25) & 0x7;

        self.r.pc = self.r.pc.wrapping_add(4);

        // TODO: some instructions can't be skipped, handle those
        if opcode == 0 {
            //self.r.pc = self.r.pc.wrapping_add(4);
            return 4;
        }
        trace!("opcode: {:032b} at 0x{:X}", opcode, self.r.pc - 4);
        let cond = Cond::from_u8(((opcode >> 28) & 0xF) as u8);
        if !self.cond_should_execute(cond) {
            trace!("Skipped!");
            return 1;
        }

        if (opcode >> 8) & 0xF_FFFF == 0b0001_0010_1111_1111_1111 {
            let cycles = self.dispatch_branch_and_exchange(opcode);
            // don't increment PC when switching execution modes
            // TODO: this might be more complicated than this
            //self.r.pc += 4;
            return cycles as u32;
        }

        let cycles = match opcode_idx {
            0b101 => self.dispatch_branch(opcode),
            0b000 => {
                let multiply_end = ((opcode >> 4) & 0xF) == 0b1001;
                let bits8to11 = (opcode >> 8) & 0xF;
                let multiply_next3 = (opcode >> 22) & 0x7;
                let multiply_next2 = multiply_next3 >> 1;
                match multiply_next2 {
                    0b00 if multiply_next3 == 0 && multiply_end => self.dispatch_multiply(opcode),
                    0b01 if multiply_end => self.dispatch_multiply(opcode),
                    0b10 if multiply_end && bits8to11 == 0 && ((opcode >> 20) & 0x3) == 0 => {
                        self.dispatch_swap(opcode)
                    }
                    0b10 if (opcode >> 20) & 1 == 0 && ((opcode >> 4) & 0xFF) == 0 => {
                        self.dispatch_psr(opcode)
                    }
                    _ => {
                        if (opcode >> 4) & 1 == 1 && (opcode >> 7) & 1 == 1 {
                            if (opcode >> 22) & 1 == 0 && (opcode >> 8) & 0xF == 0 {
                                self.dispatch_data_trans(opcode)
                            } else {
                                self.dispatch_data_trans(opcode)
                            }
                        } else {
                            self.dispatch_alu(opcode)
                        }
                    }
                }
            }
            0b001 => {
                let next2 = (opcode >> 23) & 0x3;
                //if next2 == 0b10 && (opcode >> 20) & 3 == 0b10 {
                if next2 == 0b10 && (opcode >> 20) & 1 == 0 {
                    self.dispatch_psr(opcode)
                } else {
                    self.dispatch_alu(opcode)
                }
            }
            0b010 | 0b011 => self.dispatch_mem(opcode),
            0b100 => self.dispatch_block_data(opcode),
            // TODO: 0b100 block trans
            // TODO: 0b110 co data trans
            0b111 => {
                let next_bit = (opcode >> 24) & 1 == 1;
                if next_bit {
                    self.dispatch_software_interrupt(opcode)
                } else {
                    todo!("SWI with next bit not set");
                }
            }
            //0b111 => self.dispatch_codata_op(opcode),
            _ => {
                unimplemented!("0x{:X} ({:b}) at 0x{:X}", opcode, opcode_idx, self.r.pc);
            }
        };

        cycles as u32
    }

    pub fn dispatch_thumb(&mut self) -> u8 {
        let opcode = self.get_thumb_opcode();
        let opcode_idx = (opcode >> 13) & 0x7;
        self.r.pc += 2;

        if opcode == 0 {
            return 4;
        }
        trace!("THUMB opcode: {:016b} at 0x{:X}", opcode, self.r.pc - 2);

        let cycles = match opcode_idx {
            0b000 => self.dispatch_thumb_shift_add_sub(opcode),
            0b001 => self.dispatch_thumb_imm(opcode),
            0b010 => {
                let next_bit = (opcode >> 12) & 0x1 == 1;
                if next_bit {
                    if (opcode >> 9) & 1 == 1 {
                        self.dispatch_thumb_load_store_halfword_sign_extend(opcode)
                    } else {
                        self.dispatch_thumb_load_store_reg(opcode)
                    }
                } else {
                    let sub_op = (opcode >> 10) & 0x3;
                    match sub_op {
                        0b00 => self.dispatch_thumb_alu(opcode),
                        0b01 => self.dispatch_thumb_hi_reg_branch(opcode),
                        _ => self.dispatch_thumb_load_pc_relative(opcode),
                    }
                }
            }
            0b011 => self.dispatch_thumb_load_store_imm(opcode),
            0b100 => {
                let next_bit = (opcode >> 12) & 0x1 == 1;
                if next_bit {
                    self.dispatch_thumb_load_store_sp_relative(opcode)
                } else {
                    self.dispatch_thumb_load_store_halfword(opcode)
                }
            }
            0b101 => {
                let next_bit = (opcode >> 12) & 0x1 == 1;
                let sub_op = (opcode >> 9) & 0x3;
                match (next_bit, sub_op) {
                    (false, _) => self.dispatch_thumb_get_relative_address(opcode),
                    (true, 0b10) => self.dispatch_thumb_push_pop(opcode),
                    // technically more instructions are here but not for the armv4 CPU
                    (true, _) => self.dispatch_thumb_add_offset_to_sp(opcode),
                }
            }
            0b110 => {
                let next_bit = (opcode >> 12) & 0x1 == 1;
                if next_bit {
                    let cond = (opcode >> 8) & 0xF;
                    if cond == 0b1111 {
                        self.dispatch_thumb_software_interrupt(opcode)
                    } else {
                        self.dispatch_thumb_conditional_branch(opcode)
                    }
                } else {
                    self.dispatch_thumb_load_store_multiple(opcode)
                }
            }
            0b111 => self.dispatch_thumb_branch(opcode),
            _ => unimplemented!(
                "THUMB 0x{:X} ({:b}) at 0x{:X}",
                opcode,
                opcode_idx,
                self.r.pc
            ),
        };

        cycles
    }

    pub fn dispatch_thumb_alu(&mut self, opcode: u16) -> u8 {
        let sub_op = (opcode >> 6) & 0xF;
        let rs = ((opcode >> 3) & 0x7) as u8;
        let rd = (opcode & 0x7) as u8;
        let mut skip_end_flags = false;

        let cycles = match sub_op {
            0b0000 => {
                trace!("AND r{} = r{} & r{}", rd, rs, rd);
                self.r[rd] = self.r[rs] & self.r[rd];
                1
            }
            0b0001 => {
                trace!("EOR r{} = r{} ^ r{}", rd, rs, rd);
                self.r[rd] = self.r[rs] ^ self.r[rd];
                1
            }
            0b0010 => {
                trace!("LSL r{} = r{} << r{}", rd, rs, rd);
                let shift_amt = self.r[rs] & 0xFF;
                let val = self.r[rd];
                self.r[rd] = BarrelShifter::lsl(val, shift_amt, Some(&mut self.r));

                2
            }
            0b0011 => {
                trace!("LSR r{} = r{} >> r{}", rd, rs, rd);
                let shift_amt = self.r[rs] & 0xFF;
                if shift_amt != 0 {
                    let val = self.r[rd];
                    self.r[rd] = BarrelShifter::lsr(val, shift_amt, Some(&mut self.r));
                }

                2
            }
            0b0100 => {
                trace!("ASR r{} = r{} >> r{}", rd, rs, rd);
                let shift_amt = self.r[rs] & 0xFF;
                if shift_amt != 0 {
                    let val = self.r[rd];
                    self.r[rd] = BarrelShifter::asr(val, shift_amt, Some(&mut self.r));
                }

                2
            }
            0b0101 => {
                trace!("ADC r{} = r{} + r{} + C", rd, rs, rd);
                let old_val = self.r[rd];
                // TODO: this is wrong, if rd = rs, then the flag logic is wrong
                // this applies to many instructions
                self.r[rd] = self.r[rs]
                    .wrapping_add(self.r[rd])
                    .wrapping_add(self.r.cpsr_carry_flag() as u32);
                self.r.cpsr_set_overflow_flag(
                    (!(self.r[rs] ^ old_val) & (old_val ^ self.r[rd])) >> 31 == 1,
                );
                self.r.cpsr_set_carry_flag(
                    ((self.r[rs] as u64) + (old_val as u64) + (self.r.cpsr_carry_flag() as u64))
                        > 0xFFFF_FFFF,
                );

                1
            }
            0b0110 => {
                trace!("SBC r{} = r{} - r{} - C", rd, rs, rd);
                let op1 = self.r[rs];
                let op2 = self.r[rd];
                self.r[rd] = self.r[rd]
                    .wrapping_sub(self.r[rs].wrapping_add(1 - self.r.cpsr_carry_flag() as u32));
                let a = op2;
                let b = !op1;
                self.r
                    .cpsr_set_overflow_flag((!(a ^ b) & (b ^ self.r[rd])) >> 31 == 1);
                self.r.cpsr_set_carry_flag(
                    ((a as u64) + (b as u64) + (self.r.cpsr_carry_flag() as u64)) > 0xFFFF_FFFF,
                );
                1
            }
            0b0111 => {
                trace!("ROR r{} = r{} ROR r{}", rd, rs, rd);
                self.r[rd] = self.r[rd].rotate_right(self.r[rs] & 0xFF);
                if self.r[rs] & 0xFF != 0 {
                    self.r.cpsr_set_carry_flag(self.r[rd] >> 31 == 1);
                }
                2
            }
            0b1000 => {
                trace!("TST r{} & r{}", rs, rd);
                let result = self.r[rs] & self.r[rd];
                self.r.cpsr_set_zero_flag(result == 0);
                self.r.cpsr_set_sign_flag(result & 0x8000_0000 != 0);
                skip_end_flags = true;
                1
            }
            0b1001 => {
                trace!("NEG r{} = -r{}", rd, rs);
                let old_val = self.r[rd];
                self.r[rd] = 0i32.wrapping_sub(self.r[rs] as i32) as u32;
                // TODO: review
                self.r.cpsr_set_overflow_flag(
                    old_val & 0x8000_0000 == 0 && self.r[rd] & 0x8000_0000 != 0,
                );
                self.r.cpsr_set_carry_flag(false);
                1
            }
            0b1010 => {
                trace!("CMP r{} - r{}", rs, rd);
                let op1 = self.r[rd];
                let op2 = self.r[rs];
                let result = op1.wrapping_sub(op2);
                self.r.cpsr_set_zero_flag(result == 0);
                self.r.cpsr_set_sign_flag((result >> 31) == 1);
                self.r
                    .cpsr_set_overflow_flag((op1 as i32).overflowing_sub(op2 as i32).1);
                self.r.cpsr_set_carry_flag(op2 <= op1);
                skip_end_flags = true;
                1
            }
            0b1011 => {
                trace!("CMN r{} + r{}", rs, rd);
                let old_val = self.r[rd];
                let result = self.r[rd].wrapping_add(self.r[rs]);
                self.r.cpsr_set_zero_flag(result == 0);
                self.r.cpsr_set_sign_flag(result & 0x8000_0000 != 0);
                self.r.cpsr_set_overflow_flag(
                    ((old_val ^ self.r[rs]) & 0x8000_0000 == 0)
                        && ((old_val ^ result) & 0x8000_0000 != 0),
                );
                self.r
                    .cpsr_set_carry_flag(old_val.checked_add(self.r[rs]).is_none());
                skip_end_flags = true;
                1
            }
            0b1100 => {
                trace!(
                    "ORR r{} = r{} (0x{:X}) | r{} (0x{:X})",
                    rd,
                    rs,
                    self.r[rs],
                    rd,
                    self.r[rd]
                );
                self.r[rd] = self.r[rd] | self.r[rs];
                1
            }
            0b1101 => {
                trace!("MUL r{} = r{} * r{}", rd, rs, rd);
                // TODO: update carry flag if GBA should be ARMv4. ARMv5 and above does not need to
                self.r[rd] = self.r[rd].wrapping_mul(self.r[rs]);
                4
            }
            0b1110 => {
                trace!("BIC r{} = r{} & ~r{}", rd, rs, rd);
                self.r[rd] = self.r[rd] & !self.r[rs];
                1
            }
            0b1111 => {
                trace!("MVN r{} = ~r{}", rd, rs);
                self.r[rd] = !self.r[rs];
                1
            }
            _ => unreachable!(),
        };
        if !skip_end_flags {
            self.r.cpsr_set_zero_flag(self.r[rd] == 0);
            self.r.cpsr_set_sign_flag((self.r[rd] >> 31) & 1 == 1);
        }
        cycles
    }

    pub fn dispatch_thumb_load_pc_relative(&mut self, opcode: u16) -> u8 {
        let rd = ((opcode >> 8) & 0x7) as u8;
        let nn = (opcode & 0xFF) << 2;
        let pc = (self.r.pc + 2) & !2;
        let addr = pc + nn as u32;

        let o = self.get_mem32(addr);
        trace!("LDR r{}, [PC, #{} (0x{:X})] (0x{:X})", rd, nn, addr, o.0);
        self.r[rd] = o.0;

        2 + o.1
    }

    pub fn dispatch_thumb_hi_reg_branch(&mut self, opcode: u16) -> u8 {
        let subop = (opcode >> 8) & 0x3;
        let rs = ((opcode >> 3) & 0xF) as u8;
        let mut rd = (opcode & 0x7) as u8;
        if subop != 3 {
            let hi_bit = (opcode >> 7) & 1 == 1;
            rd |= (hi_bit as u8) << 3;
        }
        let old_thumb_enabled = self.r.thumb_enabled();
        let cycles = match subop {
            0b00 => {
                trace!("ADD r{}, r{}", rd, rs);
                self.r[rd] = self.r[rd].wrapping_add(self.r[rs]);
                1
            }
            0b01 => {
                trace!("CMP r{}, r{}", rd, rs);
                let op1 = self.r[rd];
                let op2 = self.r[rs];
                let result = self.r[rd].wrapping_sub(self.r[rs]);
                self.r.cpsr_set_zero_flag(result == 0);
                self.r.cpsr_set_sign_flag(result & 0x8000_0000 != 0);
                self.r
                    .cpsr_set_overflow_flag((op1 as i32).overflowing_sub(op2 as i32).1);
                self.r.cpsr_set_carry_flag(op2 <= op1);
                1
            }
            0b10 => {
                trace!("MOV r{}, r{}", rd, rs);
                self.r[rd] = self.r[rs];
                1
            }
            0b11 => {
                let x_flag = (opcode >> 7) & 1 == 1;
                let thumb_mode = self.r[rs] & 1 == 1;
                let (thumb_mode, addr) = if rs == 15 {
                    (false, (self.r[rs] + 2) & !3)
                } else {
                    if thumb_mode {
                        (true, self.r[rs] & !1)
                    } else {
                        (false, (self.r[rs] + 2) & !3)
                    }
                };
                self.r.set_thumb(thumb_mode);
                if x_flag {
                    trace!("BLX r{}", rs);
                    let old_pc = self.r.pc + 4;
                    self.r.pc = addr;
                    *self.r.lr_mut() = old_pc + 1;
                    /*
                    self.r.pc = (self.r[rs] + 4) & !2;
                    // *self.r.lr_mut() = old_pc + 3;
                    *self.r.lr_mut() = old_pc + 1;
                    */
                } else {
                    trace!("BX r{} (0x{:X})", rs, addr);
                    // TODO(hello-world): check if this is correct
                    self.r.pc = addr; //+ 4;
                                      /*
                                      self.r.pc = (self.r[rs] + 4) & !2;
                                      //self.r.pc = (self.r[rs] + 2) & !1;
                                      */
                }
                if old_thumb_enabled != thumb_mode {
                    if thumb_mode {
                        trace!("Enabling Thumb mode");
                    } else {
                        trace!("Enabling ARM mode!");
                    }
                }
                3
            }
            _ => unreachable!(),
        };
        cycles
    }

    pub fn dispatch_thumb_load_store_reg(&mut self, opcode: u16) -> u8 {
        let sub_op_idx = (opcode >> 11) & 0x3;
        let ro = (opcode >> 6 & 0x7) as u8;
        let rb = (opcode >> 3 & 0x7) as u8;
        let rd = (opcode & 0x7) as u8;

        match sub_op_idx {
            0b00 => {
                trace!("STR r{}, [r{}, r{}]", rd, rb, ro);
                let o = self.set_mem32(self.r[rb] + self.r[ro], self.r[rd]);
                1 + o
            }
            0b01 => {
                trace!("STRB r{}, [r{}, r{}]", rd, rb, ro);
                let o = self.set_mem8(self.r[rb] + self.r[ro], self.r[rd] as u8);
                1 + o
            }
            0b10 => {
                trace!("LDR r{}, [r{}, r{}]", rd, rb, ro);
                let o = self.get_mem32(self.r[rb].wrapping_add(self.r[ro]));
                self.r[rd] = o.0;
                2 + o.1
            }
            0b11 => {
                trace!("LDRB r{}, [r{}, r{}]", rd, rb, ro);
                let o = self.get_mem8(self.r[rb].wrapping_add(self.r[ro]));
                self.r[rd] = o.0 as u32;
                2 + o.1
            }
            _ => unreachable!(),
        }
    }

    pub fn dispatch_thumb_load_store_halfword_sign_extend(&mut self, opcode: u16) -> u8 {
        let sub_opcode = (opcode >> 10) & 0x3;
        let ro = (opcode >> 6 & 0x7) as u8;
        let rb = (opcode >> 3 & 0x7) as u8;
        let rd = (opcode & 0x7) as u8;

        match sub_opcode {
            0b00 => {
                trace!("STRH r{}, [r{}, r{}]", rd, rb, ro);
                let o = self.set_mem16(self.r[rb].wrapping_add(self.r[ro]), self.r[rd] as u16);
                1 + o
            }
            0b01 => {
                trace!("LDSB r{}, [r{}, r{}]", rd, rb, ro);
                let o = self.get_mem8(self.r[rb].wrapping_add(self.r[ro]));
                // TODO: double check that this sign extends properly
                self.r[rd] = (o.0 as i8) as i32 as u32;
                2 + o.1
            }
            0b10 => {
                trace!("LDRH r{}, [r{}, r{}]", rd, rb, ro);
                let o = self.get_mem16(self.r[rb].wrapping_add(self.r[ro]));
                self.r[rd] = o.0 as u32;
                2 + o.1
            }
            0b11 => {
                trace!("LDSH r{}, [r{}, r{}]", rd, rb, ro);
                let o = self.get_mem16(self.r[rb].wrapping_add(self.r[ro]));
                // TODO: double check that this sign extends properly
                self.r[rd] = (o.0 as i16) as i32 as u32;
                2 + o.1
            }
            _ => unreachable!(),
        }
    }

    pub fn dispatch_thumb_load_store_halfword(&mut self, opcode: u16) -> u8 {
        let sub_opcode = (opcode >> 11) & 0x1 == 1;
        let offset = (opcode >> 6 & 0x1F) as u32;
        let rb = (opcode >> 3 & 0x7) as u8;
        let rd = (opcode & 0x7) as u8;
        if sub_opcode {
            // LDRH
            trace!("LDRH r{}, [r{}, r{}]", rd, rb, offset);
            let o = self.get_mem16(self.r[rb] + (offset * 2));
            self.r[rd] = o.0 as u32;
            2 + o.1
        } else {
            // STRH
            trace!("STRH r{}, [r{}, r{}]", rd, rb, offset);
            let o = self.set_mem16(self.r[rb] + (offset * 2), self.r[rd] as u16);
            1 + o
        }
    }

    pub fn dispatch_thumb_load_store_imm(&mut self, opcode: u16) -> u8 {
        let sub_op_idx = (opcode >> 11) & 0x3;
        let offset = (opcode >> 6 & 0x1F) as u32;
        let rb = (opcode >> 3 & 0x7) as u8;
        let rd = (opcode & 0x7) as u8;

        match sub_op_idx {
            0b00 => {
                trace!("STR r{}, [r{}, #{}]", rd, rb, offset);
                let o = self.set_mem32(self.r[rb].wrapping_add(offset * 4), self.r[rd]);
                // 2N?
                1 + o
            }
            0b01 => {
                trace!("LDR r{}, [r{}, #{}]", rd, rb, offset);
                let o = self.get_mem32(self.r[rb].wrapping_add(offset * 4));
                self.r[rd] = o.0;
                2 + o.1
            }
            0b10 => {
                trace!("STRB r{}, [r{}, #{}]", rd, rb, offset);
                let o = self.set_mem8(self.r[rb].wrapping_add(offset), self.r[rd] as u8);
                // 2N?
                1 + o
            }
            0b11 => {
                trace!("LDRB r{}, [r{}, #{}]", rd, rb, offset);
                // TODO: do we clear the upper 24 bits here?
                let o = self.get_mem8(self.r[rb].wrapping_add(offset));
                self.r[rd] = o.0 as u32;
                2 + o.1
            }
            _ => unreachable!(),
        }
    }

    pub fn dispatch_thumb_load_store_multiple(&mut self, opcode: u16) -> u8 {
        let subop = (opcode >> 11) & 0x1 == 1;
        let rb = (opcode >> 8 & 0x7) as u8;
        let r_list = opcode & 0xFF;
        // Execution Time: nS+1N+1I for LDM, or (n-1)S+2N for STM.
        let mut cycles = 0;

        // TODO: figure out order

        if subop {
            trace!("LDMIA r{}, {{{}}}", rb, r_list);
            for i in 0..8 {
                if r_list & (1 << i) != 0 {
                    cycles += 2;
                    let o = self.get_mem32(self.r[rb]);
                    self.r[rb] = self.r[rb].wrapping_add(4);
                    self.r[i as u8] = o.0;
                    cycles += o.1;
                }
            }
        } else {
            trace!("STMIA r{}, {{{}}}", rb, r_list);
            for i in 0..8 {
                if r_list & (1 << i) != 0 {
                    cycles += 1;
                    //println!("Storing value 0x{:X} to address 0x{:X}", self.r[i as u8], self.r[rb]);
                    cycles += self.set_mem32(self.r[rb], self.r[i as u8]);
                    self.r[rb] = self.r[rb].wrapping_add(4);
                }
            }
        }
        cycles
    }

    pub fn dispatch_thumb_load_store_sp_relative(&mut self, opcode: u16) -> u8 {
        let subop = (opcode >> 11) & 0x1 == 1;
        let rd = ((opcode >> 8) & 0x7) as u8;
        let nn = (opcode & 0xFF) as u32;

        if subop {
            trace!("LDR r{}, [SP, #{}]", rd, nn);
            let o = self.get_mem32(self.r.sp() + (nn * 4));
            self.r[rd] = o.0;
            o.1 + 2
        } else {
            trace!("STR r{}, [SP, #{}]", rd, nn);
            let o = self.set_mem32(self.r.sp() + (nn * 4), self.r[rd]);
            o * 2
        }
    }

    pub fn dispatch_thumb_get_relative_address(&mut self, opcode: u16) -> u8 {
        let subop = (opcode >> 11) & 0x1 == 1;
        let rd = (opcode >> 8 & 0x7) as u8;
        let nn = (opcode & 0xFF) as u32;

        if subop {
            trace!("ADD r{}, SP, #{}", rd, nn);
            self.r[rd] = self.r.sp() + (nn * 4);
        } else {
            trace!("ADD r{}, PC, #{}", rd, nn);
            self.r[rd] = ((self.r.pc + 2) & !2) + (nn * 4);
        }

        1
    }

    pub fn dispatch_thumb_push_pop(&mut self, opcode: u16) -> u8 {
        let subop = (opcode >> 11) & 0x1 == 1;
        let pc_lr = (opcode >> 8) & 0x1 == 1;
        let r_list = opcode & 0xFF;
        let mut cycles = 0;

        if subop {
            trace!("POP at 0x{:X}", self.r.sp());

            for i in 0..8 {
                if r_list & (1 << i) != 0 {
                    let o = self.get_mem32(self.r.sp());
                    *self.r.sp_mut() += 4;
                    self.r[i as u8] = o.0;
                    //println!("popping value 0x{:X} to r{}", o.0, i);
                    cycles += o.1;
                    cycles += 2;
                }
            }
            if pc_lr {
                let o = self.get_mem32(self.r.sp());
                *self.r.sp_mut() += 4;
                self.r.pc = o.0 & !1;
                cycles += o.1;
                cycles += 2;
            }
            // 0 1 2 3 4
            // 4 3 2 1 0
        } else {
            trace!("PUSH at 0x{:X}", self.r.sp());
            if pc_lr {
                cycles += 1;
                *self.r.sp_mut() -= 4;
                //println!("pushing LR ({:X}) to 0x{:X}", self.r.lr(), self.r.sp());
                cycles += self.set_mem32(self.r.sp(), self.r.lr());
            }
            for i in (0..8).rev() {
                if r_list & (1 << i) != 0 {
                    cycles += 1;
                    // REVIEW: docs suggest this happens first
                    *self.r.sp_mut() -= 4;
                    //println!("pushing r{} ({:X}) to 0x{:X}", i, self.r[i as u8], self.r.sp());
                    cycles += self.set_mem32(self.r.sp(), self.r[i as u8]);
                }
            }
        }

        cycles
    }

    pub fn dispatch_thumb_add_offset_to_sp(&mut self, opcode: u16) -> u8 {
        let sign = (opcode >> 7) & 0x1 == 1;
        let nn = (opcode & 0x7F) as u32;
        if sign {
            trace!("SUB SP, #{}", nn);
            *self.r.sp_mut() -= nn * 4;
        } else {
            trace!("ADD SP, #{}", nn);
            *self.r.sp_mut() += nn * 4;
        }
        1
    }

    pub fn dispatch_thumb_imm(&mut self, opcode: u16) -> u8 {
        let sub_op_idx = (opcode >> 11) & 0x3;
        let rd = (opcode >> 8 & 0x7) as u8;
        let imm = (opcode & 0xFF) as u32;

        match sub_op_idx {
            // MOV
            0b00 => {
                trace!("MOV r{}, #{}", rd, imm);
                self.r[rd] = imm;
                self.r.cpsr_set_zero_flag(self.r[rd] == 0);
                self.r.cpsr_set_sign_flag(self.r[rd] & 0x8000_0000 != 0);
            }
            // CMP
            0b01 => {
                trace!("CMP r{}, #{}", rd, imm);
                let result = self.r[rd].wrapping_sub(imm);
                self.r.cpsr_set_zero_flag(result == 0);
                self.r.cpsr_set_sign_flag(result & 0x8000_0000 != 0);
                self.r
                    .cpsr_set_overflow_flag((self.r[rd] as i32).overflowing_sub(imm as i32).1);
                self.r.cpsr_set_carry_flag(imm <= self.r[rd]);
            }
            // ADD
            0b10 => {
                trace!("ADD r{}, #{}", rd, imm);
                let result = self.r[rd].wrapping_add(imm);
                self.r.cpsr_set_zero_flag(result == 0);
                self.r.cpsr_set_sign_flag(result & 0x8000_0000 != 0);
                self.r
                    .cpsr_set_overflow_flag((self.r[rd] as i32).overflowing_add(imm as i32).1);
                self.r
                    .cpsr_set_carry_flag(((self.r[rd] as u64) + (imm as u64)) > 0xFFFF_FFFF);
                self.r[rd] = result;
            }
            // SUB
            0b11 => {
                trace!("SUB r{} = r{} (0x{:X}) - #{}", rd, rd, self.r[rd], imm);
                let result = self.r[rd].wrapping_sub(imm);
                self.r.cpsr_set_zero_flag(result == 0);
                self.r.cpsr_set_sign_flag(result & 0x8000_0000 != 0);
                self.r
                    .cpsr_set_overflow_flag((self.r[rd] as i32).overflowing_sub(imm as i32).1);
                self.r.cpsr_set_carry_flag(imm <= self.r[rd]);
                self.r[rd] = result;
            }
            _ => unreachable!(),
        }

        1
    }

    pub fn dispatch_thumb_shift_add_sub(&mut self, opcode: u16) -> u8 {
        let sub_op_idx = (opcode >> 11) & 0x3;
        let offset = (opcode >> 6) & 0x1F;
        let rs = ((opcode >> 3) & 0x7) as u8;
        let rd = (opcode & 0x7) as u8;
        match sub_op_idx {
            // LSL
            0b00 => {
                trace!("LSL r{}, r{}, #{:X}", rd, rs, offset);
                let val = self.r[rs];
                let shift_amt = offset as u32;
                let registers = Some(&mut self.r);
                self.r[rd] = BarrelShifter::lsl(val, shift_amt, registers);

                self.r.cpsr_set_sign_flag(self.r[rd] & 0x8000_0000 != 0);
                self.r.cpsr_set_zero_flag(self.r[rd] == 0);
            }
            0b01 => {
                trace!("LSR r{}, r{}, #{:X}", rd, rs, offset);
                let val = self.r[rs];
                let shift_amt = if offset == 0 { 32 } else { offset as u32 };
                let registers = Some(&mut self.r);
                self.r[rd] = BarrelShifter::lsr(val, shift_amt, registers);

                self.r.cpsr_set_sign_flag(self.r[rd] & 0x8000_0000 != 0);
                self.r.cpsr_set_zero_flag(self.r[rd] == 0);
            }
            0b10 => {
                trace!("ASR r{}, r{}, #{:X}", rd, rs, offset);
                let val = self.r[rs];
                let shift_amt = if offset == 0 { 32 } else { offset as u32 };
                let registers = Some(&mut self.r);
                self.r[rd] = BarrelShifter::asr(val, shift_amt, registers);

                self.r.cpsr_set_sign_flag(self.r[rd] & 0x8000_0000 != 0);
                self.r.cpsr_set_zero_flag(self.r[rd] == 0);
            }
            0b11 => {
                let sub_sub_op_idx = (opcode >> 9) & 0x3;
                let reg_or_imm = ((opcode >> 6) & 0x7) as u32;
                let result;
                let op2;
                match sub_sub_op_idx {
                    0b00 => {
                        trace!("ADD r{}, r{}, r{}", rd, rs, reg_or_imm);
                        op2 = self.r[reg_or_imm as u8];
                        result = self.r[rs].wrapping_add(op2);
                        self.r.cpsr_set_overflow_flag(
                            (self.r[rs] as i32).overflowing_add(op2 as i32).1,
                        );
                        self.r
                            .cpsr_set_carry_flag((self.r[rs] as u64) + (op2 as u64) > 0xFFFF_FFFF);
                        self.r[rd] = result;
                    }
                    0b01 => {
                        trace!("SUB r{}, r{}, r{}", rd, rs, reg_or_imm);
                        op2 = self.r[reg_or_imm as u8];
                        result = self.r[rs].wrapping_sub(op2);
                        self.r.cpsr_set_overflow_flag(
                            (self.r[rs] as i32).overflowing_sub(op2 as i32).1,
                        );
                        self.r.cpsr_set_carry_flag(op2 <= self.r[rs]);
                        self.r[rd] = result;
                    }
                    0b10 => {
                        trace!("ADD r{}, r{}, #{}", rd, rs, reg_or_imm);
                        op2 = reg_or_imm;
                        result = self.r[rs].wrapping_add(op2);
                        self.r.cpsr_set_overflow_flag(
                            (self.r[rs] as i32).overflowing_add(op2 as i32).1,
                        );
                        self.r.cpsr_set_carry_flag(
                            ((self.r[rs] as u64) + (op2 as u64)) > 0xFFFF_FFFF,
                        );
                        self.r[rd] = result;
                    }
                    0b11 => {
                        trace!("SUB r{}, r{}, #{}", rd, rs, reg_or_imm);
                        op2 = reg_or_imm;
                        result = self.r[rs].wrapping_sub(op2);
                        self.r.cpsr_set_overflow_flag(
                            (self.r[rs] as i32).overflowing_sub(op2 as i32).1,
                        );
                        self.r.cpsr_set_carry_flag(op2 <= self.r[rs]);
                        self.r[rd] = result;
                    }
                    _ => unreachable!(),
                }

                self.r.cpsr_set_zero_flag(self.r[rd] == 0);
                self.r.cpsr_set_sign_flag(self.r[rd] & 0x8000_0000 != 0);
            }
            _ => unreachable!(),
        }

        1
    }

    pub fn dispatch_software_interrupt(&mut self, _opcode: u32) -> u8 {
        if self.master_interrupts_enabled() {
            self.r.set_svc_mode();
            *self.r.lr_mut() = self.r.pc;
            *self.r.get_spsr_mut() = self.r.cpsr;
            self.r.set_thumb(false);
            self.r.cpsr_disable_irq();
            self.r.pc = 0x08;
        }

        3
    }

    pub fn dispatch_thumb_software_interrupt(&mut self, _opcode: u16) -> u8 {
        if self.master_interrupts_enabled() {
            self.r.set_svc_mode();
            *self.r.lr_mut() = self.r.pc | 0;
            *self.r.get_spsr_mut() = self.r.cpsr;
            self.r.set_thumb(false);
            self.r.cpsr_disable_irq();
            self.r.pc = 0x08;
        }

        3
    }

    pub fn dispatch_thumb_conditional_branch(&mut self, opcode: u16) -> u8 {
        let cond = Cond::from_u8(((opcode >> 8) & 0xF) as u8);
        trace!("B{:?} #{:X}", cond, opcode & 0xFF);
        let nn = (opcode & 0xFF) as i8 as i32 * 2;
        if self.cond_should_execute(cond) {
            self.r.pc = ((self.r.pc as i32) + 2 + nn) as u32;
            3
        } else {
            1
        }
    }

    pub fn dispatch_thumb_branch(&mut self, opcode: u16) -> u8 {
        let sub_op_idx = (opcode >> 11) & 0x3;
        match sub_op_idx {
            0b00 => {
                // unconditional branch, thumb.18
                let sign = (opcode >> 10) & 1 == 1;
                let signed_offset = (opcode & 0x7FF) as i16 | ((if sign { 0xF8 } else { 0 }) << 8);
                //println!("{} {:b} ({:b})", sign, signed_offset, signed_offset as i32);
                let signed_offset = signed_offset as i32 * 2;
                let new_pc = (self.r.pc as i32 + 2 + signed_offset) as u32;
                trace!(
                    "B #{:X} + 2 + #{:X} = #{:X}",
                    self.r.pc,
                    signed_offset,
                    new_pc
                );
                self.r.pc = new_pc;
                3
            }
            0b10 => {
                let upper_n = (opcode & 0x7FF) as u32;
                /*
                let double_width_hack = false;
                let next_op =  self.get_thumb_opcode();
                if double_width_hack && next_op >> 11 == 0x1F {
                    // just do the entire jump here
                    let lower_n = (next_op & 0x7FF) as u32;
                    let signed_offset = (((upper_n as i32) << 21) >> 9) | ((lower_n as i32) << 1);
                    println!("{}, {:b}", signed_offset, signed_offset);
                    *self.r.lr_mut() = (self.r.pc + 2) | 1;
                    self.r.pc = ((self.r.pc as i32) + 2 + signed_offset) as u32;

                    trace!("BL (32bit THUMB) to 0x{:X}", self.r.pc);
                    4
                } else {
                    */
                trace!("BL (part 1)");
                // TODO: review all other sign extension logic, apply this trick to them
                let offset = ((upper_n as i32) << 21) >> 9;
                *self.r.lr_mut() = ((self.r.pc as i32) + 2 + offset) as u32;

                1
                //}
            }
            0b11 /*| 0b01*/ => {
                let n = (opcode & 0x7FF) as u32;
                let old_pc = self.r.pc;
                let new_pc = self.r.lr().wrapping_add(n << 1);
                self.r.pc = new_pc;
                trace!("BL to 0x{:X}", self.r.pc);
                *self.r.lr_mut() = old_pc | 1;
                //println!("old_pc = 0x{:X}, n = {} (0x{:X})", old_pc, n, n);
                3
            }
            // I think 0b01 is only on ARM9, so let's just route it to the same BL
            0b01 => {
                todo!("Second opcode for THUMB branch long with link");
            }
            _ => unimplemented!("unknown sub-op-idx for THUMB branch: {:b}", sub_op_idx),
        }
    }

    pub fn dispatch_codata_op(&mut self, opcode: u32) -> u8 {
        let final_bit = (opcode >> 24) & 1 == 1;
        if final_bit {
            // software interrupt
            // HACK: sub 4 so PC increments to correct address
            self.r.pc = 0x8 - 4;
            //self.r.pc = 0x03007F08 - 4;
            self.r.set_svc_mode();
        } else {
            let coproc_opcode = (opcode >> 21) & 0x7;
            todo!()
        }
        // TODO: timing
        5
    }

    // branch and branch and link
    pub fn dispatch_branch(&mut self, opcode: u32) -> u8 {
        let sub_opcode = (opcode >> 24) & 1 == 1;
        let sign = (opcode >> 23) & 1 == 1;
        let signed_offset = (opcode & 0xFF_FFFF) as i32 | (if sign { 0xFF } else { 0 } << 24);
        /*
        let signed_offset = if sign {
            let bit = 1 << 23;
            let num24bit = (opcode & 0xFF_FFFF) as i32;
            (num24bit ^ bit) - bit

        } else {
            (opcode & 0xFF_FFFF) as i32
        };
        */

        if sub_opcode {
            trace!("Linking for branch");
            //*self.r.lr_mut() = self.r.pc; + 4;
            *self.r.lr_mut() = self.r.pc;
        }
        let new_pc = self.r.pc as i32 + 4 + (signed_offset * 4);
        //let new_pc = self.r.pc as i32 + 8 + (signed_offset * 4);
        trace!(
            "Branching at 0x{:X} to 0x{:X} with offset {} {:b}",
            self.r.pc,
            new_pc,
            signed_offset * 4,
            signed_offset
        );
        self.r.pc = new_pc as u32;

        // TODO: timing
        // 2S + 1N
        3
    }

    pub fn dispatch_alu(&mut self, opcode: u32) -> u8 {
        let sub_opcode = ((opcode >> 21) as u8) & 0xF;

        let s = (opcode >> 20) & 1 == 1;
        let imm = (opcode >> 25) & 1 == 1;
        let rn = ((opcode >> 16) & 0xF) as u8;
        let rd = ((opcode >> 12) & 0xF) as u8;
        let mut op1 = self.r[rn];
        let mut op2;
        // HACK for R15 case
        let old_cpsr = self.r.cpsr;

        //dbg!(sub_opcode, s, imm, rn, rd);
        // valid programs shouldn't give us invalid instructions
        // these checks are mostly for debugging, as hitting these in known good code suggests
        // we're executing garbage data as code
        debug_assert!(if matches!(sub_opcode, 0xD | 0xF) {
            rn == 0
        } else {
            true
        });
        debug_assert!(if matches!(sub_opcode, 0xA | 0xB | 0x8 | 0x9) {
            rd == 0xF || rd == 0
        } else {
            true
        });
        // opcodes that don't write back must set flags
        //debug_assert!(if (0x8..=0xB).contains(&sub_opcode) { s } else { true });

        if rn == 0xF {
            if !imm && (opcode >> 4) & 1 == 1 {
                op1 += 12 - 4;
            } else {
                op1 += 8 - 4;
            }
        }

        if imm {
            let ror_shift = (opcode >> 8) & 0xF;
            op2 = opcode & 0xFF;
            let shift_amt = ror_shift * 2;
            op2 = op2.rotate_right(shift_amt);
        } else {
            let shift_by_register = (opcode >> 4) & 1 == 1;
            let rm = opcode & 0xF;
            let shift_amt = if shift_by_register {
                let shift_reg_idx = (opcode >> 8) & 0xF;
                // docs say this must be true, TODO: deal with this later
                assert_eq!((opcode >> 7) & 1, 0);
                if shift_reg_idx > 14 {
                    warn!(
                        "shift reg too high in ALU shift by reg: {} in {:X}",
                        shift_reg_idx, sub_opcode
                    );
                    //panic!("shift reg too high in ALU shift by reg: {}", shift_reg_idx);
                }
                (self.r[shift_reg_idx as u8] & 0xFF) as u8
            } else {
                ((opcode >> 7) & 0x1F) as u8
            };
            let shift_type = (opcode >> 5) & 0b11;

            if shift_amt == 0 && shift_by_register {
                op2 = self.r[rm as u8];
            } else {
                let carry = self.r.cpsr_carry_flag();
                let val = self.r[rm as u8];
                let registers = if s { Some(&mut self.r) } else { None };
                match shift_type {
                    0 => op2 = BarrelShifter::lsl(val, shift_amt as u32, registers),
                    1 => op2 = BarrelShifter::lsr(val, shift_amt as u32, registers),
                    2 => op2 = BarrelShifter::asr(val, shift_amt as u32, registers),
                    3 if shift_amt == 0 => op2 = BarrelShifter::rrx(val, registers, carry),
                    3 => op2 = BarrelShifter::ror(val, shift_amt as u32, registers),
                    _ => unreachable!(),
                }
            }
        }

        match sub_opcode {
            // AND
            0x0 => {
                trace!("AND r{} = {:X} & {:X}", rd, op1, op2);
                let result = op1 & op2;
                self.r[rd] = result;
                if s {
                    self.r.cpsr_set_zero_flag(result == 0);
                    self.r.cpsr_set_sign_flag((result >> 31) == 1);
                }
            }
            // EOR
            0x1 => {
                trace!("EOR r{} = {:X} ^ {:X}", rd, op1, op2);
                let result = op1 ^ op2;
                self.r[rd] = result;
                if s {
                    self.r.cpsr_set_zero_flag(result == 0);
                    self.r.cpsr_set_sign_flag((result >> 31) == 1);
                }
            }
            // SUB
            0x2 => {
                trace!("SUB r{} = {:X} - {:X}", rd, op1, op2);
                let result = op1.wrapping_sub(op2);
                self.r[rd] = result;
                if s {
                    self.r.cpsr_set_zero_flag(result == 0);
                    self.r.cpsr_set_sign_flag(result & 0x8000_0000 != 0);
                    self.r
                        .cpsr_set_overflow_flag((op1 as i32).overflowing_sub(op2 as i32).1);
                    self.r.cpsr_set_carry_flag(op2 <= op1);
                }
            }
            // RSB
            0x3 => {
                trace!("RSB r{} = {:X} - {:X}", rd, op2, op1);
                let result = op2.wrapping_sub(op1);
                self.r[rd] = result;
                if s {
                    self.r.cpsr_set_zero_flag(result == 0);
                    self.r.cpsr_set_sign_flag(result & 0x8000_0000 != 0);
                    self.r
                        .cpsr_set_overflow_flag((op2 as i32).overflowing_sub(op1 as i32).1);
                    self.r.cpsr_set_carry_flag(op1 <= op2);
                }
            }
            // ADD
            0x4 => {
                trace!("ADD r{} = 0x{:X} + 0x{:X}", rd, op1, op2);
                let result = op1.wrapping_add(op2);
                self.r[rd] = result;
                if s {
                    self.r.cpsr_set_zero_flag(result == 0);
                    self.r.cpsr_set_sign_flag((result >> 31) == 1);
                    self.r
                        .cpsr_set_overflow_flag((op1 as i32).overflowing_add(op2 as i32).1);
                    self.r
                        .cpsr_set_carry_flag(((op1 as u64) + (op2 as u64)) > 0xFFFF_FFFF);
                }
            }
            // ADC
            0x5 => {
                trace!("ADC r{} = {:X} + {:X} + C", rd, op1, op2);
                let result = op1
                    .wrapping_add(op2)
                    .wrapping_add(self.r.cpsr_carry_flag() as u32);
                self.r[rd] = result;
                if s {
                    self.r.cpsr_set_zero_flag(result == 0);
                    self.r.cpsr_set_sign_flag((result >> 31) == 1);
                    self.r
                        .cpsr_set_overflow_flag((!(op1 ^ op2) & (op2 ^ result)) >> 31 == 1);
                    self.r.cpsr_set_carry_flag(
                        ((op1 as u64) + (op2 as u64) + (self.r.cpsr_carry_flag() as u64))
                            > 0xFFFF_FFFF,
                    );
                }
            }
            // SBC
            0x6 => {
                trace!("SBC r{} = {:X} - {:X} + C", rd, op1, op2);
                let result =
                    op1.wrapping_sub(op2.wrapping_add(1 - self.r.cpsr_carry_flag() as u32));
                self.r[rd] = result;
                if s {
                    self.r.cpsr_set_zero_flag(result == 0);
                    self.r.cpsr_set_sign_flag((result >> 31) == 1);
                    let a = op1;
                    let b = !op2;
                    self.r
                        .cpsr_set_overflow_flag((!(a ^ b) & (b ^ result)) >> 31 == 1);
                    self.r.cpsr_set_carry_flag(
                        ((a as u64) + (b as u64) + (self.r.cpsr_carry_flag() as u64)) > 0xFFFF_FFFF,
                    );
                }
            }
            // RSC
            0x7 => {
                trace!("RSC r{} = {:X} - {:X} + C", rd, op2, op1);
                let result =
                    op2.wrapping_sub(op1.wrapping_add(1 - self.r.cpsr_carry_flag() as u32));
                self.r[rd] = result;
                let a = op2;
                let b = !op1;
                if s {
                    self.r.cpsr_set_zero_flag(result == 0);
                    self.r.cpsr_set_sign_flag((result >> 31) == 1);
                    self.r
                        .cpsr_set_overflow_flag((!(a ^ b) & (b ^ result)) >> 31 == 1);
                    self.r.cpsr_set_carry_flag(
                        ((a as u64) + (b as u64) + (self.r.cpsr_carry_flag() as u64)) > 0xFFFF_FFFF,
                    );
                }
            }
            // TST
            0x8 => {
                trace!("TST {:X} & {:X}", op1, op2);
                let result = op1 & op2;
                if s {
                    self.r.cpsr_set_zero_flag(result == 0);
                    self.r.cpsr_set_sign_flag((result >> 31) == 1);
                }
            }
            // TEQ
            0x9 => {
                trace!("TEQ {:X} ^ {:X}", op1, op2);
                let result = op1 ^ op2;
                if s {
                    self.r.cpsr_set_zero_flag(result == 0);
                    self.r.cpsr_set_sign_flag((result >> 31) == 1);
                }
            }
            // CMP
            0xA => {
                trace!("CMP {:X} - {:X}", op1, op2);

                let result = op1.wrapping_sub(op2);
                if s {
                    self.r.cpsr_set_zero_flag(result == 0);
                    self.r.cpsr_set_sign_flag(result & 0x8000_0000 != 0);
                    self.r
                        .cpsr_set_overflow_flag((op1 as i32).overflowing_sub(op2 as i32).1);
                    self.r.cpsr_set_carry_flag(op2 <= op1);
                }
            }
            // CMN
            0xB => {
                trace!("CMN {:X} + {:X}", op1, op2);
                let result = op1.wrapping_add(op2);
                if s {
                    self.r.cpsr_set_zero_flag(result == 0);
                    self.r.cpsr_set_sign_flag((result >> 31) == 1);
                    self.r
                        .cpsr_set_overflow_flag((op1 as i32).overflowing_add(op2 as i32).1);
                    self.r.cpsr_set_carry_flag(op1.checked_add(op2).is_none());
                }
            }
            // ORR
            0xC => {
                trace!("ORR r{} = {:X} | {:X}", rd, op1, op2);
                let result = op1 | op2;
                self.r[rd] = result;
                if s {
                    self.r.cpsr_set_zero_flag(result == 0);
                    self.r.cpsr_set_sign_flag((result >> 31) == 1);
                }
            }
            // MOV
            0xD => {
                trace!("MOV r{} = {:X}", rd, op2);
                self.r[rd] = op2;
                if s {
                    self.r.cpsr_set_zero_flag(op2 == 0);
                    self.r.cpsr_set_sign_flag((op2 >> 31) == 1);
                }
            }
            // BIC
            0xE => {
                trace!("BIC r{} = #{:X} & !#{:X}", rd, op1, op2);
                let result = op1 & !op2;
                self.r[rd] = result;
                if s {
                    self.r.cpsr_set_zero_flag(result == 0);
                    self.r.cpsr_set_sign_flag((result >> 31) == 1);
                }
            }
            // MVN
            0xF => {
                trace!("MVN r{} = !{:X}", rd, op2);
                let result = !op2;
                self.r[rd] = result;
                if s {
                    self.r.cpsr_set_zero_flag(result == 0);
                    self.r.cpsr_set_sign_flag((result >> 31) == 1);
                }
            }
            _ => unimplemented!("ALU instruction 0x{:X} at 0x{:X}", sub_opcode, self.r.pc),
        }

        // HACK: don't set CPSR if in user mode
        // TODO: make sure this doesn't break CMP, etc
        if s && rd == 0xF {
            if self.r.register_mode() != Some(RegisterMode::User) {
                self.r.cpsr = self.r.get_spsr();
            } else {
                panic!("should be reenabling thumb mode...");
                self.r.cpsr = old_cpsr;
            }
        }

        // TODO: writing to PC affects things
        // TODO: timing
        //  "Execution Time: (1+p)S+rI+pN. Whereas r=1 if I=0 and R=1 (ie. shift by register); otherwise r=0. And p=1 if Rd=R15; otherwise p=0.""

        4
    }

    pub fn dispatch_multiply(&mut self, opcode: u32) -> u8 {
        let sub_opcode = (opcode >> 21) & 0xF;
        let s = (opcode >> 20) & 1 == 1;
        let rd = ((opcode >> 16) & 0xF) as u8;
        let rn = ((opcode >> 12) & 0xF) as u8;
        let rs = ((opcode >> 8) & 0xF) as u8;
        let rm = (opcode & 0xF) as u8;
        // is this ARM9 only? maybe we don't need it
        let half_word_multiply = (opcode >> 4) & 1 == 1;

        match sub_opcode {
            0b0000 => {
                trace!("MUL r{} = r{} * r{}", rd, rm, rs);
                let result = self.r[rm].wrapping_mul(self.r[rs]);
                self.r[rd] = result as u32;
                if s {
                    self.r.cpsr_set_zero_flag(result == 0);
                    self.r.cpsr_set_sign_flag((result >> 31) == 1);
                }
            }
            0b0001 => {
                trace!("MLA r{} = r{} * r{} + r{}", rd, rm, rs, rn);
                let result = self.r[rm].wrapping_mul(self.r[rs]).wrapping_add(self.r[rn]);
                self.r[rd] = result as u32;
                if s {
                    self.r.cpsr_set_zero_flag(result == 0);
                    self.r.cpsr_set_sign_flag((result >> 31) == 1);
                }
            }
            0b0010 => {
                todo!("UMAAL")
            }
            0b0100 => {
                trace!("UMULL r{},r{} = r{} * r{}", rd, rn, rs, rm);
                let result = (self.r[rs] as u64).wrapping_mul(self.r[rm] as u64);
                self.r[rd] = (result >> 32) as u32;
                self.r[rn] = (result & 0xFFFF_FFFF) as u32;
                if s {
                    self.r.cpsr_set_zero_flag(result == 0);
                    self.r.cpsr_set_sign_flag((result >> 63) == 1);
                }
            }
            0b0101 => {
                trace!(
                    "UMLAL r{},r{} = r{} * r{} + r{},r{}",
                    rd,
                    rn,
                    rs,
                    rm,
                    rd,
                    rn
                );
                let add_val = ((self.r[rd] as u64) << 32) | (self.r[rn] as u64);
                let result =
                    ((self.r[rs] as u64).wrapping_mul(self.r[rm] as u64)).wrapping_add(add_val);
                self.r[rd] = (result >> 32) as u32;
                self.r[rn] = (result & 0xFFFF_FFFF) as u32;
                if s {
                    self.r.cpsr_set_zero_flag(result == 0);
                    self.r.cpsr_set_sign_flag((result >> 63) == 1);
                }
            }
            0b0110 => {
                trace!("SMULL r{},r{} = r{} * r{}", rd, rn, rs, rm);
                let result = (self.r[rs] as i32 as i64).wrapping_mul(self.r[rm] as i32 as i64);
                self.r[rd] = (result >> 32) as u32;
                self.r[rn] = (result & 0xFFFF_FFFF) as u32;
                if s {
                    self.r.cpsr_set_zero_flag(result == 0);
                    self.r.cpsr_set_sign_flag((result >> 63) & 1 == 1);
                }
            }
            0b0111 => {
                trace!(
                    "SMLAL r{},r{} = r{} * r{} + r{},r{}",
                    rd,
                    rn,
                    rs,
                    rm,
                    rd,
                    rn
                );
                let add_val = ((self.r[rd] as u64) << 32) | (self.r[rn] as u64);
                let mul_result = (self.r[rs] as i32 as i64).wrapping_mul(self.r[rm] as i32 as i64);
                let result = (mul_result as u64).wrapping_add(add_val);
                self.r[rd] = (result >> 32) as u32;
                self.r[rn] = (result & 0xFFFF_FFFF) as u32;
                if s {
                    self.r.cpsr_set_zero_flag(result == 0);
                    self.r.cpsr_set_sign_flag((result >> 63) == 1);
                }
            }
            0b1000 => {
                todo!("SMLAxy");
            }
            0b1001 => {
                todo!("docs unclear, this could be either multiply, check more docs");
            }
            0b1010 => {
                todo!("SMLALxy");
            }
            0b1011 => {
                todo!("SMULxy");
            }
            _ => unreachable!(
                "multiply instruction 0x{:X} at 0x{:X}",
                sub_opcode, self.r.pc
            ),
        }
        // for armv4 we always clear this
        self.r.cpsr_set_carry_flag(false);
        //self.r.cpsr_set_overflow_flag(false);

        3
    }

    pub fn dispatch_swap(&mut self, opcode: u32) -> u8 {
        let byte = (opcode >> 22) & 1 == 1;
        let rn = ((opcode >> 16) & 0xF) as u8;
        let rd = ((opcode >> 12) & 0xF) as u8;
        let rm = (opcode & 0xF) as u8;
        debug_assert_ne!(rn, 0xF);
        debug_assert_ne!(rd, 0xF);
        debug_assert_ne!(rm, 0xF);

        trace!("SWP r{} = r{} <-> r{}", rd, rm, rn);

        let addr = self.r[rn];
        let rm_val = self.r[rm];
        let mut cycles = 0;
        if byte {
            let o = self.get_mem8(addr);
            cycles += o.1;
            self.r[rd] = o.0 as u32;
            cycles += self.set_mem8(addr, rm_val as u8);
        } else {
            let o = self.get_mem32(addr);
            cycles += o.1;
            self.r[rd] = o.0;
            cycles += self.set_mem32(addr, rm_val);
        }

        cycles + 1
    }

    pub fn dispatch_psr(&mut self, opcode: u32) -> u8 {
        let psr_src_dest = (opcode >> 22) & 1 == 1;
        let psr_subopcode = (opcode >> 21) & 1 == 1;
        let imm = psr_subopcode && (opcode >> 25) & 1 == 1;

        if psr_subopcode {
            let write_flags = (opcode >> 19) & 1 == 1;
            let write_status = (opcode >> 18) & 1 == 1;
            let write_extension = (opcode >> 17) & 1 == 1;
            let write_control = (opcode >> 16) & 1 == 1;
            let mask = {
                let mut mask = 0;
                if write_flags {
                    mask |= 0xFF << 24;
                }
                if write_status {
                    mask |= 0xFF << 16;
                }
                if write_extension {
                    mask |= 0xFF << 8;
                }
                if write_control {
                    mask |= 0xFF;
                }
                mask
            };
            let val = if imm {
                let shift_amt = (opcode >> 8) & 0xF;
                (opcode & 0xFF).rotate_right(shift_amt * 2)
            } else {
                let rm = ((opcode >> 0) & 0xF) as u8;
                debug_assert_ne!(rm, 0xF, "MSR from PC");
                self.r[rm as u8]
            };
            trace!(
                "MSR mode={:?}: 0x{:X} & 0x{:X}",
                self.r.register_mode(),
                mask,
                val
            );
            // HACK: if in user mode, just force CPSR
            if psr_src_dest
            /*&& self.r.register_mode() != Some(RegisterMode::User)*/
            {
                debug_assert_ne!(self.r.register_mode(), Some(RegisterMode::User));
                *self.r.get_spsr_mut() &= !mask;
                *self.r.get_spsr_mut() |= val & mask;
            } else {
                self.r.cpsr &= !mask;
                self.r.cpsr |= val & mask;
            }
        } else {
            trace!("MRS mode={:?}", self.r.register_mode());
            let rd = ((opcode >> 12) & 0xF) as u8;
            debug_assert_ne!(rd, 0xF, "MRS to PC");
            debug_assert!(!imm);

            let v = if psr_src_dest
            /*&& self.r.register_mode() != Some(RegisterMode::User)*/
            {
                debug_assert_ne!(self.r.register_mode(), Some(RegisterMode::User));
                self.r.get_spsr()
            } else {
                self.r.cpsr
            };

            self.r[rd] = v;
        }
        1
    }

    /*

    Opcode Format

      Bit    Expl.
      31-28  Condition
      27-25  Must be 000b for this instruction
      24     P - Pre/Post (0=post; add offset after transfer, 1=pre; before trans.)
      23     U - Up/Down Bit (0=down; subtract offset from base, 1=up; add to base)
      22     I - Immediate Offset Flag (0=Register Offset, 1=Immediate Offset)
      When above Bit 24 P=0 (Post-indexing, write-back is ALWAYS enabled):
        21     Not used, must be zero (0)
      When above Bit 24 P=1 (Pre-indexing, write-back is optional):
        21     W - Write-back bit (0=no write-back, 1=write address into base)
      20     L - Load/Store bit (0=Store to memory, 1=Load from memory)
      19-16  Rn - Base register                (R0-R15) (Including R15=PC+8)
      15-12  Rd - Source/Destination Register  (R0-R15) (Including R15=PC+12)
      11-8   When above Bit 22 I=0 (Register as Offset):
               Not used. Must be 0000b
             When above Bit 22 I=1 (immediate as Offset):
               Immediate Offset (upper 4bits)
      7      Reserved, must be set (1)
      6-5    Opcode (0-3)
             When Bit 20 L=0 (Store) (and Doubleword Load/Store):
              0: Reserved for SWP instruction
              1: STR{cond}H  Rd,<Address>  ;Store halfword   [a]=Rd
              2: LDR{cond}D  Rd,<Address>  ;Load Doubleword  R(d)=[a], R(d+1)=[a+4]
              3: STR{cond}D  Rd,<Address>  ;Store Doubleword [a]=R(d), [a+4]=R(d+1)
             When Bit 20 L=1 (Load):
              0: Reserved.
              1: LDR{cond}H  Rd,<Address>  ;Load Unsigned halfword (zero-extended)
              2: LDR{cond}SB Rd,<Address>  ;Load Signed byte (sign extended)
              3: LDR{cond}SH Rd,<Address>  ;Load Signed halfword (sign extended)
      4      Reserved, must be set (1)
      3-0    When above Bit 22 I=0:
               Rm - Offset Register            (R0-R14) (not including R15)
             When above Bit 22 I=1:
               Immediate Offset (lower 4bits)  (0-255, together with upper bits)
         */
    pub fn dispatch_data_trans(&mut self, opcode: u32) -> u8 {
        let p = (opcode >> 24) & 1 == 1;
        let up = (opcode >> 23) & 1 == 1;
        let imm = (opcode >> 22) & 1 == 1;
        let write_back = !p || ((opcode >> 21) & 1 == 1);
        let load = (opcode >> 20) & 1 == 1;
        let rn = ((opcode >> 16) & 0xF) as u8;
        let rd = ((opcode >> 12) & 0xF) as u8;
        let mut base_val = self.r[rn];
        if rn == 15 {
            base_val += 4;
        }
        let mut src_val = self.r[rd];
        if rd == 15 {
            src_val += 8;
        }
        let offset = if imm {
            (((opcode >> 8) & 0xF) << 4) | (opcode & 0xF)
        } else {
            let rm = (opcode & 0xF) as u8;
            debug_assert_ne!(rm, 0xF, "Data transfer from PC");
            self.r[(opcode & 0xF) as u8]
        };
        let opcode = (opcode >> 5) & 0x3;

        if p {
            if up {
                base_val += offset;
            } else {
                base_val -= offset;
            }
        }
        let cycles;

        match (load, opcode) {
            (true, 0b00) => todo!("reserved"),
            (true, 0b01) => {
                trace!("LDRH r{} = [r{} + 0x{:X}]", rd, rn, offset);
                let o = self.get_mem16(base_val);
                self.r[rd] = o.0 as u32;
                cycles = 2 + o.1;
            }
            (true, 0b10) => {
                trace!("LDRSB r{} = [r{} + 0x{:X}]", rd, rn, offset);
                let o = self.get_mem8(base_val);
                self.r[rd] = (o.0 as i8) as i32 as u32;
                cycles = 2 + o.1;
            }
            (true, 0b11) => {
                trace!("LDRSH r{} = [r{} + 0x{:X}]", rd, rn, offset);
                let o = self.get_mem16(base_val);
                self.r[rd] = (o.0 as i16) as i32 as u32;
                cycles = 2 + o.1;
            }
            (false, 0b00) => todo!("SWP"),
            (false, 0b01) => {
                trace!("STRH r{} = [r{} + 0x{:X}]", rd, rn, offset);
                let o = self.set_mem16(base_val, src_val as u16);
                cycles = 2 + o;
            }
            (false, 0b10) => {
                trace!("LDRD r{} = [r{} + 0x{:X}]", rd, rn, offset);
                let o1 = self.get_mem32(base_val);
                let o2 = self.get_mem32(base_val + 4);
                self.r[rd] = o1.0;
                self.r[rd + 1] = o2.0;
                cycles = 2 + o1.1 + o2.1;
            }
            (false, 0b11) => {
                trace!("STRD r{} = [r{} + 0x{:X}]", rd, rn, offset);
                let o1 = self.set_mem32(base_val, src_val);
                let mut src_val2 = self.r[rd + 1];
                if rd + 1 == 15 {
                    src_val2 += 8;
                }
                let o2 = self.set_mem32(base_val + 4, src_val2);
                cycles = 2 + o1 + o2;
            }
            _ => unreachable!(),
        }
        if !p {
            if up {
                base_val = base_val.wrapping_add(offset);
            } else {
                base_val = base_val.wrapping_sub(offset);
            }
        }
        if write_back {
            self.r[rd] = base_val;
            // TODO: do we need to write back twice for double words? probably
        }
        cycles
    }

    // ARM Opcodes: Memory: Single Data Transfer (LDR, STR, PLD)
    pub fn dispatch_mem(&mut self, opcode: u32) -> u8 {
        let mut cycles = 0;
        let imm = (opcode >> 25) & 1 == 1;
        let p = (opcode >> 24) & 1 == 1;
        let up = (opcode >> 23) & 1 == 1;
        let byte = (opcode >> 22) & 1 == 1;
        // only when P is false
        let force_nonpriviliged = (opcode >> 21) & 1 == 1;
        let write_back = !p || ((opcode >> 21) & 1 == 1);
        let load = (opcode >> 20) & 1 == 1;
        let base_reg = (opcode >> 16) & 0xF;
        let src_dest_reg = (opcode >> 12) & 0xF;
        if force_nonpriviliged && !p {
            todo!("figure this out");
        }

        let offset = if imm {
            let shift_type = (opcode >> 5) & 3;
            let rm = (opcode & 0xF) as u8;
            let shift_amount = (opcode >> 7) & 0x1F;
            if shift_amount == 0 {
                match shift_type {
                    // LSL
                    0b00 => self.r[rm],
                    // LSR
                    0b01 => 0,
                    // ASR
                    0b10 => ((self.r[rm] as i32) >> 31) as u32,
                    // ROR
                    0b11 => todo!("ROR!"),
                    _ => unreachable!(),
                }
            } else {
                match shift_type {
                    0b00 => self.r[rm] << shift_amount,
                    0b01 => self.r[rm] >> shift_amount,
                    0b10 => ((self.r[rm] as i32) >> shift_amount) as u32,
                    0b11 => self.r[rm].rotate_right(shift_amount),
                    _ => unreachable!(),
                }
            }
        } else {
            opcode & 0xFFF
        };

        let mut val = self.r[base_reg as u8];
        if base_reg == 15 {
            val += 4;
        }
        if p {
            if up {
                val = val.wrapping_add(offset);
            } else {
                val = val.wrapping_sub(offset);
            }
        }

        if load {
            if byte {
                trace!("LDRB r{} = mem[{:X}]", src_dest_reg, val);
                let o = self.get_mem8(val);
                cycles += o.1;
                self.r[src_dest_reg as u8] = o.0 as u32;
            } else {
                let o = self.get_mem32(val);
                trace!(
                    "LDR r{} = mem[r{} (0x{:X})] (0x{:X})",
                    src_dest_reg,
                    base_reg,
                    val,
                    o.0
                );
                cycles += o.1;
                self.r[src_dest_reg as u8] = o.0;
            }
        } else {
            let write_val = if src_dest_reg == 15 {
                self.r[src_dest_reg as u8] + 8
            } else {
                self.r[src_dest_reg as u8]
            };
            if byte {
                trace!(
                    "STRB mem[{:X}] = r{} (0x{:X})",
                    val,
                    src_dest_reg,
                    write_val as u8
                );
                cycles += self.set_mem8(val, write_val as u8);
            } else {
                trace!("STR mem[{:X}] = r{} (0x{:X})", val, src_dest_reg, write_val);
                cycles += self.set_mem32(val, write_val);
            }
        }
        if !p {
            if up {
                val = val.wrapping_add(offset);
            } else {
                val = val.wrapping_sub(offset);
            }
        }

        if write_back || !p {
            self.r[base_reg as u8] = val;
        }
        /*
        if !p && write_back {
            //warn!("Write back bit has special meaning in post-inc mode, figure this out");
            todo!("Write back bit has special meaning in post-inc mode, figure this out");
        }
        */

        cycles
    }

    pub fn dispatch_block_data(&mut self, opcode: u32) -> u8 {
        let mut p = (opcode >> 24) & 1 == 1;
        let up = (opcode >> 23) & 1 == 1;
        let force_user_mode = (opcode >> 22) & 1 == 1;
        let mut write_back = (opcode >> 21) & 1 == 1;
        let load = (opcode >> 20) & 1 == 1;
        let rn = ((opcode >> 16) & 0xF) as u8;
        let r_list = opcode & 0xFFFF;
        let mut cycles = 0;

        let mut base = self.r[rn];

        if force_user_mode {
            //todo!("Figure out force user mode");
            warn!("Figure out force user mode");
        }

        let debug_type = match (load, p, up) {
            (true, true, true) => "LDMED",
            (true, false, true) => "LDMFD",
            (true, true, false) => "LDMEA",
            (true, false, false) => "LDMFA",
            (false, false, false) => "STMED",
            (false, true, false) => "STMFD",
            (false, false, true) => "STMEA",
            (false, true, true) => "STMFA",
        };

        let r_list_count = r_list.count_ones();

        if r_list_count == 0 {
            todo!("0 is a special case, handle it when it happens");
        } else if !up {
            base = base.wrapping_sub(4 * r_list_count);
            if write_back {
                self.r[rn] = base;
                write_back = false;
            }
            p = !p;
        }

        // LDM, STM
        trace!(
            "{} r{} (0x{:X}) - {:016b}: in {:?}",
            debug_type,
            rn,
            base,
            r_list,
            self.r.register_mode()
        );
        // TOOD: all accesses should be done lower to higher
        if load {
            for i in 0..16 {
                if r_list & (1 << i) == 0 {
                    continue;
                }
                if i == rn {
                    write_back = false;
                }
                if p {
                    base = base.wrapping_add(4);
                }
                let o = self.get_mem32(base);
                //println!("Loading r{} (0x{:X}) from 0x{:X}", i, o.0, base);
                self.r[i as u8] = o.0;
                cycles += o.1 + 2;
                if !p {
                    base = base.wrapping_add(4);
                }
            }
        } else {
            for i in 0..16 {
                if r_list & (1 << i) == 0 {
                    continue;
                }
                if p {
                    base = base.wrapping_add(4);
                }
                //println!("pushing r{} ({:X}) to 0x{:X}", i, self.r[i as u8], base);
                cycles += self.set_mem32(base, self.r[i as u8]);
                cycles += 1;
                if !p {
                    base = base.wrapping_add(4);
                }
            }
        }
        if write_back {
            self.r[rn] = base;
        }
        cycles
    }

    pub fn dispatch_branch_and_exchange(&mut self, opcode: u32) -> u8 {
        let subopcode = (opcode >> 4) & 0xF;
        let op_reg = opcode & 0xF;
        debug_assert!(op_reg < 15);
        let mut op = self.r[op_reg as u8];
        if op_reg == 15 {
            op += 4;
        }
        let thumb_mode = op & 1 == 1;
        let old_pc = self.r.pc;
        let new_pc = if thumb_mode { op & !1 } else { op & !3 };
        let old_thumb_enabled = self.r.thumb_enabled();

        match subopcode {
            0b0001 => {
                // BX
                trace!("BX to r{} (0x{:X})", op_reg, new_pc);
                self.r.pc = new_pc;
                self.r.set_thumb(thumb_mode);
            }
            0b0010 => {
                panic!("Change to Jazelle mode not implemented");
            }
            0b0011 => {
                // BLX
                trace!("BLX pc = r{} (0x{:X}), lr = 0x{:X}", op_reg, op, self.r.pc);
                self.r.set_thumb(thumb_mode);
                self.r.pc = new_pc;
                //*self.r.lr_mut() = old_pc + 4;
                *self.r.lr_mut() = old_pc;
            }
            _ => panic!("Unknown branch and exchange subopcode 0x{:X}", subopcode),
        }

        if old_thumb_enabled != thumb_mode {
            if thumb_mode {
                trace!("Enabling Thumb mode from 0x{:X} to 0x{:X}!", old_pc, new_pc);
            } else {
                trace!("Enabling ARM mode!");
            }
        }
        // 2S + 1N
        3
    }
}

use crate::io::graphics::renderer::Button;
fn button_to_bit(button: Button) -> u16 {
    match button {
        Button::A => 0x0001,
        Button::B => 0x0002,
        Button::Select => 0x0004,
        Button::Start => 0x0008,
        Button::Right => 0x0010,
        Button::Left => 0x0020,
        Button::Up => 0x0040,
        Button::Down => 0x0080,
        Button::R => 0x0100,
        Button::L => 0x0200,
    }
}

impl crate::io::graphics::renderer::InputReceiver for GameboyAdvance {
    // TODO: 4000132h - KEYCNT - Key Interrupt Control (R/W)
    fn press(&mut self, button: Button) {
        let bit = button_to_bit(button);
        if bit > 0xFF {
            self.io_registers[0x131] &= !((bit >> 8) as u8);
        } else {
            self.io_registers[0x130] &= !(bit as u8);
        }
        let keycnt = self.io_registers.get_mem16(0x4000132);
        if self.keypad_interrupt_enabled() && (keycnt >> 14) & 1 == 1 {
            let buttons = self.io_registers.get_mem16(0x4000130) & 0x3F;
            if (keycnt >> 15) == 1 {
                // AND mode
                if (keycnt & 0x3F) & buttons == (keycnt & 0x3F) {
                    self.keypad_interrupt_requested();
                }
            } else {
                // OR mode
                if (keycnt & 0x3F) & buttons != 0 {
                    self.keypad_interrupt_requested();
                }
            }
        }
    }
    fn unpress(&mut self, button: Button) {
        let bit = button_to_bit(button);
        if bit > 0xFF {
            self.io_registers[0x131] |= (bit >> 8) as u8;
        } else {
            self.io_registers[0x130] |= bit as u8;
        }
    }
    fn reset(&mut self) {
        todo!("reset");
    }
}
