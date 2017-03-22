//! The memory logic
//! Contains all "Virtual memory" handling, memory bank controlling, and otherwise
//! Its behavior is dictated by the cartridge, but is separate

use std::ops::{Index, IndexMut};
use std::iter::Iterator;

use cpu::cartridge::*;
use cpu::constants::*;

pub struct Memory {
    cartridge: Cartridge,

    /// 8kb video ram
    /// 0x8000-0x9FFF
    video_ram: [byte; 0x2000],

    /// 8kb internal ram
    /// 0xC000-0xDFFF (second half needs to be switchable in CBG)
    internal_ram: [byte; 0x2000],

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
}

impl Memory {
    pub fn new() -> Memory {
        Memory {
            cartridge: Cartridge::new(),
            video_ram: [0u8; 0x2000],
            internal_ram: [0u8; 0x2000],
            oam: [0u8; 0xA0],
            io_ports: [0u8; 0x80],
            hram: [0u8; 0x80],
            interrupt_flag: 0,
        }
    }

    pub fn load(&mut self, input_file: &str) {
        self.cartridge = Cartridge::load(input_file);
    }
}

impl Index<usize> for Memory {
    type Output = byte;

    fn index(&self, index: usize) -> &byte {
        unimplemented!();
    }
}

impl IndexMut<usize> for Memory {
    fn index_mut(&mut self, index: usize) -> &mut byte {
        unimplemented!();
    }
}


impl Iterator for Memory {
    type Item = byte;

    fn next(&mut self) -> Option<Self::Item> {
        unimplemented!()
    }
}

impl Clone for Memory {
    fn clone(&self) -> Memory {
        unimplemented!()
    }
}
