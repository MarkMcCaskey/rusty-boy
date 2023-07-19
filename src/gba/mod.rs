//! Experimental GBA support

pub struct GameboyAdvance {
    r: Registers,
    // TODO: move this later
    entire_rom: Vec<u8>,
    bios: [u8; 0x4000],
    iw_ram: [u8; 0x8000],
    wram: [u8; 0x40000],
    io_registers: IoRegisters,
    pub obj_palette_ram: [u8; 0x400],
    pub vram: [u8; 0x18000],
    pub oam: [u8; 0x400],
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
}

impl IoRegisters {
    pub fn new() -> Self {
        IoRegisters {
            io_registers: [0; 0x400],
            dma0_enabled: false,
            dma1_enabled: false,
            dma2_enabled: false,
            dma3_enabled: false,
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
            0x100..=0x110 => {
                println!("TIMER: {:X} = {:X}", addr, val);
                self.io_registers[addr as usize] = val;
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
            },
            14 => match mode {
                RegisterMode::FIQ => &self.r14_fiq,
                RegisterMode::Supervisor => &self.r14_svc,
                RegisterMode::Abort => &self.r14_abt,
                RegisterMode::IRQ => &self.r14_irq,
                RegisterMode::Undefined => &self.r14_und,
                RegisterMode::User => &self.lr,
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
            },
            14 => match mode {
                RegisterMode::FIQ => &mut self.r14_fiq,
                RegisterMode::Supervisor => &mut self.r14_svc,
                RegisterMode::Abort => &mut self.r14_abt,
                RegisterMode::IRQ => &mut self.r14_irq,
                RegisterMode::Undefined => &mut self.r14_und,
                RegisterMode::User => &mut self.lr,
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
            0b11111 => Some(Self::User),
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
            sp: 0,
            lr: 0,
            pc: 0, //0x08000000,
            r8_fiq: 0,
            r9_fiq: 0,
            r10_fiq: 0,
            r11_fiq: 0,
            r12_fiq: 0,
            r13_fiq: 0,
            r14_fiq: 0,
            r13_svc: 0,
            r14_svc: 0,
            r13_abt: 0,
            r14_abt: 0,
            r13_irq: 0,
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
            RegisterMode::User => unimplemented!("Is this possible?"),
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
            RegisterMode::User => unimplemented!("Is this possible?"),
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
        let bits = ((self.io_registers[0x8] as u16) << 8) | (self.io_registers[0x9] as u16) & 0x1F;
        PpuBgControl::from_bits(bits)
    }
    pub fn ppu_bg1_control(&self) -> PpuBgControl {
        let bits = ((self.io_registers[0xA] as u16) << 8) | (self.io_registers[0xB] as u16) & 0x1F;
        PpuBgControl::from_bits(bits)
    }
    pub fn ppu_bg2_control(&self) -> PpuBgControl {
        let bits = ((self.io_registers[0xC] as u16) << 8) | (self.io_registers[0xD] as u16) & 0x1F;
        PpuBgControl::from_bits(bits)
    }
    pub fn ppu_bg3_control(&self) -> PpuBgControl {
        let bits = ((self.io_registers[0xE] as u16) << 8) | (self.io_registers[0xF] as u16) & 0x1F;
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
        self.io_registers[0x6] = ly;
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
            0x02000000..=0x0203FFFF => {
                // on-board work ram
                (self.wram[(address & 0x3FFFF) as usize], 3)
            }
            //0x03000000..=0x03007FFF => (self.iw_ram[(address & 0x7FFF) as usize], 1),
            0x03000000..=0x03FFFFFF => (self.iw_ram[(address & 0x7FFF) as usize], 1),
            //0x04000000..=0x040003FE => (self.io_registers[(address & 0x3FE) as usize], 1),
            0x04000000..=0x04FFFFFE => (self.io_registers[(address & 0x3FF) as usize], 1),
            //0x05000000..=0x050003FF => (self.obj_palette_ram[(address & 0x3FF) as usize], 1),
            0x05000000..=0x05FFFFFF => (self.obj_palette_ram[(address & 0x3FF) as usize], 1),
            //0x06000000..=0x06017FFF => (self.vram[(address & 0x17FFF) as usize], 1),
            0x06000000..=0x06FFFFFF => (self.vram[(address & 0x17FFF) as usize], 1),
            //0x07000000..=0x070003FF => (self.oam[(address & 0x3FF) as usize], 1),
            0x07000000..=0x07FFFFFF => (self.oam[(address & 0x3FF) as usize], 1),
            0x08000000..=0x09FFFFFF => (self.entire_rom[(address & 0x1FFFFFF) as usize], 5),
            0x0A000000..=0x0BFFFFFF => todo!("Game Pak ROM/FlashROM (max 32MB) - Wait State 1"),
            0x0C000000..=0x0DFFFFFF => todo!("Game Pak ROM/FlashROM (max 32MB) - Wait State 2"),
            0x0E000000..=0x0E00FFFF => todo!("Game Pak SRAM    (max 64 KBytes) - 8bit Bus width"),
            _ => (0, 0),
        }
    }
    pub fn get_mem16(&self, address: u32) -> (u16, u8) {
        let lo_bit_idx = address & !0x1;
        let hi_bit_idx = address | 0x1;
        match address {
            0x00000000..=0x01FF3FFF => {
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
                let lo_bit = self.io_registers[(lo_bit_idx & 0x3FF) as usize] as u16;
                let hi_bit = self.io_registers[(hi_bit_idx & 0x3FF) as usize] as u16;
                ((hi_bit << 8) | lo_bit, 1)
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
            0x0E000000..=0x0E00FFFF => todo!("Game Pak SRAM    (max 64 KBytes) - 8bit Bus width"),
            _ => (0, 0),
        }
    }
    pub fn get_mem32(&self, address: u32) -> (u32, u8) {
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
                let bit1 = self.io_registers[(bit1_idx & 0x3FF) as usize] as u32;
                let bit2 = self.io_registers[(bit2_idx & 0x3FF) as usize] as u32;
                let bit3 = self.io_registers[(bit3_idx & 0x3FF) as usize] as u32;
                let bit4 = self.io_registers[(bit4_idx & 0x3FF) as usize] as u32;
                let out = (bit4 << 24) | (bit3 << 16) | (bit2 << 8) | bit1;
                (out, 1)
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
            0x08000000..=0x09FFFFFF => {
                let bit1 = self.entire_rom[(bit1_idx & 0x1FFFFFF) as usize] as u32;
                let bit2 = self.entire_rom[(bit2_idx & 0x1FFFFFF) as usize] as u32;
                let bit3 = self.entire_rom[(bit3_idx & 0x1FFFFFF) as usize] as u32;
                let bit4 = self.entire_rom[(bit4_idx & 0x1FFFFFF) as usize] as u32;
                let out = (bit4 << 24) | (bit3 << 16) | (bit2 << 8) | bit1;
                (out, 8)
            }
            0x0A000000..=0x0BFFFFFF => todo!("Game Pak ROM/FlashROM (max 32MB) - Wait State 1"),
            0x0C000000..=0x0DFFFFFF => todo!("Game Pak ROM/FlashROM (max 32MB) - Wait State 2"),
            0x0E000000..=0x0E00FFFF => todo!("Game Pak SRAM    (max 64 KBytes) - 8bit Bus width"),
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
            0x06000000..=0x06017FFF => {
                todo!("VRAM")
            }
            0x07000000..=0x070003FF => {
                todo!("OAM")
            }
            0x08000000..=0x09FFFFFF => todo!("Game Pak ROM/FlashROM (max 32MB) - Wait State 0"),
            0x0A000000..=0x0BFFFFFF => todo!("Game Pak ROM/FlashROM (max 32MB) - Wait State 1"),
            0x0C000000..=0x0DFFFFFF => todo!("Game Pak ROM/FlashROM (max 32MB) - Wait State 2"),
            0x0E000000..=0x0E00FFFF => todo!("Game Pak SRAM    (max 64 KBytes) - 8bit Bus width"),
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
            0x08000000..=0x09FFFFFF => todo!("Game Pak ROM/FlashROM (max 32MB) - Wait State 0"),
            0x0A000000..=0x0BFFFFFF => todo!("Game Pak ROM/FlashROM (max 32MB) - Wait State 1"),
            0x0C000000..=0x0DFFFFFF => todo!("Game Pak ROM/FlashROM (max 32MB) - Wait State 2"),
            0x0E000000..=0x0E00FFFF => todo!("Game Pak SRAM    (max 64 KBytes) - 8bit Bus width"),
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
            0x0E000000..=0x0E00FFFF => todo!("Game Pak SRAM    (max 64 KBytes) - 8bit Bus width"),
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
            trace!("VBLANK interrupt started");
            self.irq_interrupt();
        } else if self.lcdc_hblank_interrupt_requested() && self.lcdc_hblank_interrupt_enabled() {
            trace!("HBLANK interrupt started");
            self.irq_interrupt();
        } else if self.lcdc_vcounter_interrupt_requested() && self.lcdc_vcounter_interrupt_enabled()
        {
            trace!("VCOUNTER interrupt started");
            self.irq_interrupt();
        } else if self.timer0_interrupt_requested() && self.timer0_interrupt_enabled() {
            trace!("TIMER0 interrupt started");
            self.irq_interrupt();
        } else if self.timer1_interrupt_requested() && self.timer1_interrupt_enabled() {
            trace!("TIMER1 interrupt started");
            self.irq_interrupt();
        } else if self.timer2_interrupt_requested() && self.timer2_interrupt_enabled() {
            trace!("TIMER2 interrupt started");
            self.irq_interrupt();
        } else if self.timer3_interrupt_requested() && self.timer3_interrupt_enabled() {
            trace!("TIMER3 interrupt started");
            self.irq_interrupt();
        } else if self.serial_interrupt_requested() && self.serial_interrupt_enabled() {
            trace!("SERIAL interrupt started");
            self.irq_interrupt();
        } else if self.dma0_interrupt_requested() && self.dma0_interrupt_enabled() {
            trace!("DMA0 interrupt started");
            self.irq_interrupt();
        } else if self.dma1_interrupt_requested() && self.dma1_interrupt_enabled() {
            trace!("DMA1 interrupt started");
            self.irq_interrupt();
        } else if self.dma2_interrupt_requested() && self.dma2_interrupt_enabled() {
            trace!("DMA2 interrupt started");
            self.irq_interrupt();
        } else if self.dma3_interrupt_requested() && self.dma3_interrupt_enabled() {
            trace!("DMA3 interrupt started");
            self.irq_interrupt();
        } else if self.keypad_interrupt_requested() && self.keypad_interrupt_enabled() {
            trace!("KEYPAD interrupt started");
            self.irq_interrupt();
        } else if self.game_pak_interrupt_requested() && self.game_pak_interrupt_enabled() {
            trace!("GAMEPAK interrupt started");
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
        // TODO: some instructions can't be skipped, handle those
        if opcode == 0 {
            self.r.pc += 4;
            //self.r.pc = self.r.pc.wrapping_add(4);
            return 4;
        }
        //println!("opcode: {:032b} at 0x{:X}", opcode, self.r.pc);
        let cond = Cond::from_u8(((opcode >> 28) & 0xF) as u8);
        if !self.cond_should_execute(cond) {
            //println!("Skipped!");
            self.r.pc += 4;
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
            // TODO: add 10 to end of above and PSR
            // TODO: add 000 to end of above and multiply
            // TODO: add 01 for mul long
            //  |_Cond__|0_0_0_0_1|U|A|S|_RdHi__|_RdLo__|__Rs___|1_0_0_1|__Rm___| MulLong
            0b001 | 0b000 => self.dispatch_alu(opcode),
            0b010 | 0b011 => self.dispatch_mem(opcode),
            0b100 => self.dispatch_block_data(opcode),
            // TODO: 0b100 block trans
            // TODO: 0b110 co data trans
            0b111 => self.dispatch_codata_op(opcode),
            /*
            |_Cond__|1_1_1_0|_CPopc_|__CRn__|__CRd__|__CP#__|_CP__|0|__CRm__| CoDataOp
            |_Cond__|1_1_1_0|CPopc|L|__CRn__|__Rd___|__CP#__|_CP__|1|__CRm__| CoRegTrans
            |_Cond__|1_1_1_1|_____________Ignored_by_Processor______________| SWI
                       */
            _ => {
                unimplemented!("0x{:X} ({:b}) at 0x{:X}", opcode, opcode_idx, self.r.pc);
            }
        };

        self.r.pc += 4;

        cycles as u32
    }

    pub fn dispatch_thumb(&mut self) -> u8 {
        let opcode = self.get_thumb_opcode();
        let opcode_idx = (opcode >> 13) & 0x7;
        if opcode == 0 {
            self.r.pc += 2;
            return 4;
        }
        //println!("THUMB opcode: {:016b} at 0x{:X}", opcode, self.r.pc);

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
                        todo!("THUMB SWI");
                        //self.dispatch_thumb_software_interrupt(opcode)
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

        self.r.pc += 2;

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
                let result = self.r[rd] << (self.r[rs] & 0xFF);
                self.r[rd] = result;
                if self.r[rs] & 0xFF != 0 {
                    // TODO: REVIEW
                    self.r
                        .cpsr_set_carry_flag(self.r[rs] & (1 << (32 - (self.r[rs] & 0xFF))) != 0);
                }
                2
            }
            0b0011 => {
                trace!("LSR r{} = r{} >> r{}", rd, rs, rd);
                self.r[rd] = self.r[rd] >> (self.r[rs] & 0xFF);
                if self.r[rs] & 0xFF != 0 {
                    // TODO: REVIEW
                    self.r
                        .cpsr_set_carry_flag(self.r[rs] & (1 << ((self.r[rs] & 0xFF) - 1)) != 0);
                }
                2
            }
            0b0100 => {
                trace!("ASR r{} = r{} >> r{}", rd, rs, rd);
                self.r[rd] = ((self.r[rd] as i32) >> (self.r[rs] & 0xFF)) as u32;
                if self.r[rs] & 0xFF != 0 {
                    // TODO: REVIEW
                    self.r
                        .cpsr_set_carry_flag(self.r[rs] & (1 << ((self.r[rs] & 0xFF) - 1)) != 0);
                }
                2
            }
            0b0101 => {
                trace!("ADC r{} = r{} + r{} + C", rd, rs, rd);
                let overflow_check = self.r[rd]
                    .checked_add(self.r[rs])
                    .and_then(|v| v.checked_add(self.r.cpsr_carry_flag() as u32));
                let old_val = self.r[rd];
                self.r[rd] = self.r[rs]
                    .wrapping_add(self.r[rd])
                    .wrapping_add(self.r.cpsr_carry_flag() as u32);
                self.r.cpsr_set_carry_flag(overflow_check.is_none());
                // TODO: what does overflow mean here?
                // TODO: include carry in overflow check
                self.r.cpsr_set_overflow_flag(
                    ((old_val ^ self.r[rs]) & 0x8000_0000 == 0)
                        && ((old_val ^ self.r[rd]) & 0x8000_0000 != 0),
                );
                1
            }
            0b0110 => {
                trace!("SBC r{} = r{} - r{} - C", rd, rs, rd);
                // TODO: review what not carry means, does it just mean add the carry? or is it 32bit negation?
                self.r[rd] = self.r[rd]
                    .wrapping_sub(self.r[rs])
                    .wrapping_sub(self.r.cpsr_carry_flag() as u32);
                // TODO: review this
                self.r.cpsr_set_overflow_flag(false);
                self.r.cpsr_set_carry_flag(false);
                1
            }
            0b0111 => {
                trace!("ROR r{} = r{} ROR r{}", rd, rs, rd);
                self.r[rd] = self.r[rd].rotate_right(self.r[rs] & 0xFF);
                if self.r[rs] & 0xFF != 0 {
                    todo!("Carry flag for THUMB ROR");
                    //self.r.cpsr_set_carry_flag()
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
                let result = self.r[rd].wrapping_sub(self.r[rs]);
                self.r.cpsr_set_zero_flag(result == 0);
                self.r.cpsr_set_sign_flag(result & 0x8000_0000 != 0);
                // TODO: review this
                self.r.cpsr_set_overflow_flag(false);
                self.r.cpsr_set_carry_flag(false);
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
                trace!("ORR r{} = r{} | r{}", rd, rs, rd);
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
            self.r.cpsr_set_sign_flag(self.r[rd] & 0x8000_0000 != 0);
        }
        cycles
    }

    pub fn dispatch_thumb_load_pc_relative(&mut self, opcode: u16) -> u8 {
        let rd = ((opcode >> 8) & 0x7) as u8;
        let nn = (opcode & 0xFF) << 2;
        let pc = (self.r.pc + 4) & !2;
        trace!("LDR r{}, [PC, #{}]", rd, nn);

        let o = self.get_mem32(pc + nn as u32);
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
        let cycles = match subop {
            0b00 => {
                trace!("ADD r{}, r{}", rd, rs);
                self.r[rd] = self.r[rd].wrapping_add(self.r[rs]);
                1
            }
            0b01 => {
                trace!("CMP r{}, r{}", rd, rs);
                let result = self.r[rd].wrapping_sub(self.r[rs]);
                self.r.cpsr_set_zero_flag(result == 0);
                self.r.cpsr_set_sign_flag(result & 0x8000_0000 != 0);
                // TODO: review this
                self.r.cpsr_set_overflow_flag(false);
                self.r.cpsr_set_carry_flag(false);
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
                if self.r.thumb_enabled() != thumb_mode {
                    if thumb_mode {
                        info!("Enabling Thumb mode");
                    } else {
                        info!("Enabling ARM mode!");
                    }
                }
                self.r.set_thumb(thumb_mode);
                if x_flag {
                    trace!("BLX r{}", rs);
                    let old_pc = self.r.pc;
                    self.r.pc = (self.r[rs] + 4) & !2;
                    *self.r.lr_mut() = old_pc + 3;
                } else {
                    trace!("BX r{}", rs);
                    //self.r.pc = (self.r[rs] + 4) & !2;
                    self.r.pc = (self.r[rs] + 2) & !1;
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
                    cycles += self.set_mem32(self.r[rb], self.r[i as u8]);
                    self.r[rb] = self.r[rb].wrapping_add(4);
                }
            }
        }
        cycles
    }

    pub fn dispatch_thumb_load_store_sp_relative(&mut self, opcode: u16) -> u8 {
        let subop = (opcode >> 11) & 0x1 == 1;
        let rd = (opcode >> 8 & 0x7) as u8;
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
            self.r[rd] = ((self.r.pc + 4) & !2) + (nn * 4);
        }

        1
    }

    pub fn dispatch_thumb_push_pop(&mut self, opcode: u16) -> u8 {
        let subop = (opcode >> 11) & 0x1 == 1;
        let pc_lr = (opcode >> 8) & 0x1 == 1;
        let r_list = opcode & 0xFF;
        let mut cycles = 0;

        if subop {
            trace!("POP");
            if pc_lr {
                let o = self.get_mem32(self.r.sp());
                *self.r.sp_mut() += 4;
                self.r.pc = o.0 & !1;
                cycles += o.1;
                cycles += 2;
            }
            // is reverse correct? do we need to do it elsewhere?
            for i in (0..8).rev() {
                if r_list & (1 << i) != 0 {
                    let o = self.get_mem32(self.r.sp());
                    *self.r.sp_mut() += 4;
                    self.r[i as u8] = o.0;
                    cycles += o.1;
                    cycles += 2;
                }
            }
            // 0 1 2 3 4
            // 4 3 2 1 0
        } else {
            trace!("PUSH");
            for i in 0..8 {
                if r_list & (1 << i) != 0 {
                    cycles += 1;
                    cycles += self.set_mem32(self.r.sp(), self.r[i as u8]);
                    *self.r.sp_mut() -= 4;
                }
            }
            if pc_lr {
                cycles += 1;
                cycles += self.set_mem32(self.r.sp(), self.r.lr());
                *self.r.sp_mut() -= 4;
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
        let old_val = self.r[rd];

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
                // TODO: review this
                self.r.cpsr_set_overflow_flag(false);
                self.r.cpsr_set_carry_flag(false);
            }
            // ADD
            0b10 => {
                trace!("ADD r{}, #{}", rd, imm);
                self.r[rd] = self.r[rd].wrapping_add(imm);
                self.r.cpsr_set_zero_flag(self.r[rd] == 0);
                self.r.cpsr_set_sign_flag(self.r[rd] & 0x8000_0000 != 0);
                self.r.cpsr_set_overflow_flag(
                    ((old_val ^ imm) & 0x8000_0000 == 0)
                        && ((old_val ^ self.r[rd]) & 0x8000_0000 != 0),
                );
                self.r
                    .cpsr_set_carry_flag(old_val.checked_add(imm).is_none());
            }
            // SUB
            0b11 => {
                trace!("SUB r{}, #{}", rd, imm);
                self.r[rd] = self.r[rd].wrapping_sub(imm);
                self.r.cpsr_set_zero_flag(self.r[rd] == 0);
                self.r.cpsr_set_sign_flag(self.r[rd] & 0x8000_0000 != 0);
                // TODO: review this
                self.r.cpsr_set_overflow_flag(false);
                self.r.cpsr_set_carry_flag(false);
            }
            _ => unreachable!(),
        }

        1
    }

    pub fn dispatch_thumb_shift_add_sub(&mut self, opcode: u16) -> u8 {
        let sub_op_idx = (opcode >> 11) & 0x3;
        let offset = opcode >> 6 & 0x1F;
        let rs = (opcode >> 3 & 0x7) as u8;
        let rd = (opcode & 0x7) as u8;
        match sub_op_idx {
            // LSL
            0b00 => {
                trace!("LSL r{}, r{}, #{}", rd, rs, offset);
                self.r[rd] = self.r[rs] << offset;
                // shift of 0 = don't modify carry flag
                if offset != 0 {
                    // TODO: review this
                    self.r
                        .cpsr_set_carry_flag(self.r[rs] & (1 << (32 - offset)) != 0);
                }
                self.r.cpsr_set_sign_flag(self.r[rd] & 0x8000_0000 != 0);
                self.r.cpsr_set_zero_flag(self.r[rd] == 0);
            }
            0b01 => {
                trace!("LSR r{}, r{}, #{}", rd, rs, offset);
                self.r[rd] = self.r[rs] >> offset;
                // shift of 0 = don't modify carry flag
                if offset != 0 {
                    // TODO: review this
                    self.r
                        .cpsr_set_carry_flag(self.r[rs] & (1 << (offset - 1)) != 0);
                }
                self.r.cpsr_set_sign_flag(self.r[rd] & 0x8000_0000 != 0);
                self.r.cpsr_set_zero_flag(self.r[rd] == 0);
            }
            0b10 => {
                trace!("ASR r{}, r{}, #{}", rd, rs, offset);
                self.r[rd] = ((self.r[rs] as i32) >> offset) as u32;
                // shift of 0 = don't modify carry flag
                if offset != 0 {
                    // TODO: review this
                    self.r
                        .cpsr_set_carry_flag(self.r[rs] & (1 << (offset - 1)) != 0);
                }
                self.r.cpsr_set_sign_flag(self.r[rd] & 0x8000_0000 != 0);
                self.r.cpsr_set_zero_flag(self.r[rd] == 0);
            }
            0b11 => {
                let sub_sub_op_idx = (opcode >> 9) & 0x3;
                let reg_or_imm = ((opcode >> 6) & 0x7) as u32;
                let result;
                let op2;
                let old_value = self.r[rd];
                match sub_sub_op_idx {
                    0b00 => {
                        trace!("ADD r{}, r{}, r{}", rd, rs, reg_or_imm);
                        op2 = self.r[reg_or_imm as u8];
                        result = self.r[rs].wrapping_add(op2);
                        self.r[rd] = result;
                        self.r
                            .cpsr_set_carry_flag(self.r[rs].checked_add(op2).is_none());
                    }
                    0b01 => {
                        trace!("SUB r{}, r{}, r{}", rd, rs, reg_or_imm);
                        op2 = self.r[reg_or_imm as u8];
                        result = self.r[rs].wrapping_sub(op2);
                        self.r[rd] = result;
                        // REVIEW:
                        self.r.cpsr_set_carry_flag(false);
                    }
                    0b10 => {
                        trace!("ADD r{}, r{}, #{}", rd, rs, reg_or_imm);
                        op2 = reg_or_imm;
                        result = self.r[rs].wrapping_add(op2);
                        self.r[rd] = result;
                        self.r
                            .cpsr_set_carry_flag(self.r[rs].checked_add(op2).is_none());
                    }
                    0b11 => {
                        trace!("SUB r{}, r{}, #{}", rd, rs, reg_or_imm);
                        op2 = reg_or_imm;
                        result = self.r[rs].wrapping_sub(op2);
                        self.r[rd] = result;
                        // REVIEW:
                        self.r.cpsr_set_carry_flag(false);
                    }
                    _ => unreachable!(),
                }

                self.r.cpsr_set_zero_flag(self.r[rd] == 0);
                self.r.cpsr_set_sign_flag(self.r[rd] & 0x8000_0000 != 0);
                self.r.cpsr_set_overflow_flag(
                    ((self.r[rs] ^ op2) & 0x8000_0000 == 0)
                        && ((self.r[rs] ^ result) & 0x8000_0000 != 0),
                );
            }
            _ => unreachable!(),
        }

        1
    }

    pub fn dispatch_thumb_conditional_branch(&mut self, opcode: u16) -> u8 {
        let cond = Cond::from_u8(((opcode >> 8) & 0xF) as u8);
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
                trace!("B #{:X} + 2 + #{:X}", self.r.pc, signed_offset);
                self.r.pc = (self.r.pc as i32 + 2 + signed_offset) as u32;
                3
            }
            0b10 => {
                trace!("BL (part 1)");
                let n = (opcode & 0x7FF) as u32;
                *self.r.lr_mut() = self.r.pc + 4 + /*2 +*/ (n << 12);
                1
            }
            0b11 | 0b01 => {
                let n = (opcode & 0x7FF) as u32;
                let old_pc = self.r.pc; //+ 2;
                                        // add 2 here?
                                        // as this is the second half of the instruction, probably not...
                self.r.pc = self.r.lr() + (n << 1);
                trace!("BL to 0x{:X}", self.r.pc);
                *self.r.lr_mut() = old_pc;
                3
            }
            // I think 0b01 is only on ARM9, so let's just route it to the same BL
            /*
            0b01 => {
                todo!("Second opcode for THUMB branch long with link");
            }
            */
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
            *self.r.lr_mut() = self.r.pc + 4;
        }
        let new_pc = self.r.pc as i32 + 4 + (signed_offset * 4);
        trace!(
            "Branching at 0x{:X} to 0x{:X} with offset {} {:b}",
            self.r.pc,
            new_pc,
            signed_offset,
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
        let op_reg = (opcode >> 16) & 0xF;
        // if it's the PC, we do extra logic
        let op_reg = if op_reg == 0xE { 0 } else { op_reg };
        let dest_reg = (opcode >> 12) & 0xF;
        let mut op1 = self.r[op_reg as u8];
        let mut op2;

        if op_reg == 0xF {
            if !imm && (opcode >> 4) & 1 == 1 {
                op1 += 12;
            } else {
                op1 += 8;
            }
        }

        if imm {
            let ror_shift = (opcode >> 8) & 0xF;
            op2 = opcode & 0xFF;
            if ror_shift != 0 {
                op2 = op2.rotate_right(ror_shift * 2);
                self.r.cpsr_set_carry_flag(op2 & 0x8000_0000 != 0);
            }
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

            if shift_amt == 0 {
                // shift amount of 0 is a special case
                /*
                          Zero Shift Amount (Shift Register by Immediate, with Immediate=0)

                LSL#0: No shift performed, ie. directly Op2=Rm, the C flag is NOT affected.
                LSR#0: Interpreted as LSR#32, ie. Op2 becomes zero, C becomes Bit 31 of Rm.
                ASR#0: Interpreted as ASR#32, ie. Op2 and C are filled by Bit 31 of Rm.
                ROR#0: Interpreted as RRX#1 (RCR), like ROR#1, but Op2 Bit 31 set to old C.

                           */
                match shift_type {
                    // LSL
                    0 => {
                        op2 = self.r[rm as u8];
                    }
                    // LSR
                    1 => todo!("LSR zero shift amount handling"),
                    // ASR
                    2 => {
                        op2 = ((self.r[rm as u8] as i32) >> 31) as u32;
                        self.r.cpsr_set_carry_flag(op2 & 1 == 1);
                    }
                    // ROR
                    3 => todo!("ROR zero shift amount handling"),
                    _ => unreachable!(),
                }
            } else {
                match shift_type {
                    // LSL
                    0 => {
                        // TODO: review overflowing here
                        op2 = self.r[rm as u8].overflowing_shl(shift_amt as _).0;
                    }
                    // LSR
                    1 => {
                        op2 = self.r[rm as u8] >> shift_amt;
                    }
                    // ASR
                    2 => todo!("ASR"),
                    // ROR
                    3 => todo!("ROR"),
                    _ => unreachable!(),
                }
            }
        }

        // detect if ALU instruction is actually an MRS/MSR: PSR transfer
        // TODO: op_reg != 0xF = SWP
        if (sub_opcode >> 2) == 0b10 && !s {
            let psr_src_dest = (opcode >> 22) & 1 == 1;
            let psr_subopcode = (opcode >> 21) & 1 == 1;
            if psr_subopcode {
                trace!("MSR");
                // MSR: Psr[field] = Op
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
                    // TODO: set flags?
                } else {
                    debug_assert_eq!((opcode >> 4) & 0xFF, 0);
                    let reg_idx = opcode & 0xF;
                    debug_assert!(reg_idx < 15);
                    self.r[reg_idx as u8]
                };
                // HACK: if in user mode, just force CPSR
                if psr_src_dest && self.r.register_mode() != Some(RegisterMode::User) {
                    *self.r.get_spsr_mut() &= !mask;
                    *self.r.get_spsr_mut() |= val & mask;
                } else {
                    self.r.cpsr &= !mask;
                    self.r.cpsr |= val & mask;
                }
            } else {
                if op_reg != 0xF {
                    let byte = (opcode >> 22) & 1 == 1;
                    let rn = ((opcode >> 16) & 0xF) as u8;
                    let rd = ((opcode >> 12) & 0xF) as u8;
                    let rm = (opcode & 0xF) as u8;
                    let mut cycles = 0;

                    trace!("SWP r{} r{} [r{}]", rd, rm, rn);

                    if byte {
                        let o = self.get_mem8(self.r[rn]);
                        self.r[rd] = o.0 as u32;
                        cycles += o.1;
                        cycles += self.set_mem8(self.r[rn], self.r[rm] as u8);
                    } else {
                        let o = self.get_mem32(self.r[rn]);
                        self.r[rd] = o.0;
                        cycles += o.1;
                        cycles += self.set_mem32(self.r[rn], self.r[rm]);
                    }

                    return 4 + cycles;
                }

                trace!("MRS");
                debug_assert_eq!(opcode & 0xFFF, 0);
                debug_assert!(dest_reg < 15);
                debug_assert_eq!(imm, false);

                // HACK: if in user mode just force CPSR
                // MRS: Rd = Psr
                let v = if psr_src_dest && self.r.register_mode() != Some(RegisterMode::User) {
                    self.r.get_spsr()
                } else {
                    self.r.cpsr
                };

                self.r[dest_reg as u8] = v;
            }
            return 1;
        }
        match sub_opcode {
            // AND
            0x0 => {
                trace!("AND r{:X} = {:X} & {:X}", dest_reg, op1, op2);
                let result = op1 & op2;
                self.r[dest_reg as u8] = result;
                if s {
                    self.r.cpsr_set_zero_flag(result == 0);
                    self.r.cpsr_set_sign_flag((result >> 31) == 1);
                }
            }
            // EOR
            0x1 => {
                trace!("EOR r{:X} = {:X} ^ {:X}", dest_reg, op1, op2);
                let result = op1 ^ op2;
                self.r[dest_reg as u8] = result;
                if s {
                    self.r.cpsr_set_zero_flag(result == 0);
                    self.r.cpsr_set_sign_flag((result >> 31) == 1);
                }
            }
            // SUB
            0x2 => {
                trace!("SUB r{:X} = {:X} - {:X}", dest_reg, op1, op2);
                let result = op1.wrapping_sub(op2);
                self.r[dest_reg as u8] = result;
                if s {
                    self.r.cpsr_set_zero_flag(result == 0);
                    self.r.cpsr_set_sign_flag(result & 0x8000_0000 != 0);
                    // TODO: review this
                    self.r.cpsr_set_overflow_flag(false);
                    self.r.cpsr_set_carry_flag(false);
                }
            }
            // ADD
            0x4 => {
                trace!("ADD r{:X} = {:X} + {:X}", dest_reg, op1, op2);
                let result = op1.wrapping_add(op2);
                self.r[dest_reg as u8] = result;
                if s {
                    self.r.cpsr_set_zero_flag(result == 0);
                    self.r.cpsr_set_sign_flag((result >> 31) == 1);
                    self.r.cpsr_set_overflow_flag(
                        ((op1 ^ op2) & 0x8000_0000 == 0) && ((op1 ^ result) & 0x8000_0000 != 0),
                    );
                    self.r
                        .cpsr_set_carry_flag(((op1 as u64) + (op2 as u64)) > 0xFFFF_FFFF);
                }
            }
            // ADC
            0x5 => {
                trace!("ADC r{:X} = {:X} + {:X} + C", dest_reg, op1, op2);
                let result = op1
                    .wrapping_add(op2)
                    .wrapping_add(self.r.cpsr_carry_flag() as u32);
                self.r[dest_reg as u8] = result;
                if s {
                    self.r.cpsr_set_zero_flag(result == 0);
                    self.r.cpsr_set_sign_flag((result >> 31) == 1);
                    self.r.cpsr_set_overflow_flag(
                        ((op1 ^ op2) & 0x8000_0000 == 0) && ((op1 ^ result) & 0x8000_0000 != 0),
                    );
                    self.r
                        .cpsr_set_carry_flag(((op1 as u64) + (op2 as u64)) > 0xFFFF_FFFF);
                }
            }
            // SBC
            0x6 => {
                trace!("SBC r{:X} = {:X} - {:X} + C", dest_reg, op1, op2);
                let result = op1
                    .wrapping_sub(op2)
                    .wrapping_add(self.r.cpsr_carry_flag() as u32)
                    .wrapping_sub(1);
                self.r[dest_reg as u8] = result;
                if s {
                    self.r.cpsr_set_zero_flag(result == 0);
                    self.r.cpsr_set_sign_flag((result >> 31) == 1);
                    self.r.cpsr_set_overflow_flag(
                        ((op1 ^ op2) & 0x8000_0000 == 0) && ((op1 ^ result) & 0x8000_0000 != 0),
                    );
                    // TODO: carry flag
                }
            }
            // RSC
            0x7 => {
                trace!("RSC r{:X} = {:X} - {:X} + C", dest_reg, op2, op1);
                let result = op2
                    .wrapping_sub(op1)
                    .wrapping_add(self.r.cpsr_carry_flag() as u32)
                    .wrapping_sub(1);
                self.r[dest_reg as u8] = result;
                if s {
                    self.r.cpsr_set_zero_flag(result == 0);
                    self.r.cpsr_set_sign_flag((result >> 31) == 1);
                    self.r.cpsr_set_overflow_flag(
                        ((op1 ^ op2) & 0x8000_0000 == 0) && ((op1 ^ result) & 0x8000_0000 != 0),
                    );
                    // TODO: carry flag
                }
            }
            // TST
            0x8 => {
                trace!("TST r{:X} = {:X} & {:X}", dest_reg, op1, op2);
                let result = op1 & op2;
                if s {
                    self.r.cpsr_set_zero_flag(result == 0);
                    self.r.cpsr_set_sign_flag((result >> 31) == 1);
                }
            }
            // TEQ
            0x9 => {
                trace!("TEQ r{:X} = {:X} ^ {:X}", dest_reg, op1, op2);
                let result = op1 ^ op2;
                if s {
                    self.r.cpsr_set_zero_flag(result == 0);
                    self.r.cpsr_set_sign_flag((result >> 31) == 1);
                }
            }
            // CMP
            0xA => {
                trace!("CMP r{:X} = {:X} - {:X}", dest_reg, op1, op2);
                let result = op1.wrapping_sub(op2);
                if s {
                    self.r.cpsr_set_zero_flag(result == 0);
                    self.r.cpsr_set_sign_flag((result >> 31) == 1);
                    self.r.cpsr_set_overflow_flag(false);
                    self.r.cpsr_set_carry_flag(false);
                }
            }
            // ORR
            0xC => {
                trace!("ORR r{:X} = {:X} | {:X}", dest_reg, op1, op2);
                let result = op1 | op2;
                self.r[dest_reg as u8] = result;
                if s {
                    self.r.cpsr_set_zero_flag(result == 0);
                    self.r.cpsr_set_sign_flag((result >> 31) == 1);
                }
            }
            // MOV
            0xD => {
                trace!("MOV r{:X} = {:X}", dest_reg, op2);
                self.r[dest_reg as u8] = op2;
                if s {
                    self.r.cpsr_set_zero_flag(op2 == 0);
                    self.r.cpsr_set_sign_flag((op2 >> 31) == 1);
                }
            }
            // BIC
            0xE => {
                trace!("BIC r{:X} = {:X} & !{:X}", dest_reg, op1, op2);
                let result = op1 & !op2;
                self.r[dest_reg as u8] = result;
                if s {
                    self.r.cpsr_set_zero_flag(result == 0);
                    self.r.cpsr_set_sign_flag((result >> 31) == 1);
                }
            }
            // MVN
            0xF => {
                trace!("MVN r{:X} = !{:X}", dest_reg, op2);
                let result = !op2;
                self.r[dest_reg as u8] = result;
                if s {
                    self.r.cpsr_set_zero_flag(result == 0);
                    self.r.cpsr_set_sign_flag((result >> 31) == 1);
                    // carry flag from shift
                }
            }
            _ => unimplemented!("ALU instruction 0x{:X} at 0x{:X}", sub_opcode, self.r.pc),
        }

