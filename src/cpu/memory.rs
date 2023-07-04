//! The memory logic
//! Contains all "Virtual memory" handling, memory bank controlling, and otherwise
//! Its behavior is dictated by the cartridge, but is separate

use std::iter::Iterator;
use std::ops::{Index, IndexMut};
use std::path::PathBuf;

use crate::cpu::cartridge::*;
use crate::cpu::constants::*;
use crate::cpu::memvis::cpumemvis::*;

#[derive(Clone)]
pub struct Memory {
    cartridge: Box<Cartridge>,

    /// 8kb video ram
    /// 0x8000-0x9FFF
    pub video_ram: [[byte; 0x2000]; 2],

    pub gbc_vram_bank: u8,

    /// 8kb internal ram
    /// 0xC000-0xDFFF (second half needs to be switchable in CBG)
    internal_ram: [[byte; 0x1000]; 8],

    pub gbc_wram_bank: u8,

    /// unusable values
    /// 0xFEA0-0xFF00
    empty: [byte; 0x6F],

    /// sprite attribute memory
    /// 0xFE00-0xF9F
    oam: [byte; 0xA0],

    /// IO ports
    /// 0xFF00-0xFF7F
    io_ports: [byte; 0x80],

    /// High RAM
    /// 0xFF80-0xFFFF
    hram: [byte; 0x80],

    /// Interrupt enable flag
    /// 0xFFFF
    interrupt_flag: byte,

    iterator_index: u16,

    pub logger: Option<DeqCpuEventLogger>,
}

impl Default for Memory {
    fn default() -> Self {
        Self::new()
    }
}

impl Memory {
    pub fn new() -> Memory {
        Memory {
            cartridge: Box::new(Cartridge::new()),
            video_ram: [[0u8; 0x2000]; 2],
            internal_ram: [[0u8; 0x1000]; 8],
            empty: [0u8; 0x6F],
            oam: [0u8; 0xA0],
            io_ports: [0u8; 0x80],
            hram: [0u8; 0x80],
            interrupt_flag: 0,
            iterator_index: 0,
            logger: Some(DeqCpuEventLogger::new(None)),
            // default to 1 so normal gameboy works as expected
            gbc_wram_bank: 1,
            gbc_vram_bank: 0,
        }
    }

    pub fn write_ram_value(&mut self, index: u16, value: byte) {
        match index {
            0x0000..=0x7FFF | 0xA000..=0xBFFF => self.cartridge.write_ram_value(index, value),
            _ => self[index as usize] = value,
        }
    }

    pub fn load(&mut self, input_file: &str) {
        *self.cartridge = Cartridge::load(input_file).expect("Could not load ROM");
    }

    pub fn load_saved_ram(&mut self, mut path: PathBuf, game_name: &str) {
        path.push(game_name);
        path.set_extension("savedgame");

        self.cartridge.load_ram(&path);
    }

    pub fn save_ram(&self, data_path: PathBuf, game_name: &str) {
        let mut path = data_path.clone();

        path.push(game_name);
        path.set_extension("savedgame");

        self.cartridge.save_ram(&data_path);
    }

    pub fn gbc_mode(&self) -> bool {
        self.cartridge.gbc
    }

    pub fn sgb_mode(&self) -> bool {
        self.cartridge.sgb
    }

    pub fn set_gbc_mode(&mut self) {
        self.gbc_wram_bank = 0;
    }

    pub fn initialize_logger(&mut self) {
        let mut mem_buffer: [byte; 0x1_0000] = [0u8; 0x1_0000];
        for index in 0..0xFFFF {
            mem_buffer[index] = match index {
                0x0000..=0x7FFF | 0xA000..=0xBFFF => *self.cartridge.index(index as u16), //self.cartridge[index as u16],
                0x8000..=0x9FFF => self.video_ram[self.gbc_vram_bank as usize][index - 0x8000],
                0xC000..=0xCFFF => self.internal_ram[0][index - 0xC000],
                0xD000..=0xDFFF => self.internal_ram[self.gbc_wram_bank as usize][index - 0xD000],
                0xE000..=0xEFFF => self.internal_ram[0][index - 0xE000],
                0xF000..=0xFDFF => self.internal_ram[self.gbc_wram_bank as usize][index - 0xF000],
                0xFE00..=0xFE9F => self.oam[index - 0xFE00],
                0xFEA0..=0xFEFF => self.empty[index - 0xFEA0],
                0xFF00..=0xFF7F => self.io_ports[index - 0xFF00],
                0xFF80..=0xFFFE => self.hram[index - 0xFF80],
                0xFFFF => self.interrupt_flag,
                _ => 0,
            };
        }
        self.logger = Some(DeqCpuEventLogger::new(Some(&mem_buffer[..])));
    }

