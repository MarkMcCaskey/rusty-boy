//! Constant values relating to the CPU
#[allow(non_camel_case_types)]
pub type byte = u8;
/// Cannot index memory directly, must be cast to a `usize`
pub type MemAddr = u16;

/// Location of the `Z` flag in the `F` register
pub const ZL: byte = 0x80;
/// Location of the `N` flag in the `F` register
pub const NLV: byte = 0x40;
/// Location of the `H` flag in the `F` register
pub const HL: byte = 0x20;
/// Location of the `C` flag in the `F` register
pub const CL: byte = 0x10;

/// The size of the Gameboy's memory
/// Additional 3 bytes to skip bounds check when fetching instr. operands
pub const MEM_ARRAY_SIZE: usize = 0xFFFF + 1 + 3;


/// Where the PC should go when the vblank interupt is handled
pub const VBLANK_INTERRUPT_ADDRESS: u16 = 0x40;
pub const LCDC_INTERRUPT_ADDRESS: u16 = 0x48;
pub const TIMER_OVERFLOW_INTERRUPT_ADDRESS: u16 = 0x50;
pub const SERIAL_TRANSFER_INTERRUPT_ADDRESS: u16 = 0x58;
/// Where the PC should go when button press interupts are handled
/// TODO: separate these out, offset of 0x8 per interrupt address
pub const P1013_INTERRUPT_ADDRESS: u16 = 0x60;

/// Begining of Display ram, CPU cannot access during certain states
/// of the PPU.  See `STAT` for more information (TODO: add more info here)
pub const DISPLAY_RAM_START: usize = 0x8000;
pub const DISPLAY_RAM_END: usize = 0x9FFF;
/// Start of OAM memory. OAM memory cannot be accessed during certain states
/// of the PPU
pub const OAM_START: usize = 0xFE00;
pub const OAM_END: usize = 0xFE9F;
/// TODO: List what STAT contains
pub const STAT_ADDR: usize = 0xFF41;




/// The state of the CPU's execution
#[derive(Debug,Clone,Copy,PartialEq)]
pub enum CpuState {
    /// CPU is running normally
    Normal,
    /// CPU is off and waiting for an interrupt
    Halt,
    /// CPU and screen are off and waiting for a button press
    Stop,
    /// CPU have executed illegal instruction
    Crashed,
}

/// 8-bit registers of the CPU and an 8-bit numeric literal
#[derive(Clone,Copy,PartialEq,Debug)]
pub enum CpuRegister {
    A,
    B,
    C,
    D,
    E,
    H,
    L,
    HL,
    Num(byte),
}

/// 16-bit registers of the CPU and an 16-bit numeric literal
#[derive(Clone,Copy,PartialEq,Debug)]
pub enum CpuRegister16 {
    BC,
    DE,
    HL,
    SP,
    AF,
    Num(i16),
}

/// TODO: verify CC
/// Control codes used in some control flow instructions
#[derive(Clone,Copy)]
pub enum Cc {
    /// Not zero
    NZ,
    /// Zero
    Z,
    /// No carry
    NC,
    /// Carry
    C,
}

/// The type of ROM
/// Located in the ROM itself at addr (TODO: this)
#[derive(Debug, PartialEq, Copy, Clone)]
pub enum CartridgeType {
    /// The only ROM type being targeted for version 0.1.0
    RomOnly = 0,
    RomMBC1 = 1,
    RomMBC1Ram = 2,
    RomMBC1RamBatt = 3,
    RomMBC2 = 5,
    RomMBC2Batt = 6,
    RomRam = 8,
    RomRamBatt = 9,
    RomMMM01 = 0xB,
    RomMMM01SRam = 0xC,
    RomMMM01SRamBatt = 0xD,
    RomMBC3TimerRamBatt = 0x10,
    RomMBC3 = 0x11,
    RomMBC3Ram = 0x12,
    RomMBC3RamBatt = 0x13,
    RomMBC5 = 0x19,
    RomMBC5Ram = 0x1A,
    RomMBC5RamBatt = 0x1B,
    RomMBC5RumbleSRam = 0x1D,
    RomMBC5RumbleSRamBatt = 0x1E,
    PocketCamera = 0x1F,
    BandaiTAMA5 = 0xFD,
    HudsonHuC3 = 0xFE,
    HudsonHuC1 = 0xFF,
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum MBCType {
    MBC1_16_8,
    MBC1_4_32,
}

pub fn to_cartridge_type(v: u8) -> Option<CartridgeType> {
    match v {
        0 => Some(CartridgeType::RomOnly),
        1 => Some(CartridgeType::RomMBC1),
        2 => Some(CartridgeType::RomMBC1Ram),
        3 => Some(CartridgeType::RomMBC1RamBatt),
        5 => Some(CartridgeType::RomMBC2),
        6 => Some(CartridgeType::RomMBC2Batt),
        8 => Some(CartridgeType::RomRam),
        9 => Some(CartridgeType::RomRamBatt),
        0xB => Some(CartridgeType::RomMMM01),
        0xC => Some(CartridgeType::RomMMM01SRam),
        0xD => Some(CartridgeType::RomMMM01SRamBatt),
        0x10 => Some(CartridgeType::RomMBC3TimerRamBatt),
        0x11 => Some(CartridgeType::RomMBC3),
        0x12 => Some(CartridgeType::RomMBC3Ram),
        0x13 => Some(CartridgeType::RomMBC3RamBatt),
        0x19 => Some(CartridgeType::RomMBC5),
        0x1A => Some(CartridgeType::RomMBC5Ram),
        0x1B => Some(CartridgeType::RomMBC5RamBatt),
        0x1D => Some(CartridgeType::RomMBC5RumbleSRam),
        0x1E => Some(CartridgeType::RomMBC5RumbleSRamBatt),
        0x1F => Some(CartridgeType::PocketCamera),
        0xFD => Some(CartridgeType::BandaiTAMA5),
        0xFE => Some(CartridgeType::HudsonHuC3),
        0xFF => Some(CartridgeType::HudsonHuC1),
        _ => None,
    }
}

/// Turns a number into a `CC` code
/// used in dispatching opcodes
pub fn cc_dispatch(num: u8) -> Cc {
    match num {
        0 => Cc::NZ,
        1 => Cc::Z,
        2 => Cc::NC,
        3 => Cc::C,
        _ => panic!("Invalid number for Cc dispatch"),
    }
}

/// Turns a number into a `CpuRegister`
pub fn cpu_dispatch(num: u8) -> CpuRegister {
    match num {
        0 => CpuRegister::B,
        1 => CpuRegister::C,
        2 => CpuRegister::D,
        3 => CpuRegister::E,
        4 => CpuRegister::H,
        5 => CpuRegister::L,
        6 => CpuRegister::HL,
        7 => CpuRegister::A,
        _ => panic!("Invalid 8bit register in cpu_dispatch!"),
    }
}

/// Turns a number into a `CpuRegister16`
pub fn cpu16_dispatch(num: u8) -> CpuRegister16 {
    match num {
        0 => CpuRegister16::BC,
        1 => CpuRegister16::DE,
        2 => CpuRegister16::HL,
        3 => CpuRegister16::SP,
        _ => panic!("Invalid number for 16bit register dispatch"),
    }
}

/// Turns a number into a `CpuRegister16`
pub fn cpu16_dispatch_push_pop(num: u8) -> CpuRegister16 {
    match num {
        0 => CpuRegister16::BC,
        1 => CpuRegister16::DE,
        2 => CpuRegister16::HL,
        3 => CpuRegister16::AF,
        _ => panic!("Invalid number for 16bit register dispatch"),
    }
}
