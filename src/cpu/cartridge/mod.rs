use std::ops::{Index, IndexMut};
use std::path::PathBuf;

use gameboy_rom::header::RomType;

use crate::cpu::constants::*;

/// A thing that is like a Cartridge
pub trait Cartridgey {
    fn load(rom_file: &str) -> Result<Cartridge, String>;

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
#[derive(Clone, Default)]
pub struct Cartridge {
    /// it's safe to assume that the size of this vec is at least 0x4000
    entire_rom_data: Vec<u8>,
    cart_sub: Option<CartridgeSubType>,
    // used when indexing into RAM when there's no RAM, etc.
    dummy_value: u8,
    pub gbc: bool,
    pub sgb: bool,
}

impl Cartridgey for Cartridge {
    fn load(file_path: &str) -> Result<Cartridge, String> {
        use std::fs::File;
        use std::io::Read;

        let mut rom =
            File::open(file_path).map_err(|e| format!("Could not open ROM file: {}", e))?;
        let mut rom_buffer = Vec::with_capacity(0x4000);
        rom.read_to_end(&mut rom_buffer)
            .map_err(|e| format!("Could not read ROM data from file: {}", e))?;
        let rom = gameboy_rom::GameBoyRom::new(rom_buffer.as_slice());
        let rom_header = rom.parse_header()?;

        info!("Loading game {}", rom_header.game_title);
        info!(
            "{} RAM banks of size {}",
            rom_header.ram_banks, rom_header.ram_bank_size
        );
        info!("{} 16KB ROM banks", rom_header.rom_size);
        info!("Cartridge type: {:?}", rom_header.rom_type);

        if rom_buffer.len() < 0x4000 {
            return Err(format!(
                "Suspicious ROM detected: ROM should be at least 0x4000 bytes, found {}",
                rom_buffer.len()
            ));
        }

        match rom_header.rom_type {
            RomType::RomOnly | RomType::RomRam | RomType::RomRamBattery => Ok(Cartridge {
                gbc: rom_header.gameboy_color.supports_color(),
                sgb: rom_header.super_gameboy,
                entire_rom_data: rom_buffer,
                cart_sub: Some(CartridgeSubType::RomOnly {
                    ram_bank: [0u8; 0x2000],
                }),
                dummy_value: 0,
            }),
            RomType::Mbc1 | RomType::Mbc1Ram | RomType::Mbc1RamBattery => {
                const RAM_BANK_SIZE: usize = 0x2000;
                // TODO: figure out why I had this as an assert before
                //debug_assert_eq!(RAM_BANK_SIZE, rom_header.ram_bank_size as usize);
                //let ram_active = rom_header.ram_banks > 0;
                Ok(Cartridge {
                    cart_sub: Some(CartridgeSubType::Mbc1 {
                        memory_model: Mbc1Type::SixteenEight,
                        ram_banks: vec![[0; RAM_BANK_SIZE]; rom_header.ram_banks as usize],
                        ram_active: false,
                        mem_bank_selector: 1,
                        ram_bank_selector: 0,
                        num_rom_banks: (rom_buffer.len() / 0x4000) as u32,
                    }),
                    gbc: rom_header.gameboy_color.supports_color(),
                    sgb: rom_header.super_gameboy,
                    dummy_value: 0,
                    entire_rom_data: rom_buffer,
                })
            }
            RomType::Mbc3
            | RomType::Mbc3Ram
            | RomType::Mbc3RamBattery
            | RomType::Mbc3TimerRamBattery => {
                const RAM_BANK_SIZE: usize = 0x2000;
                debug_assert!(RAM_BANK_SIZE == rom_header.ram_bank_size as usize);

                Ok(Cartridge {
                    cart_sub: Some(CartridgeSubType::Mbc3 {
                        ram_banks: vec![[0; RAM_BANK_SIZE]; rom_header.ram_banks as usize],
                        ram_active: false,
                        mem_bank_selector: 1,
                        ram_bank_selector: 0,
                    }),
                    gbc: rom_header.gameboy_color.supports_color(),
                    sgb: rom_header.super_gameboy,
                    entire_rom_data: rom_buffer,
                    dummy_value: 0,
                })
            }
            RomType::Mbc5
            | RomType::Mbc5Ram
            | RomType::Mbc5RamBattery
            | RomType::Mbc5Rumble
            | RomType::Mbc5RumbleSram
            | RomType::Mbc5RumbleSramBattery => {
                const RAM_BANK_SIZE: usize = 0x2000;
                // TODO: figure out why I had this as an assert before
                //assert_eq!(RAM_BANK_SIZE, rom_header.ram_bank_size as usize);

                Ok(Cartridge {
                    cart_sub: Some(CartridgeSubType::Mbc5 {
                        ram_banks: vec![[0; RAM_BANK_SIZE]; rom_header.ram_banks as usize],
                        ram_active: false,
                        mem_bank_selector: 1,
                        ram_bank_selector: 0,
                        num_rom_banks: (rom_buffer.len() / 0x4000) as u32,
                    }),
                    gbc: rom_header.gameboy_color.supports_color(),
                    sgb: rom_header.super_gameboy,
                    entire_rom_data: rom_buffer,
                    dummy_value: 0,
                })
            }
            otherwise => Err(format!("Cartridge type {:?} is not supported", otherwise)),
        }
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
                    0x0000..=0x1FFF => {
                        let ram_active = (value & 0xF) == 0b1010;
                        if ram_active {
                            debug!("MBC1: set RAM to active");
                        } else {
                            debug!("MBC1: set RAM to inactive");
                        }

                        *ra = ram_active;
                    }
                    // bank select
                    0x2000..=0x3FFF => {
                        let rom_bank = if (value & 0x1F) == 0 {
                            1
                        } else {
                            (value & 0x1F) as u32
                        };
                        debug!("MBC1: Switching to ROM bank {}", rom_bank);
                        *mbs = rom_bank
                    }
                    // TODO: selecting MSBs of ROM bank in 16/8 mode
                    0x4000..=0x5FFF => match *mm {
                        Mbc1Type::FourThirtytwo => {
                            debug!("MBC1 4-32: selecting ROM bank {}", value & 0x3);
                            *rbs = (value & 0x3) as u32;
                        }
                        Mbc1Type::SixteenEight => {
                            *rbs = (value & 0x3) as u32;
                            // TODO: review all this MBS stuff
                            /*
                            *mbs &= !(0x3 << 5);
                            *mbs |= ((value & 0x3) as u32) << 4;
                            *mbs %= rb.len() as u32;
                            if *mbs == 0 {
                                *mbs = 1;
                            }
                            debug!("MBC1 16-8: selecting ROM bank {}", *mbs);
                            */
                        }
                    },
                    // cartridge memory model select
                    0x6000..=0x7FFF => {
                        *mm = if (index & 1) == 1 {
                            debug!("MBC1: Switching to 4-32 mode");
                            // swap bits of mbs and rbs here
                            Mbc1Type::FourThirtytwo
                        } else {
                            debug!("MBC1: Switching to 16-8 mode");
                            // swap bits here
                            Mbc1Type::SixteenEight
                        }
                    }
                    _ => self[index] = value,
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
                    0x0000..=0x1FFF => {
                        let ram_active = (value & 0xA) == 0xA;
                        if ram_active {
                            debug!("MBC3: set RAM to active");
                        } else {
                            debug!("MBC3: set RAM to inactive");
                        }

                        *ra = ram_active;
                    }
                    // bank select
                    0x2000..=0x3FFF => {
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
            Some(CartridgeSubType::Mbc5 {
                mem_bank_selector: ref mut mbs,
                ram_bank_selector: ref mut rbs,
                ram_active: ref mut ra,
                ..
            }) => match index {
                0x0000..=0x1FFF => {
                    let ram_active = (value & 0xA) == 0xA;
                    if ram_active {
                        debug!("MBC5: set RAM to active");
                    } else {
                        debug!("MBC5: set RAM to inactive");
                    }

                    *ra = ram_active;
                }
                // bank select 1
                0x2000..=0x2FFF => {
                    *mbs &= !0xFF;
                    *mbs |= value as u32;
                    debug!("MBC5: Switching to ROM bank {}", *mbs);
                }
                // bank select 2
                0x3000..=0x3FFF => {
                    *mbs &= !0x100;
                    *mbs |= (value as u32 & 1) << 8;
                    debug!("MBC5: Switching to ROM bank {}", *mbs);
                }
                // ram select
                0x4000..=0x5FFF => {
                    *rbs = value as u32 & 0xF;
                    debug!("MBC5: Switching to RAM bank {}", *rbs);
                }
                0x6000..=0x7FFF => {
                    // nop
                }
                _ => self[index] = value,
                /*
                _ => debug!(
                    "Out of bounds write in MBC5: writing 0x{:X} to 0x:{:X}",
                    value, index
                ),
                */
            },
            Some(CartridgeSubType::RomOnly { .. }) | None | _ => self[index] = value,
        }

        //panic!("This cartridge type does not provide RAM")
    }
}

#[derive(Clone)]
pub enum CartridgeSubType {
    RomOnly {
        ram_bank: [byte; 0x2000],
    },
    Mbc1 {
        memory_model: Mbc1Type,
        //memory_banks: [byte; 0x4000], //(2 << 13) + (2 << 21) - 0x4000],
        //memory_banks: Vec<[byte; 0x4000]>,
        ram_banks: Vec<[byte; 0x2000]>,
        ram_active: bool,
        // calculated from the rom size, just a cached division
        num_rom_banks: u32,
        //top two bits (21 & 22?) used for selecting RAM in 4_32 mode
        mem_bank_selector: u32,
        ram_bank_selector: u32,
    },
    Mbc3 {
        //unclear if this has 16-8/4-32 mode....
        //memory_banks: Vec<[byte; 0x4000]>,
        ram_banks: Vec<[byte; 0x2000]>,
        ram_active: bool, //unsure if this is needed
        mem_bank_selector: u32,
        ram_bank_selector: u32,
    },
    Mbc5 {
        ram_banks: Vec<[byte; 0x2000]>,
        ram_active: bool,
        num_rom_banks: u32,
        mem_bank_selector: u32,
        ram_bank_selector: u32,
    },
}

#[derive(Clone, Copy)]
pub enum Mbc1Type {
    SixteenEight,
    FourThirtytwo,
}

//for reading and writing
impl IndexMut<u16> for Cartridge {
    fn index_mut(&mut self, ind: u16) -> &mut byte {
        trace!("indexmut: {:X}", ind);
        match ind {
            0x0000..=0x3FFF => {
                // constructor guarantees this to be true
                debug_assert!(self.entire_rom_data.len() >= 0x4000);
                &mut self.entire_rom_data[ind as usize]
            }
            0x4000..=0x7FFF => {
                match self.cart_sub {
                    Some(CartridgeSubType::RomOnly { .. }) => {
                        // TODO: investigate generated code and remove bounds checks/verify this
                        // invariant in the constructor if present
                        &mut self.entire_rom_data[ind as usize]
                    }
                    Some(CartridgeSubType::Mbc1 { memory_model: Mbc1Type::SixteenEight,
                                                  mem_bank_selector: bank_selector,
                                                  num_rom_banks,
                                             .. }) => {
                        let adjusted_bank_selector = if bank_selector == 0 { 1 } else { bank_selector as usize } - 1;
                        let m = (0x4000 * num_rom_banks as usize) - 1;
                        let idx = ((adjusted_bank_selector * 0x4000) + ind as usize) & m;
                        &mut self.entire_rom_data[idx]
                    }
                    Some(CartridgeSubType::Mbc1 { .. /*memory_model: Mbc1Type::FourThirtytwo,
                                             memory_banks: ref mb,
                                             //ram_active: ra,
                                             mem_bank_selector: index*/ }) => unimplemented!(),
                    Some(CartridgeSubType::Mbc5 {  mem_bank_selector: bank_selector,
                                                  num_rom_banks,
                                             .. }) => {
                        let m = (0x4000 * num_rom_banks as usize) - 1;
                        let idx = ((bank_selector as usize * 0x4000) + ind as usize) & m;
                        &mut self.entire_rom_data[idx]
                    }

                    _ => unimplemented!(),
                }
            }
            // Video RAM:
            0x8000..=0x9FFF => {
                //TODO: block needs to handle if reads should be blocked
                panic!("At access video ram");
            }
            // switchable RAM bank
            0xA000..=0xBFFF => {
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
                        ram_active: true,
                        ram_bank_selector: rbs,
                        ..
                    }) => {
                        if (rbs as usize) < rb.len() {
                            &mut rb[rbs as usize][(ind - 0xA000) as usize]
                        } else {
                            &mut self.dummy_value
                        }
                    }
                    Some(CartridgeSubType::Mbc1 {
                        ram_active: false, ..
                    }) => &mut self.dummy_value,
                    Some(CartridgeSubType::Mbc5 {
                        ram_banks: ref mut rb,
                        ram_active: true,
                        ram_bank_selector: rbs,
                        ..
                    }) => {
                        if (rbs as usize) < rb.len() {
                            &mut rb[rbs as usize][(ind - 0xA000) as usize]
                        } else {
                            &mut self.dummy_value
                        }
                    }
                    Some(CartridgeSubType::Mbc5 {
                        ram_active: false, ..
                    }) => &mut self.dummy_value,
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

static NO_RAM_BUS_NOISE: u8 = 0xFF;

//for reading
impl Index<u16> for Cartridge {
    type Output = byte;

    fn index<'a>(&'a self, ind: u16) -> &'a byte {
        match ind {
            0x0000..=0x3FFF => {
                // constructor guarantees this to be true
                debug_assert!(self.entire_rom_data.len() >= 0x4000);
                &self.entire_rom_data[ind as usize]
            }
            0x4000..=0x7FFF => match self.cart_sub {
                Some(CartridgeSubType::RomOnly { .. }) => &self.entire_rom_data[ind as usize],
                Some(CartridgeSubType::Mbc1 {
                    memory_model: Mbc1Type::SixteenEight,
                    mem_bank_selector: bank_selector,
                    num_rom_banks,
                    ..
                }) => {
                    let adjusted_bank_selector = if bank_selector == 0 {
                        1
                    } else {
                        bank_selector as usize
                    } - 1;
                    let m = (0x4000 * num_rom_banks as usize) - 1;
                    let idx = ((adjusted_bank_selector * 0x4000) + ind as usize) & m;
                    &self.entire_rom_data[idx]
                }
                Some(CartridgeSubType::Mbc1 {
                    memory_model: Mbc1Type::FourThirtytwo,
                    mem_bank_selector: bank_selector,
                    ..
                }) => {
                    let adjusted_bank_selector = if bank_selector == 0 {
                        1
                    } else {
                        bank_selector as usize
                    } - 1;
                    &self.entire_rom_data[(adjusted_bank_selector * 0x4000) + ind as usize]
                }
                Some(CartridgeSubType::Mbc3 {
                    mem_bank_selector: bank_selector,
                    ..
                }) => {
                    let adjusted_bank_selector = if bank_selector == 0 {
                        1
                    } else {
                        bank_selector as usize
                    } - 1;
                    &self.entire_rom_data[(adjusted_bank_selector * 0x4000) + ind as usize]
                }
                Some(CartridgeSubType::Mbc5 {
                    mem_bank_selector: bank_selector,
                    //num_rom_banks,
                    ..
                }) => {
                    //let m = (0x4000 * num_rom_banks as usize) - 1;
                    let idx = /*(*/(bank_selector as usize * 0x4000) + ind as usize; // ) & m;
                    &self.entire_rom_data[idx]
                }
                _ => panic!("Indexing {:X}", ind),
            },
            // Video RAM:
            0x8000..=0x9FFF => {
                //TODO: block needs to handle if reads should be blocked
                panic!("At access video ram");
            }
            // switchable RAM bank
            0xA000..=0xBFFF => match self.cart_sub {
                Some(CartridgeSubType::RomOnly {
                    ram_bank: ref rambank,
                    ..
                }) => &rambank[(ind - 0xA000) as usize],
                Some(CartridgeSubType::Mbc1 {
                    ram_active: true,
                    ram_banks: ref ram_vec,
                    ram_bank_selector: rbs,
                    ..
                }) => {
                    if (rbs as usize) < ram_vec.len() {
                        &ram_vec[rbs as usize][(ind as u32 - 0xA000) as usize]
                    } else {
                        &NO_RAM_BUS_NOISE
                    }
                }
                Some(CartridgeSubType::Mbc3 {
                    ram_banks: ref ram_vec,
                    ram_bank_selector: rbs,
                    ..
                }) => &ram_vec[rbs as usize][(ind as u32 - 0xA000) as usize],
                Some(CartridgeSubType::Mbc5 {
                    ram_active: true,
                    ram_banks: ref ram_vec,
                    ram_bank_selector: rbs,
                    ..
                }) => {
                    if (rbs as usize) < ram_vec.len() {
                        &ram_vec[rbs as usize][(ind as u32 - 0xA000) as usize]
                    } else {
                        &NO_RAM_BUS_NOISE
                    }
                }
                Some(CartridgeSubType::Mbc1 {
                    ram_active: false, ..
                }) => &NO_RAM_BUS_NOISE,
                Some(CartridgeSubType::Mbc5 {
                    ram_active: false, ..
                }) => &NO_RAM_BUS_NOISE,
                _ => panic!("at switchable ram bank"),
            },
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
                for b in ra {
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
            entire_rom_data: vec![0; 0x4000],
            cart_sub: None,
            gbc: false,
            sgb: false,
            dummy_value: 0,
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