        // HACK: don't set CPSR if in user mode
        // TODO: this is probably wrong, we should avoid modifying flags in user mode too
        if s && dest_reg == 0xF && self.r.register_mode() != Some(RegisterMode::User) {
            self.r.cpsr = self.r.get_spsr();
        }

        // TODO: writing to PC affects things
        // TODO: timing
        //  "Execution Time: (1+p)S+rI+pN. Whereas r=1 if I=0 and R=1 (ie. shift by register); otherwise r=0. And p=1 if Rd=R15; otherwise p=0.""

        4
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
        // TODO: R15 may need special logic here
        let base_reg = (opcode >> 16) & 0xF;
        // TODO: R15 may need special logic here
        let src_dest_reg = (opcode >> 12) & 0xF;

        let offset = if imm {
            let shift_type = (opcode >> 5) & 3;
            let rm = (opcode & 0xF) as u8;
            let shift_amount = (opcode >> 7) & 0x1F;
            if shift_amount == 0 {
                // TODO:
                //panic!("Special logic here? docs don't describe it much, maybe we ignore it here");
                warn!("Special logic here? docs don't describe it much, maybe we ignore it here");
            }
            match shift_type {
                0b00 => self.r[rm] << shift_amount,
                0b01 => self.r[rm] >> shift_amount,
                0b10 => ((self.r[rm] as i32) >> shift_amount) as u32,
                0b11 => self.r[rm].rotate_right(shift_amount),
                _ => unreachable!(),
            }
        } else {
            opcode & 0xFFF
        };

