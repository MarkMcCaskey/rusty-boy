pub const ZL: i8 = 0x80;
pub const NLV: i8 = 0x40;
pub const HL: i8 = 0x20;
pub const CL: i8 = 0x10;
pub const VBLANK_INTERRUPT_ADDRESS: u16 = 0x40;
pub const LCDC_INTERRUPT_ADDRESS: u16 = 0x48;
pub const TIMER_OVERFLOW_INTERRUPT_ADDRESS: u16 = 0x50;
pub const SERIAL_TRANSFER_INTERRUPT_ADDRESS: u16 = 0x58;
pub const P1013_INTERRUPT_ADDRESS: u16 = 0x60;
pub const DISPLAY_RAM_START: usize = 0x8000;
pub const DISPLAY_RAM_END: usize = 0x9FFF;
pub const OAM_START: usize = 0xFE00;
pub const OAM_END: usize = 0xFE9F;
pub const STAT_ADDR: usize = 0xFF41;


#[derive(Debug,Clone,Copy,PartialEq)]
pub enum CpuState {
    Normal,
    Halt,
    Stop,
}

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
    Num(i8),
}

#[derive(Clone,Copy,PartialEq,Debug)]
pub enum CpuRegister16 {
    BC,
    DE,
    HL,
    SP,
    Num(i16),
}

#[derive(Clone,Copy)]
pub enum Cc {
    NZ,
    Z,
    NC,
    C,
}

#[derive(Debug, PartialEq)]
enum CartridgeType {
    RomOnlvy = 0,
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

pub fn cc_dispatch(num: u8) -> Cc {
    match num {
        0 => Cc::NZ,
        1 => Cc::Z,
        2 => Cc::NC,
        3 => Cc::C,
        _ => panic!("Invalid number for Cc dispatch"),
    }
}

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

pub fn cpu16_dispatch(num: u8) -> CpuRegister16 {
    match num {
        0 => CpuRegister16::BC,
        1 => CpuRegister16::DE,
        2 => CpuRegister16::HL,
        3 => CpuRegister16::SP,
        _ => panic!("Invalid number for 16bit register dispatch"),
    }
}
