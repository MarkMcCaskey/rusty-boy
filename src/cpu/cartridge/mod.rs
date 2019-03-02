use std::ops::{Index, IndexMut};
use std::path::PathBuf;

use crate::cpu::constants::*;

/// A thing that is like a Cartridge
pub trait Cartridgey {
    fn load(rom_file: &str) -> Cartridge;

    fn read_rom_value(&self, index: u16) -> byte;

    #[allow(unused_variables)]
    fn read_ram_value(&self, index: u16) -> byte {
        panic!("This cartridge type does not provide RAM")
    }

    #[allow(unused_variables)]
    fn write_ram_value(&mut self, index: u16, value: byte) {
        panic!("This cartridge type does not provide RAM")
    }
}

/// The things that are constant between all types of cartridges
/// This also includes things like video ram
/// Thus this struct is best understood as dealing with any and all things
/// addressable
///
/// TODO: memory locking during certain periods (i.e. the rest of the virtual
/// memory system...)
pub struct Cartridge {
    memory_bank0: [byte; 0x4000],
    cart_sub: Option<CartridgeSubType>,
}

impl Cartridgey for Cartridge {
    fn load(file_path: &str) -> Cartridge {
        use std::fs::File;
        use std::io::Read;

        let mut rom = File::open(file_path).expect("Could not open rom file");
        let mut rom_buffer: [u8; 0x4000] = [0u8; 0x4000];

        match rom.read_exact(&mut rom_buffer) {
            Ok(_) => (),
            Err(e) => error!("Could not read from ROM: {:?}", e),
        }

        info!("RAM bank value: {:X}", rom_buffer[0x149]);
        info!("ROM bank value: {:X}", rom_buffer[0x148]);

        if let Some(cart_type) = to_cartridge_type(rom_buffer[0x147]) {
            info!("Cartridge type: {:?}", cart_type);

            match cart_type {
                //TODO: verify this
                CartridgeType::RomOnly | CartridgeType::RomRam | CartridgeType::RomRamBatt => {
                    let mut rom_buffer2: [u8; 0x4000] = [0u8; 0x4000];
                    match rom.read_exact(&mut rom_buffer2) {
                        Ok(_) => (),
                        Err(e) => error!("Could not read from ROM bank: {:?}", e),
                    }

                    Cartridge {
                        memory_bank0: rom_buffer,
                        cart_sub: Some(CartridgeSubType::RomOnly {
                            memory_bank1: rom_buffer2,
                            ram_bank: [0u8; 0x2000],
                        }),
                    }
                }
                CartridgeType::RomMBC1
                | CartridgeType::RomMBC1Ram
                | CartridgeType::RomMBC1RamBatt => {
                    let mut rom_buffer2: [u8; 0x4000] = [0u8; 0x4000];

                    let mut rom_vec: Vec<[byte; 0x4000]> = vec![];
                    let mut ram_vec: Vec<[byte; 0x2000]> = vec![];

                    //TODO: improve this.  This is hacky code for testing...
                    let ram_buffer: [byte; 0x2000] = [0u8; 0x2000];
                    for _ in 0..16 {
                        ram_vec.push(ram_buffer);
                    }

                    //DEBUG CODE: FIXME etc.
                    rom_vec.push(rom_buffer);
                    while let Ok(_) = rom.read_exact(&mut rom_buffer2) {
                        rom_vec.push(rom_buffer2);
                    }

                    Cartridge {
                        memory_bank0: rom_buffer,
                        cart_sub: Some(CartridgeSubType::Mbc1 {
                            memory_model: Mbc1Type::SixteenEight,
                            memory_banks: rom_vec,
                            ram_banks: ram_vec,
                            ram_active: false,
                            mem_bank_selector: 1, //TODO: verify
                            ram_bank_selector: 0,
                        }),
                    }
                }
                CartridgeType::RomMBC3
                | CartridgeType::RomMBC3Ram
                | CartridgeType::RomMBC3RamBatt
                | CartridgeType::RomMBC3TimerRamBatt => {
                    //COPIED FROM ABOVE
                    let mut rom_buffer2: [u8; 0x4000] = [0u8; 0x4000];

                    let mut rom_vec: Vec<[byte; 0x4000]> = vec![];
                    let mut ram_vec: Vec<[byte; 0x2000]> = vec![];

                    //TODO: improve this.  This is hacky code for testing...
                    let ram_buffer: [byte; 0x2000] = [0u8; 0x2000];
                    for _ in 0..16 {
                        ram_vec.push(ram_buffer);
                    }

                    //DEBUG CODE: FIXME etc.
                    rom_vec.push(rom_buffer);
                    while let Ok(_) = rom.read_exact(&mut rom_buffer2) {
                        rom_vec.push(rom_buffer2);
                    }

                    Cartridge {
                        memory_bank0: rom_buffer,
                        cart_sub: Some(CartridgeSubType::Mbc3 {
                            memory_banks: rom_vec,
                            ram_banks: ram_vec,
                            ram_active: false,
                            mem_bank_selector: 1, //TODO: verify
                            ram_bank_selector: 0,
                        }),
                    }
                }
                _ => {
                    panic!("Cartridge type {:?} is not supported!", cart_type);
                }
            }
        } else {
            // to_cartridge_type failed
            panic!("Could not find a cartridge type!");
        }

        /*        debug!("Cart loaded with {} ram banks",
               match self.mem[0x149] {
                   0 => 0,
                   1 => 1,
                   2 => 1,
                   3 => 4,
                   4 => 16,
                   _ => {
            error!("Undefined value at 0x149 in ROM");
            -1
        }
               });*/
    }

