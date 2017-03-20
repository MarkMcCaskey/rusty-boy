use std::ops::{Index, IndexMut};
use cpu::constants::*;

/// The things that are constant between all types of cartridges
/// This also includes things like video ram
/// Thus this struct is best understood as dealing with any and all things
/// addressable
///
/// TODO: memory locking during certain periods (i.e. the rest of the virtual
/// memory system...)
pub struct Cartridge {
    /// top 16kb
    memory_bank0: [byte; 0x4000],
    cart_sub: CartridgeSubType,
    /// 8kb video ram
    video_ram: [byte; 0x2000],
    /// 8kb internal ram
    internal_ram: [byte; 0x2000],
    /// sprite attribute memory
    oam: [byte; 0xA0],

    /// 0xFF80-0xFFFF
    internal_ram2: [byte; 0x80],
    interrupt_flag: byte,
}

pub enum CartridgeSubType {
    ROM_only { memory_bank1: [byte; 0x4000] },
    MBC1 {
        /*
        MBC1 has two modes:
          * 16mbit ROM (with 128 banks), 8KB RAM (1 bank)
          * 4mbit (with 32 banks), 32KB RAM (4 banks)
         */
        //13 bits for 8KB addressing
        //addressing 16mbit = 2MB, (1kb = 10) (8kb = 13) (16kb = 14)
        //(2mb = 21)
        //21bits to index fully, because first 0x4000 address are sep
        memory_model: MBC1_type,
        memory_banks: [byte; (2 << 13) + (2 << 21) - 0x4000],
        ram_active: bool,
        //top two bits (21 & 22?) used for selecting RAM in 4_32 mode
        mem_bank_selector: u32,
    },
}

pub enum MBC1_type {
    sixteen_eight,
    four_thirtytwo,
}

const ref_zero: u8 = 0;

//for reading
impl Index<u16> for Cartridge {
    type Output = byte;

    fn index<'a>(&'a self, ind: u16) -> &'a byte {
        match ind {
            0x0000...0x3FFF => &self.memory_bank0[ind as usize],
            0x4000...0x7FFF => {
                match self.cart_sub {
                    CartridgeSubType::ROM_only { memory_bank1: ref membank1 } => {
                        &membank1[(ind - 0x4000) as usize]
                    }
                    CartridgeSubType::MBC1 { memory_model: MBC1_type::sixteen_eight,
                                             memory_banks: ref mb,
                                             ram_active: ra,
                                             mem_bank_selector: index } => {
                        &mb[((ind - 0x4000) as usize) + ((index * 0x4000) as usize)]
                    }
                    CartridgeSubType::MBC1 { memory_model: MBC1_type::four_thirtytwo,
                                             memory_banks: ref mb,
                                             ram_active: ra,
                                             mem_bank_selector: index } => unimplemented!(),

                    _ => unimplemented!(),
                }
            }
            // Video RAM:
            0x8000...0x9FFF => {
                //TODO: block reads if reads should be blocked
                unimplemented!()
            }
            // switchable RAM bank
            0xA000...0xBFFF => unimplemented!(),
            //internal ram
            0xC000...0xDFFF => &self.internal_ram[(ind - 0xC000) as usize],
            //echo of internal ram
            0xE000...0xFDFF => &self.internal_ram[(ind - 0xE000) as usize],
            // OAM
            0xFE00...0xFF9F => &self.oam[(ind - 0xFE00) as usize],
            // IO ports
            0xFF00...0xFF4B => {
                //TODO:
                unimplemented!()
            }
            //more internal RAM
            0xFF80...0xFFFE => &self.internal_ram2[(ind - 0xFF80) as usize],
            //interrupt flag
            0xFFFF => {
                //TODO:
                &self.interrupt_flag
            }
            _ => {
                panic!("Address 0x{:X} cannot be read from", ind);
            }
        }
    }
}

impl Cartridge {
    pub fn index_set(&mut self, ind: u16, val: u8) {
        match self.cart_sub {
            CartridgeSubType::ROM_only { memory_bank1: membank1 } => {
                match ind as usize {
                    0xFF80...0xFFFE => {
                        self.internal_ram2[(ind - 0xFF80) as usize] = val;
                    }
                    //internal ram
                    0xC000...0xDFFF => {
                        self.internal_ram[(ind - 0xC000) as usize] = val;
                    }
                    //echo of internal ram
                    0xE000...0xFDFF => {
                        self.internal_ram[(ind - 0xE000) as usize] = val;
                    }
                    0x0000...0x7FFF => {
                        error!("Cannot write to address 0x{:X} of a ROM-only cartridge",
                               ind);
                    }
                    addr => {
                        unimplemented!();
                    }
                }
            }
            CartridgeSubType::MBC1 { memory_model: MBC1_type::sixteen_eight,
                                     memory_banks: mb,
                                     ram_active: ra,
                                     mem_bank_selector: mut index } => {
                match ind as usize {
                    0x2000...0x3FFF => {
                        //take the lower 5 bits to select the 2nd ROM bank
                        let bank_select = if (val & 0x1F) == 0 { 1 } else { val & 0x1F };
                        index = bank_select as u32;
                        debug!("MBC1 switching second ROM bank to ROM bank {}", bank_select);
                    }
                    0x6000...0x7FFF => {
                        if (val & 0x1) == 1 {
                            debug!("MBC1 switching to 4-32 mode");
                            unimplemented!();
                        } else {
                            debug!("MBC1: already in 16-8 mode");
                        }
                    }
                    _ => unimplemented!(),
                }
            }

            _ => unimplemented!(),
        }
    }
}