    pub fn reset(&mut self, sgb_mode: bool) {
        self[0xFF00] = 0xCF;
        self[0xFF01] = 0x00;
        self[0xFF02] = 0x7E;
        self[0xFF04] = 0xAB;
        self[0xFF05] = 0x00;
        self[0xFF06] = 0x00;
        self[0xFF07] = 0xF8;
        self[0xFF0F] = 0xE1;
        self[0xFF10] = 0x80;
        self[0xFF11] = 0xBF;
        self[0xFF12] = 0xF3;
        self[0xFF13] = 0xFF;
        self[0xFF14] = 0xBF;
        self[0xFF16] = 0x3F;
        self[0xFF17] = 0x00;
        self[0xFF18] = 0xFF;
        self[0xFF19] = 0xBF;
        self[0xFF1A] = 0x7F;
        self[0xFF1B] = 0xFF;
        self[0xFF1C] = 0x9F;
        self[0xFF1E] = 0xBF;
        self[0xFF20] = 0xFF;
        self[0xFF21] = 0x00;
        self[0xFF22] = 0x00;
        self[0xFF23] = 0xBF;
        self[0xFF24] = 0x77;
        self[0xFF25] = 0xF3;
        self[0xFF26] = if sgb_mode { 0xF0 } else { 0xF1 };
        self[0xFF40] = 0x91;
        self[0xFF41] = 0x85;
        self[0xFF42] = 0x00;
        self[0xFF43] = 0x00;
        self[0xFF44] = 0x00;
        self[0xFF45] = 0x00;
        self[0xFF46] = 0xFF;
        self[0xFF47] = 0xFC;
        self[0xFF48] = 0xFF;
        self[0xFF49] = 0xFF;
        self[0xFF4A] = 0x00;
        self[0xFF4B] = 0x00;
        self[0xFFFF] = 0x00;
    }
}

impl Index<u16> for Memory {
    type Output = u8;

    fn index(&self, index: u16) -> &u8 {
        match index {
            0x0000..=0x7FFF | 0xA000..=0xBFFF => self.cartridge.index(index), //self.cartridge[index as u16],
            0x8000..=0x9FFF => {
                &self.video_ram[self.gbc_vram_bank as usize][(index - 0x8000) as usize]
            }
            0xC000..=0xCFFF => &self.internal_ram[0][(index - 0xC000) as usize],
            0xD000..=0xDFFF => {
                &self.internal_ram[self.gbc_wram_bank as usize][(index - 0xD000) as usize]
            }
            0xE000..=0xEFFF => {
                &self.internal_ram[self.gbc_wram_bank as usize][(index - 0xE000) as usize]
            }
            0xF000..=0xFDFF => {
                &self.internal_ram[self.gbc_wram_bank as usize][(index - 0xF000) as usize]
            }
            0xFE00..=0xFE9F => &self.oam[(index - 0xFE00) as usize],
            0xFEA0..=0xFEFF => &self.empty[(index - 0xFEA0) as usize],
            0xFF00..=0xFF7F => &self.io_ports[(index - 0xFF00) as usize],
            0xFF80..=0xFFFE => &self.hram[(index - 0xFF80) as usize],
            0xFFFF => &self.interrupt_flag,
        }
    }
}

impl Index<usize> for Memory {
    type Output = byte;

    fn index(&self, index: usize) -> &byte {
        //TODO: figure out why it's being indexed too high
        &self[(index % 0x1_0000) as u16]
    }
}

impl IndexMut<usize> for Memory {
    fn index_mut(&mut self, index: usize) -> &mut byte {
        match index % 0x1_0000 {
            0x0000..=0x7FFF | 0xA000..=0xBFFF => &mut self.cartridge[index as u16],
            0x8000..=0x9FFF => unsafe {
                self.video_ram
                    .get_unchecked_mut(self.gbc_vram_bank as usize)
                    .get_unchecked_mut(index - 0x8000)
            },
            0xC000..=0xCFFF => &mut self.internal_ram[0][index - 0xC000],
            0xD000..=0xDFFF => &mut self.internal_ram[self.gbc_wram_bank as usize][index - 0xD000],
            0xE000..=0xEFFF => &mut self.internal_ram[0][index - 0xE000],
            0xF000..=0xFDFF => &mut self.internal_ram[self.gbc_wram_bank as usize][index - 0xF000],
            0xFE00..=0xFE9F => &mut self.oam[index - 0xFE00],
            0xFEA0..=0xFEFF => &mut self.empty[index - 0xFEA0],
            0xFF00..=0xFF7F => {
                match index {
                    // link port synchronization
                    // hack for now
                    0xFF02 => {
                        //self.io_ports[0x02] = 0x7E;
                        // interrupt handler should be called, but let's try not doing it for now
                        print!("{}", self.io_ports[0x01] as char);
                        &mut self.io_ports[0x02]
                    }
                    _ => &mut self.io_ports[index - 0xFF00],
                }
            }
            0xFF80..=0xFFFE => &mut self.hram[index - 0xFF80],
            0xFFFF => &mut self.interrupt_flag,
            _ => panic!("Address out of bounds!"),
        }
    }
}

impl Iterator for Memory {
    type Item = byte;

    fn next(&mut self) -> Option<Self::Item> {
        if self.iterator_index < u16::max_value() {
            self.iterator_index += 1;
            Some(self[(self.iterator_index - 1) as usize])
        } else {
            None
        }
    }
}