    #[allow(unused_variables)]
    fn read_rom_value(&self, index: u16) -> byte {
        unimplemented!()
        //        *self.index(index)
    }

    #[allow(unused_variables)]
    fn read_ram_value(&self, index: u16) -> byte {
        unimplemented!()
        //*self.index(index)
        //panic!("This cartridge type does not provide RAM")
    }

    fn write_ram_value(&mut self, index: u16, value: byte) {
        match self.cart_sub {
            Some(CartridgeSubType::Mbc1 {
                memory_model: ref mut mm,
                mem_bank_selector: ref mut mbs,
                ram_bank_selector: ref mut rbs,
                ram_active: ref mut ra,
                ..
            }) if index <= 0x7FFF => {
                match index {
                    //RAM activation
                    0x0000...0x1FFF => {
                        let ram_active = (value & 0xA) == 0xA;
                        if ram_active {
                            debug!("MBC1: set RAM to active");
                        } else {
                            debug!("MBC1: set RAM to inactive");
                        }

                        *ra = ram_active;
                    }
                    // bank select
                    0x2000...0x3FFF => {
                        let rom_bank = if (value & 0x1F) == 0 {
                            1
                        } else {
                            (value & 0x1F) as u32
                        };
                        debug!("MBC1: Switching to ROM bank {}", rom_bank);
                        *mbs = rom_bank
                    }
                    // TODO: selecting MSBs of ROM bank in 16/8 mode
                    0x4000...0x5FFF => {
                        match *mm {
                            Mbc1Type::FourThirtytwo => {
                                debug!("MBC1 4-32: selecting ROM bank {}", value & 0x3);
                                *rbs = (value & 0x3) as u32;
                            }
                            Mbc1Type::SixteenEight => {
                                //NOTE WARNING:
                                error!("MBC1 16-8: Setting two MSBS of ROM bank {}", value & 0x3);
                                *mbs = (value & 0x3) as u32;
                            }
                        }
                    }
                    // cartridge memory model select
                    0x6000...0x7FFF => {
                        *mm = if (index & 1) == 1 {
                            debug!("MBC1: Switching to 4-32 mode");
                            Mbc1Type::FourThirtytwo
                        } else {
                            debug!("MBC1: Switching to 16-8 mode");
                            Mbc1Type::SixteenEight
                        }
                    }
                    _ => unreachable!(),
                }
            }
            Some(CartridgeSubType::Mbc3 {
                mem_bank_selector: ref mut mbs,
                //ram_bank_selector: ref mut rbs,
                ram_active: ref mut ra,
                ..
            }) => {
                match index {
                    //RAM activation
                    0x0000...0x1FFF => {
                        let ram_active = (value & 0xA) == 0xA;
                        if ram_active {
                            debug!("MBC3: set RAM to active");
                        } else {
                            debug!("MBC3: set RAM to inactive");
                        }

                        *ra = ram_active;
                    }
                    // bank select
                    0x2000...0x3FFF => {
                        let rom_bank = if (value & 0x7F) == 0 {
                            1
                        } else {
                            (value & 0x7F) as u32
                        };
                        debug!("MBC3: Switching to ROM bank {}", rom_bank);
                        *mbs = rom_bank
                    }
                    _ => debug!(
                        "MBC3 likely not fully implemented!: writing 0x{:X} to 0x{:X}",
                        value, index
                    ),
                }
            }

            Some(CartridgeSubType::RomOnly { .. }) | None | _ => self[index] = value,
        }
        //panic!("This cartridge type does not provide RAM")
    }
}

