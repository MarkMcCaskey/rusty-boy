//! The memory logic
//! Contains all "Virtual memory" handling, memory bank controlling, and otherwise
//! Its behavior is dictated by the cartridge, but is separate

use std::ops::{Index, IndexMut};
use std::iter::Iterator;

use cpu::cartridge::*;
use cpu::constants::*;
use cpu::memvis::cpumemvis::*;

pub struct Memory {
    cartridge: Box<Cartridge>,

    /// 8kb video ram
    /// 0x8000-0x9FFF
    video_ram: [byte; 0x2000],

    /// 8kb internal ram
    /// 0xC000-0xDFFF (second half needs to be switchable in CBG)
    internal_ram: [byte; 0x2000],

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
            video_ram: [0u8; 0x2000],
            internal_ram: [0u8; 0x2000],
            empty: [0u8; 0x6F],
            oam: [0u8; 0xA0],
            io_ports: [0u8; 0x80],
            hram: [0u8; 0x80],
            interrupt_flag: 0,
            iterator_index: 0,
            logger: Some(DeqCpuEventLogger::new(None)),
        }
    }

    pub fn write_ram_value(&mut self, index: u16, value: byte) {
        match index {
            0x0000...0x7FFF | 0xA000...0xBFFF => self.cartridge.write_ram_value(index, value),
            n => self[index as usize] = value,
        }
    }

    pub fn load(&mut self, input_file: &str) {
        *self.cartridge = Cartridge::load(input_file);
    }

    pub fn initialize_logger(&mut self) {
        let mut mem_buffer: [byte; 0x10000] = [0u8; 0x10000];
        for index in 0..0xFFFF {
            mem_buffer[index] = match index {
                0x0000...0x7FFF => *self.cartridge.index(index as u16), //self.cartridge[index as u16],
                0x8000...0x9FFF => self.video_ram[index - 0x8000],
                0xC000...0xDFFF => self.internal_ram[index - 0xC000],
                0xE000...0xFDFF => self.internal_ram[index - 0xE000],
                0xFE00...0xFE9F => self.oam[index - 0xFE00],
                0xFEA0...0xFEFF => self.empty[index - 0xFEA0],
                0xFF00...0xFF7F => self.io_ports[index - 0xFF00],
                0xFF80...0xFFFE => self.hram[index - 0xFF80],
                0xFFFF => self.interrupt_flag,
                _ => 0,

            };
        }
        self.logger = Some(DeqCpuEventLogger::new(Some(&mem_buffer[..])));
    }

    pub fn reset(&mut self) {
        self[0xFF05] = 0x00;
        self[0xFF06] = 0x00;
        self[0xFF07] = 0x00;
        self[0xFF10] = 0x80;
        self[0xFF11] = 0xBF;
        self[0xFF12] = 0xF3;
        self[0xFF14] = 0xBF;
        self[0xFF16] = 0x3F;
        self[0xFF17] = 0x00;
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
        self[0xFF26] = 0xF1; //F1 for GB // TODOA:
        self[0xFF40] = 0x91;
        self[0xFF42] = 0x00;
        self[0xFF43] = 0x00;
        self[0xFF45] = 0x00;
        self[0xFF47] = 0xFC;
        self[0xFF48] = 0xFF;
        self[0xFF49] = 0xFF;
        self[0xFF4A] = 0x00;
        self[0xFF4B] = 0x00;
        self[0xFFFF] = 0x00;

    }
}

impl Index<usize> for Memory {
    type Output = byte;

    fn index(&self, index: usize) -> &byte {
        //TODO: figure out why it's being indexed too high
        match index % 0x10000 {
            0x0000...0x7FFF | 0xA000...0xBFFF => self.cartridge.index(index as u16), //self.cartridge[index as u16],
            0x8000...0x9FFF => &self.video_ram[index - 0x8000],
            0xC000...0xDFFF => &self.internal_ram[index - 0xC000],
            0xE000...0xFDFF => &self.internal_ram[index - 0xE000],
            0xFE00...0xFE9F => &self.oam[index - 0xFE00],
            0xFEA0...0xFEFF => &self.empty[index - 0xFEA0],
            0xFF00...0xFF7F => &self.io_ports[index - 0xFF00],
            0xFF80...0xFFFE => &self.hram[index - 0xFF80],
            0xFFFF => &self.interrupt_flag,
            _ => panic!("Address 0x{:X} is out of bounds!", index),
        }
    }
}

impl IndexMut<usize> for Memory {
    fn index_mut(&mut self, index: usize) -> &mut byte {
        match index % 0x10000 {
            0x0000...0x7FFF | 0xA000...0xBFFF => &mut self.cartridge[index as u16],
            0x8000...0x9FFF => &mut self.video_ram[index - 0x8000],
            0xC000...0xDFFF => &mut self.internal_ram[index - 0xC000],
            0xE000...0xFDFF => &mut self.internal_ram[index - 0xE000],
            0xFE00...0xFE9F => &mut self.oam[index - 0xFE00],
            0xFEA0...0xFEFF => &mut self.empty[index - 0xFEA0],
            0xFF00...0xFF7F => &mut self.io_ports[index - 0xFF00],
            0xFF80...0xFFFE => &mut self.hram[index - 0xFF80],
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

impl Clone for Memory {
    fn clone(&self) -> Memory {
        trace!("Cloning memory");

        let mut vram: [u8; 0x2000] = [0u8; 0x2000];
        let mut iram: [u8; 0x2000] = [0u8; 0x2000];
        let mut oam: [u8; 0xA0] = [0u8; 0xA0];
        let mut hram: [u8; 0x80] = [0u8; 0x80];
        let mut io_ports: [u8; 0x80] = [0u8; 0x80];

        for (i, &vram_val) in self.video_ram
                .iter()
                .enumerate()
                .take(0x2000) {
            vram[i] = vram_val;
        }

        for (i, &internal_ram_val) in
            self.internal_ram
                .iter()
                .enumerate()
                .take(0x2000) {
            iram[i] = internal_ram_val;
        }

        for (i, &oam_val) in self.oam
                .iter()
                .enumerate()
                .take(0xA0) {
            oam[i] = oam_val;
        }

        for (i, &hram_val) in self.hram
                .iter()
                .enumerate()
                .take(0x80) {
            hram[i] = hram_val;
        }

        for (i, &io_port_val) in
            self.io_ports
                .iter()
                .enumerate()
                .take(0x80) {
            io_ports[i] = io_port_val;
        }

        Memory {
            cartridge: self.cartridge.clone(),
            video_ram: vram,
            internal_ram: iram,
            empty: [0u8; 0x6F],
            oam: oam,
            io_ports: io_ports,
            hram: hram,
            interrupt_flag: self.interrupt_flag,
            iterator_index: self.iterator_index,
            logger: self.logger.clone(),
        }
    }
}