        let mut val = self.r[base_reg as u8];
        if base_reg == 15 {
            val += 8;
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
                trace!("LDRB r{:X} = mem[{:X}]", src_dest_reg, val);
                let o = self.get_mem8(val);
                cycles += o.1;
                // TODO: does this zero the high bits?
                /*
                self.r[src_dest_reg as u8] &= !0xFF;
                self.r[src_dest_reg as u8] |= o.0 as u32;
                */
                self.r[src_dest_reg as u8] = o.0 as u32;
            } else {
                trace!("LDR r{:X} = mem[{:X}]", src_dest_reg, val);
                let o = self.get_mem32(val);
                cycles += o.1;
                self.r[src_dest_reg as u8] = o.0;
            }
        } else {
            if byte {
                trace!("STRB mem[{:X}] = r{:X}", val, src_dest_reg);
                cycles += self.set_mem8(val, self.r[src_dest_reg as u8] as u8);
            } else {
                trace!("STR mem[{:X}] = r{:X}", val, src_dest_reg);
                cycles += self.set_mem32(val, self.r[src_dest_reg as u8]);
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
        if !p && write_back {
            warn!("Write back bit has special meaning in post-inc mode, figure this out");
            //todo!("Write back bit has special meaning in post-inc mode, figure this out");
        }

        cycles
    }

    pub fn dispatch_block_data(&mut self, opcode: u32) -> u8 {
        let p = (opcode >> 24) & 1 == 1;
        let up = (opcode >> 23) & 1 == 1;
        let force_user_mode = (opcode >> 22) & 1 == 1;
        let write_back = (opcode >> 21) & 1 == 1;
        let load = (opcode >> 20) & 1 == 1;
        let rn = ((opcode >> 16) & 0xF) as u8;
        let r_list = opcode & 0xFFFF;
        let mut cycles = 0;

        let mut base = self.r[rn];

        if force_user_mode {
            //todo!("Figure out force user mode");
            warn!("Figure out force user mode");
        }

        if load {
            trace!("LDM");
            for i in 0..16 {
                if r_list & (1 << i) == 0 {
                    continue;
                }
                if p {
                    base = if up {
                        base.wrapping_add(4)
                    } else {
                        base.wrapping_sub(4)
                    }
                }
                let o = self.get_mem32(base);
                self.r[i as u8] = o.0;
                cycles += o.1 + 2;
                if !p {
                    base = if up {
                        base.wrapping_add(4)
                    } else {
                        base.wrapping_sub(4)
                    }
                }
            }
        } else {
            trace!("STM");
            for i in 0..16 {
                if r_list & (1 << i) == 0 {
                    continue;
                }
                if p {
                    base = if up {
                        base.wrapping_add(4)
                    } else {
                        base.wrapping_sub(4)
                    }
                }
                cycles += self.set_mem32(base, self.r[i as u8]);
                cycles += 1;
                if !p {
                    base = if up {
                        base.wrapping_add(4)
                    } else {
                        base.wrapping_sub(4)
                    }
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
        let op = self.r[op_reg as u8];
        let thumb_mode = op & 1 == 1;
        let new_pc = op - (thumb_mode as u32);
        if self.r.thumb_enabled() != thumb_mode {
            if thumb_mode {
                info!(
                    "Enabling Thumb mode from 0x{:X} to 0x{:X}!",
                    self.r.pc, new_pc
                );
            } else {
                info!("Enabling ARM mode!");
            }
        }
        match subopcode {
            0b0001 => {
                // BX
                trace!("BX pc = r{} (0x{:X})", op_reg, op);
                self.r.pc = new_pc;
                self.r.set_thumb(thumb_mode);
            }
            0b0010 => {
                panic!("Change to Jazelle mode not implemented");
            }
            0b0011 => {
                // BLX
                trace!(
                    "BLX pc = r{} (0x{:X}), lr = 0x{:X}",
                    op_reg,
                    op,
                    self.r.pc + 4
                );
                let old_pc = self.r.pc;
                self.r.set_thumb(thumb_mode);
                self.r.pc = new_pc;
                *self.r.lr_mut() = old_pc + 4;
            }
            _ => panic!("Unknown branch and exchange subopcode 0x{:X}", subopcode),
        }
        // 2S + 1N
        3
    }
}