pub enum CartridgeSubType {
    RomOnly {
        memory_bank1: [byte; 0x4000],
        ram_bank: [byte; 0x2000],
    },
    Mbc1 {
        memory_model: Mbc1Type,
        //memory_banks: [byte; 0x4000], //(2 << 13) + (2 << 21) - 0x4000],
        memory_banks: Vec<[byte; 0x4000]>,
        ram_banks: Vec<[byte; 0x2000]>,
        ram_active: bool,
        //top two bits (21 & 22?) used for selecting RAM in 4_32 mode
        mem_bank_selector: u32,
        ram_bank_selector: u32,
    },
    Mbc3 {
        //unclear if this has 16-8/4-32 mode....
        memory_banks: Vec<[byte; 0x4000]>,
        ram_banks: Vec<[byte; 0x2000]>,
        ram_active: bool, //unsure if this is needed
        mem_bank_selector: u32,
        ram_bank_selector: u32,
    },
}

#[derive(Clone, Copy)]
pub enum Mbc1Type {
    SixteenEight,
    FourThirtytwo,
}

const REF_ZERO: &'static u8 = &0;

//for reading and writing
impl IndexMut<u16> for Cartridge {
    fn index_mut(&mut self, ind: u16) -> &mut byte {
        trace!("indexmut: {:X}", ind);
        match ind {
            0x0000...0x3FFF => &mut self.memory_bank0[ind as usize],
            0x4000...0x7FFF => {
                match self.cart_sub {
                    Some(CartridgeSubType::RomOnly { memory_bank1: ref mut membank1, .. }) => {
                        &mut membank1[(ind - 0x4000) as usize]
                    }
                    Some(CartridgeSubType::Mbc1 { memory_model: Mbc1Type::SixteenEight,
                                                  memory_banks: ref mut mb,
                                                  mem_bank_selector: index,
                                             .. }) => {
                        &mut mb[index as usize][((ind - 0x4000) as usize)]
                    }
                    Some(CartridgeSubType::Mbc1 { .. /*memory_model: Mbc1Type::FourThirtytwo,
                                             memory_banks: ref mb,
                                             //ram_active: ra,
                                             mem_bank_selector: index*/ }) => unimplemented!(),

                    _ => unimplemented!(),
                }
            }
            // Video RAM:
            0x8000...0x9FFF => {
                //TODO: block needs to handle if reads should be blocked
                panic!("At access video ram");
            }
            // switchable RAM bank
            0xA000...0xBFFF => {
                match self.cart_sub {
                    Some(CartridgeSubType::RomOnly {
                        ram_bank: ref mut rambank,
                        ..
                    }) => {
                        //error!("Writing to 0x{:X} does not do anything on a ROM only cartridge",
                        //      ind);
                        &mut rambank[(ind - 0xA000) as usize]
                    }
                    Some(CartridgeSubType::Mbc1 {
                        ram_banks: ref mut rb,
                        //ram_active: true,
                        ram_bank_selector: rbs,
                        ..
                    }) => &mut rb[rbs as usize][(ind - 0xA000) as usize],
                    _ => panic!("at switchable ram bank"),
                }
            }
            //internal ram
            /*0xC000...0xDFFF => &self.internal_ram[(ind - 0xC000) as usize],
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
            }*/
            _ => {
                panic!("Address 0x{:X} cannot be read from", ind);
            }
        }
    }
}

//for reading
impl Index<u16> for Cartridge {
    type Output = byte;

    fn index<'a>(&'a self, ind: u16) -> &'a byte {
        match ind {
            0x0000...0x3FFF => unsafe { self.memory_bank0.get_unchecked(ind as usize) },
            0x4000...0x7FFF => {
                match self.cart_sub {
                    Some(CartridgeSubType::RomOnly {
                        memory_bank1: ref membank1,
                        ..
                    }) => unsafe { membank1.get_unchecked((ind - 0x4000) as usize) },
                    Some(CartridgeSubType::Mbc1 {
                        memory_model: Mbc1Type::SixteenEight,
                        memory_banks: ref mb,
                        mem_bank_selector: index,
                        ..
                    }) => unsafe { mb[index as usize].get_unchecked((ind - 0x4000) as usize) },
                    Some(CartridgeSubType::Mbc1 {
                        memory_model: Mbc1Type::FourThirtytwo,
                        memory_banks: ref mb,
                        mem_bank_selector: index,
                        ..
                    }) => {
                        unsafe { mb[index as usize].get_unchecked((ind - 0x4000) as usize) }
                        //panic!("Indexing {:X}", ind)
                    }
                    Some(CartridgeSubType::Mbc3 {
                        memory_banks: ref mb,
                        mem_bank_selector: index,
                        ..
                    }) => unsafe { mb[index as usize].get_unchecked((ind - 0x4000) as usize) },

                    _ => panic!("Indexing {:X}", ind),
                }
            }
            // Video RAM:
            0x8000...0x9FFF => {
                //TODO: block needs to handle if reads should be blocked
                panic!("At access video ram");
            }
            // switchable RAM bank
            0xA000...0xBFFF => {
                match self.cart_sub {
                    Some(CartridgeSubType::RomOnly {
                        ram_bank: ref rambank,
                        ..
                    }) => unsafe { rambank.get_unchecked((ind - 0xA000) as usize) },
                    Some(CartridgeSubType::Mbc1 {
                        // ram_active: true,
                        ram_banks: ref ram_vec,
                        ram_bank_selector: rbs,
                        ..
                    }) => unsafe {
                        ram_vec[rbs as usize].get_unchecked((ind as u32 - 0xA000) as usize)
                    },
                    Some(CartridgeSubType::Mbc3 {
                        ram_banks: ref ram_vec,
                        ram_bank_selector: rbs,
                        ..
                    }) => unsafe {
                        ram_vec[rbs as usize].get_unchecked((ind as u32 - 0xA000) as usize)
                    },
                    /*Some(CartridgeSubType::Mbc1 { ram_active: false, .. }) => {
                        panic!("Ram is not active")
                    }*/
                    _ => panic!("at switchable ram bank"),
                }
            }
            //internal ram
            /*0xC000...0xDFFF => &self.internal_ram[(ind - 0xC000) as usize],
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
            }*/
            _ => {
                panic!("Address 0x{:X} cannot be read from", ind);
            }
        }
    }
}

impl Cartridge {
    pub fn load_ram(&mut self, path: &PathBuf) {
        use std::fs::File;
        use std::io::Read;

        let mut file = match File::open(path) {
            Ok(f) => f,
            _ => return,
        };

        match self.cart_sub {
            Some(CartridgeSubType::Mbc1 {
                ram_banks: ref mut ra,
                ..
            }) => {
                for ref mut b in ra {
                    file.read_exact(&mut b[..]).unwrap();
                }
            }
            _ => (),
        }
    }

    pub fn save_ram(&self, path: &PathBuf) {
        use std::fs::File;
        use std::io::Write;

        let mut file = match File::create(path) {
            Ok(f) => f,
            _ => return,
        };

        match self.cart_sub {
            Some(CartridgeSubType::Mbc1 {
                ram_banks: ref ra, ..
            }) => {
                for ref b in ra {
                    match file.write(&b[..]) {
                        Ok(_) => (),
                        Err(e) => error!("Error saving ram: {:?}", e),
                    }
                }
            }
            _ => (),
        }
    }

    pub fn reset(&mut self) {
        panic!("at reset");
        /*self.mem[0xFF05] = 0x00;
        self.mem[0xFF06] = 0x00;
        self.mem[0xFF07] = 0x00;
        self.mem[0xFF10] = 0x80;
        self.mem[0xFF11] = 0xBF;
        self.mem[0xFF12] = 0xF3;
        self.mem[0xFF14] = 0xBF;
        self.mem[0xFF16] = 0x3F;
        self.mem[0xFF17] = 0x00;
        self.mem[0xFF19] = 0xBF;
        self.mem[0xFF1A] = 0x7F;
        self.mem[0xFF1B] = 0xFF;
        self.mem[0xFF1C] = 0x9F;
        self.mem[0xFF1E] = 0xBF;
        self.mem[0xFF20] = 0xFF;
        self.mem[0xFF21] = 0x00;
        self.mem[0xFF22] = 0x00;
        self.mem[0xFF23] = 0xBF;
        self.mem[0xFF24] = 0x77;
        self.mem[0xFF25] = 0xF3;
        self.mem[0xFF26] = 0xF1; //F1 for GB // TODOA:
        self.mem[0xFF40] = 0x91;
        self.mem[0xFF42] = 0x00;
        self.mem[0xFF43] = 0x00;
        self.mem[0xFF45] = 0x00;
        self.mem[0xFF47] = 0xFC;
        self.mem[0xFF48] = 0xFF;
        self.mem[0xFF49] = 0xFF;
        self.mem[0xFF4A] = 0x00;
        self.mem[0xFF4B] = 0x00;
        self.mem[0xFFFF] = 0x00;
        */
    }

    pub fn new() -> Cartridge {
        Cartridge {
            memory_bank0: [0u8; 0x4000],
            cart_sub: None,
        }
    }

    /*    pub fn index_set(&mut self, ind: u16, val: u8) {
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
    */
}

impl Default for Cartridge {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for Cartridge {
    fn clone(&self) -> Cartridge {
        let mut mem_bank0buf = [0u8; 0x4000];
        for (i, &mem_val) in self.memory_bank0.iter().enumerate() {
            mem_bank0buf[i] = mem_val;
        }

        Cartridge {
            memory_bank0: mem_bank0buf,
            cart_sub: self.cart_sub.clone(),
        }
    }
}

impl Clone for CartridgeSubType {
    fn clone(&self) -> CartridgeSubType {
        match *self {
            CartridgeSubType::RomOnly { memory_bank1: mem, ram_bank: rambank } => {
                let mut mem_buf = [0u8; 0x4000];
                let mut ram_buf = [0u8; 0x2000];

                for (i, &mem_val) in mem.iter().enumerate() {
                    mem_buf[i] = mem_val;
                }

                for (i, &ram_val) in rambank.iter().enumerate() {
                    ram_buf[i] = ram_val;
                }
                CartridgeSubType::RomOnly {
                    memory_bank1: mem_buf,
                    ram_bank: ram_buf,
                }
            }
            CartridgeSubType::Mbc1 { memory_model: ref mm,
                                     memory_banks: ref mb,
                                     ram_banks: ref rb,
                                     ram_active: ref ra,
                                     mem_bank_selector: ref mbs,
                                     ram_bank_selector: ref rbs } => {
                let mut ram_buf = [0u8; 0x2000];
                let mut rom_buf = [0u8; 0x4000];

                let mut new_rom_vec: Vec<[byte; 0x4000]> = vec![];
                let mut new_ram_vec: Vec<[byte; 0x2000]> = vec![];

                for ar in mb.iter() {
                    for (i, &v) in ar.iter().enumerate() {
                        rom_buf[i] = v;
                    }
                    new_rom_vec.push(rom_buf);
                }

                for ar in rb.iter() {
                    for (i, &v) in ar.iter().enumerate() {
                        ram_buf[i] = v;
                    }
                    new_ram_vec.push(ram_buf);
                }

                CartridgeSubType::Mbc1 {
                    memory_model: *mm,
                    memory_banks: new_rom_vec,
                    ram_banks: new_ram_vec,
                    ram_active: *ra,
                    mem_bank_selector: *mbs,
                    ram_bank_selector: *rbs,
                }
            }
            CartridgeSubType::Mbc3 { memory_banks: ref mb,
                                     ram_banks: ref rb,
                                     ram_active: ref ra,
                                     mem_bank_selector: ref mbs,
                                     ram_bank_selector: ref rbs } => {
                let mut ram_buf = [0u8; 0x2000];
                let mut rom_buf = [0u8; 0x4000];

                let mut new_rom_vec: Vec<[byte; 0x4000]> = vec![];
                let mut new_ram_vec: Vec<[byte; 0x2000]> = vec![];

                for ar in mb.iter() {
                    for (i, &v) in ar.iter().enumerate() {
                        rom_buf[i] = v;
                    }
                    new_rom_vec.push(rom_buf);
                }

                for ar in rb.iter() {
                    for (i, &v) in ar.iter().enumerate() {
                        ram_buf[i] = v;
                    }
                    new_ram_vec.push(ram_buf);
                }


                CartridgeSubType::Mbc3 {
                    memory_banks: new_rom_vec,
                    ram_banks: new_ram_vec,
                    ram_active: *ra,
                    mem_bank_selector: *mbs,
                    ram_bank_selector: *rbs,
                }
            }
            //        _ => panic!("Cannot clone this cartridge type"),
        }
    }
}
