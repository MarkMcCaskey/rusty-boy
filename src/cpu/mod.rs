//! The hardware emulation logic

#![allow(dead_code)]

#[macro_use]
mod macros;
pub mod cartridge;
pub mod constants;
pub mod memory;
pub mod memvis;
mod tests;

use std::num::Wrapping;
use std::path::PathBuf;

use self::constants::*;
use self::memory::*;
use self::memvis::cpumemvis::*;
use crate::disasm::*;

#[inline]
pub fn byte_to_u16(low_byte: u8, high_byte: u8) -> u16 {
    ((high_byte as u16) << 8) | (low_byte as u16)
}

#[inline]
pub fn add_u16_i8(word: u16, sbyte: i8) -> u16 {
    //((word as i16) + ((sbyte as i8) as i16)) as u16
    (Wrapping(word as i32) + Wrapping(sbyte as i32)).0 as u16
}

/// The CPU itself.
///
///Currently contains memory (including the ROM (which is not
/// read-only!), which should be abstracted later)
pub struct Cpu {
    a: byte,
    b: byte,
    c: byte,
    d: byte,
    e: byte,
    //NOTE: bit 7: zero flag; bit 6: subtract flag; bit 5: half carry; bit 4: carry flag
    pub f: byte,
    h: byte,
    l: byte,
    sp: MemAddr,
    // Interrupt Master Enable flag (aka "Interrupt Flip-Flop")
    ime: bool,
    pub pc: MemAddr,

    /// Handles all memory read/writes
    pub mem: Memory,

    /// Whether or not the CPU is running, waiting for input, or stopped
    pub state: CpuState,

    // State of all buttons (low is pressed)
    input_state: u8,

    /*    /// Log of events, used in `MemVis`
    pub event_logger: Option<DeqCpuEventLogger>,*/
    /// TODO: document this
    pub cycles: CycleCount,
    interrupt_next_inst: bool,
    pub gbc_mode: bool,
    sgb_mode: bool,
    /// for CGB, run at double speed
    pub double_speed: bool,
}

/// Used for save-states and reverting to old CPU on resets
impl Clone for Cpu {
    fn clone(&self) -> Cpu {
        Cpu {
            a: self.a,
            b: self.b,
            c: self.c,
            d: self.d,
            e: self.e,
            f: self.f,
            h: self.h,
            l: self.l,
            sp: self.sp,
            ime: self.ime,
            pc: self.pc,
            mem: self.mem.clone(),
            state: self.state,
            input_state: self.input_state,

            //event_logger: self.event_logger.clone(),
            cycles: self.cycles,
            interrupt_next_inst: false,
            gbc_mode: self.gbc_mode,
            sgb_mode: self.sgb_mode,
            double_speed: false,
        }
    }
}

impl Default for Cpu {
    fn default() -> Self {
        Self::new()
    }
}

impl Cpu {
    pub fn new() -> Cpu {
        let mut new_cpu = Cpu {
            // TODO: abstract this later~
            a: 0x01, //for GB/SGB (GBP & GBC need different values)
            b: 0,
            c: 0,
            d: 0,
            e: 0,
            f: 0xB0,
            h: 0,
            l: 0,
            sp: 0xFFFE,
            ime: true, // TODO verify initial
            pc: 0,
            mem: Memory::new(),
            state: CpuState::Normal,
            input_state: 0xFF,

            //event_logger: None,//Some(DeqCpuEventLogger::new(None)),
            cycles: 0,
            interrupt_next_inst: false,
            gbc_mode: false,
            sgb_mode: false,
            double_speed: false,
        };

        // The reset state is the default state of the CPU
        new_cpu.reset();

        new_cpu
    }

    /// Sets the CPU to as it would be after the boot rom has executed
    pub fn reset(&mut self) {
        self.state = CpuState::Normal;
        self.a = if self.gbc_mode { 0x11 } else { 0x01 }; // gameboy pocket needs 0xFF
        self.b = 0;
        self.c = if self.gbc_mode { 0 } else { 0x13 };
        self.d = if self.gbc_mode { 0xFF } else { 0 };
        self.e = if self.gbc_mode { 0x56 } else { 0xD8 };
        self.f = if self.gbc_mode { 0x80 } else { 0xB0 };
        self.h = if self.gbc_mode { 0 } else { 0x01 };
        self.l = if self.gbc_mode { 0x0D } else { 0x4D };
        self.sp = 0xFFFE;
        self.pc = 0x100;
        self.cycles = 0;

        info!("reset {}", if self.gbc_mode { "GBC" } else { "GB" });
        self.mem.reset(self.sgb_mode);
    }

    pub fn reinit_logger(&mut self) {
        let mut mem_cpy = [0u8; 0xFFFF];

        for i in 0..0xFFFF {
            mem_cpy[i] = self.mem[i];
        }

        self.mem.logger = Some(DeqCpuEventLogger::new(Some(&mem_cpy[..])));
    }

    pub fn toggle_logger(&mut self) {
        match self.mem.logger {
            Some(_) => self.mem.logger = None,
            None => self.reinit_logger(),
        }
    }

    ///FF04 Div
    ///
    /// This needs to be called 16384 (~16779 on SGB) times a second
    ///
    pub fn inc_div(&mut self) {
        let old_val = self.mem[0xFF04_u16];
        self.mem[0xFF04] = old_val.wrapping_add(1);
    }
    pub fn get_div(&self) -> u8 {
        self.mem[0xFF04_u16]
    }

    /// The speed at which the timer runs, settable by the program by
    /// writing to 0xFF07
    pub fn timer_frequency_hz(&self) -> u32 {
        // NOTE these values differ for SGB
        match self.mem[0xFF07_u16] & 0x3 {
            0 => 4096,
            1 => 262_144,
            2 => 65_536,
            3 => 16_384,
            _ => unreachable!("The impossible happened!"),
        }
    }

    /* sound */
    pub fn channel1_sweep_time(&self) -> f32 {
        (((self.mem[0xFF10_u16] >> 4) & 0x7) as f32) / 128.0
    }

    // number is multiplied by 128 and is the hz of how often it's updated.
    pub fn channel1_sweep_pace(&self) -> u8 {
        (self.mem[0xFF10_u16] >> 4) & 0x7
    }

    pub fn channel1_sweep_increase(&self) -> bool {
        ((self.mem[0xFF10_u16] >> 3) & 1) == 0
    }

    pub fn channel1_sweep_shift(&self) -> u8 {
        self.mem[0xFF10_u16] & 0x7
    }

    // Runs the sweep logic
    pub fn channel1_sweep_step(&mut self) {
        let freq = self.channel1_frequency();
        let shift = self.channel1_sweep_shift();
        let new_value = if self.channel1_sweep_increase() {
            let n = freq + (freq >> shift);
            if n > 0x7FF {
                self.unset_sound1();
            }
            n & 0x7FF
        } else {
            freq - (freq >> shift)
        };
        self.mem[0xFF13] = (new_value & 0xFF) as u8;
        self.mem[0xFF14] &= !0x7;
        self.mem[0xFF14] |= ((new_value >> 8) & 0x7) as u8;
    }

    pub fn channel1_wave_pattern_duty(&self) -> f32 {
        match (self.mem[0xFF11_u16] >> 6) & 0x3 {
            0 => 0.125,
            1 => 0.25,
            2 => 0.5,
            3 => 0.75,
            _ => unreachable!(),
        }
    }

    pub fn channel1_sound_length(&self) -> u8 {
        self.mem[0xFF11_u16] & 0x3F
    }

    pub fn channel1_inc_sound_length(&mut self) {
        if !self.channel1_sound_length_enabled() {
            return;
        }
        let mut val = self.mem[0xFF11_u16] & 0x3F;
        val += 1;
        if val >= 64 {
            self.unset_sound1();
        }
        self.mem[0xFF11] &= !0x3F;
        self.mem[0xFF11] |= val & 0x3F;
    }

    pub fn channel1_envelope_volume(&self) -> u8 {
        (self.mem[0xFF12_u16] >> 4) & 0xF
    }

    pub fn channel1_step_envelope(&mut self) {
        let val = self.channel1_envelope_volume();
        let new_val = if self.channel1_envelope_increasing() {
            let n = val + 1;
            if n >= 0xF {
                0xF
            } else {
                n
            }
        } else {
            if val == 0 {
                0
            } else {
                val - 1
            }
        };
        self.mem[0xFF12] &= !0xF0;
        self.mem[0xFF12] |= new_val << 4;
    }

    pub fn channel1_envelope_increasing(&self) -> bool {
        ((self.mem[0xFF12_u16] >> 3) & 0x1) == 1
    }

    pub fn channel1_envelope_sweep_pace(&self) -> u8 {
        self.mem[0xFF12_u16] & 0x7
    }

    pub fn channel1_frequency(&self) -> u16 {
        let lower = self.mem[0xFF13_u16];
        let higher = self.mem[0xFF14_u16] & 0x7;
        byte_to_u16(lower, higher)
    }

    pub fn channel1_sound_length_enabled(&self) -> bool {
        ((self.mem[0xFF14_u16] >> 6) & 1) == 1
    }

    pub fn channel2_wave_pattern_duty(&self) -> u8 {
        (self.mem[0xFF16_u16] >> 6) & 0x3
    }

    pub fn channel2_sound_length(&self) -> u8 {
        self.mem[0xFF16_u16] & 0x3F
    }

    pub fn channel2_inc_sound_length(&mut self) {
        if !self.channel2_sound_length_enabled() {
            return;
        }
        let mut val = self.mem[0xFF16_u16] & 0x3F;
        val += 1;
        if val >= 64 {
            self.unset_sound2();
        }
        self.mem[0xFF16] &= !0x3F;
        self.mem[0xFF16] |= val & 0x3F;
    }

    pub fn channel2_envelope_volume(&self) -> u8 {
        (self.mem[0xFF17_u16] >> 4) & 0xF
    }

    pub fn channel2_step_envelope(&mut self) {
        let val = self.channel2_envelope_volume();
        let new_val = if self.channel2_envelope_increasing() {
            let n = val + 1;
            if n >= 0xF {
                0xF
            } else {
                n
            }
        } else {
            if val == 0 {
                0
            } else {
                val - 1
            }
        };
        self.mem[0xFF17] &= !0xF0;
        self.mem[0xFF17] |= new_val << 4;
    }

    pub fn channel2_envelope_increasing(&self) -> bool {
        ((self.mem[0xFF17_u16] >> 3) & 0x1) == 1
    }

    pub fn channel2_envelope_sweep_pace(&self) -> u8 {
        self.mem[0xFF17_u16] & 0x7
    }

    pub fn channel2_frequency(&self) -> u16 {
        let lower = self.mem[0xFF18_u16];
        let higher = self.mem[0xFF19_u16] & 0x7;

        byte_to_u16(lower, higher)
    }

    pub fn channel2_sound_length_enabled(&self) -> bool {
        ((self.mem[0xFF19_u16] >> 6) & 1) == 1
    }

    pub fn channel3_on(&self) -> bool {
        ((self.mem[0xFF1A_u16] >> 7) & 1) == 1
    }

    pub fn channel3_sound_length(&self) -> u8 {
        self.mem[0xFF1B_u16]
    }

    pub fn channel3_inc_sound_length(&mut self) {
        // REVIEW: do we care about DAC here?
        if !self.channel3_sound_length_enabled()
        /*|| self.mem[0xFF1A_u16] >> 7 == 0*/
        {
            return;
        }
        let mut val = self.mem[0xFF1B_u16];
        if val == 0xFF {
            val = 0;
            self.unset_sound3();
        } else {
            val += 1;
        }
        self.mem[0xFF1B] = val;
    }

    pub fn channel3_output_level(&self) -> f32 {
        match (self.mem[0xFF1C_u16] >> 5) & 0x3 {
            0 => 0.0,
            1 => 1.0,
            2 => 0.5,
            3 => 0.25,
            _ => unreachable!(),
        }
    }

    pub fn channel3_shift_amount(&self) -> u8 {
        match (self.mem[0xFF1C_u16] >> 5) & 0x3 {
            0 | 1 => 0,
            2 => 1,
            3 => 2,
            _ => unreachable!(),
        }
    }

    pub fn channel3_frequency(&self) -> u16 {
        let lower = self.mem[0xFF1D_u16];
        let higher = self.mem[0xFF1E_u16] & 0x7;

        byte_to_u16(lower, higher)
    }

    pub fn channel3_sound_length_enabled(&self) -> bool {
        ((self.mem[0xFF1E_u16] >> 6) & 1) == 1
    }

    pub fn channel3_wave_pattern_ram(&self) -> [u8; 32] {
        let mut ret = [0u8; 32];
        for i in 0..32 {
            ret[i] = (self.mem[0xFF30 + (i / 2)] >> (((i + 1) % 2) * 4)) & 0xF;
        }

        ret
    }

    pub fn channel4_sound_length(&self) -> u8 {
        self.mem[0xFF20_u16] & 0x3F
    }

    pub fn channel4_inc_sound_length(&mut self) {
        if !self.channel4_sound_length_enabled() {
            return;
        }
        let mut val = self.mem[0xFF20_u16] & 0x3F;
        val += 1;
        if val >= 64 {
            self.unset_sound4();
        }
        self.mem[0xFF20] &= !0x3F;
        self.mem[0xFF20] |= val & 0x3F;
    }

    pub fn channel4_sound_length_enabled(&self) -> bool {
        ((self.mem[0xFF23_u16] >> 6) & 1) == 1
    }

    pub fn channel4_envelope_volume(&self) -> u8 {
        (self.mem[0xFF21_u16] >> 4) & 0xF
    }

    pub fn channel4_step_envelope(&mut self) {
        let val = self.channel4_envelope_volume();
        let new_val = if self.channel4_envelope_increasing() {
            let n = val + 1;
            if n >= 0xF {
                0xF
            } else {
                n
            }
        } else {
            if val == 0 {
                0
            } else {
                val - 1
            }
        };
        self.mem[0xFF21] &= !0xF0;
        self.mem[0xFF21] |= new_val << 4;
    }

    pub fn channel4_envelope_increasing(&self) -> bool {
        ((self.mem[0xFF21_u16] >> 3) & 0x1) == 1
    }

    pub fn channel4_envelope_sweep_pace(&self) -> u8 {
        self.mem[0xFF21_u16] & 0x7
    }

    /// Abstracts the logic of the timer
    /// Call this from the loop when the timer should be incremented
    /// NOTE: does not appear to take timer frequency into account...
    pub fn timer_cycle(&mut self) {
        if self.is_timer_on() {
            self.inc_timer();
        }
    }

    fn is_timer_on(&self) -> bool {
        (self.mem[0xFF07_u16] & 0x4) >> 2 == 1
    }

    fn inc_timer(&mut self) {
        let old_val = self.mem[0xFF05_u16];
        // TMA; value which is to be set on overflow
        let new_val = self.mem[0xFF06_u16];

        self.mem[0xFF05] = if old_val.wrapping_add(1) == 0 {
            // on overflow...
            self.set_timer_interrupt_bit();
            // this seems wrong, but things break without it
            if self.state == CpuState::Halt {
                self.state = CpuState::Normal;
            }
            new_val
        } else {
            old_val.wrapping_add(1)
        };
    }

    set_interrupt_bit!(set_vblank_interrupt_bit, 0x1);
    set_interrupt_bit!(set_lcdc_interrupt_bit, 0x2);
    set_interrupt_bit!(set_timer_interrupt_bit, 0x4);
    set_interrupt_bit!(set_serial_io_interrupt_bit, 0x8);
    set_interrupt_bit!(set_input_interrupt_bit, 0x10);

    unset_interrupt_bit!(unset_vblank_interrupt_bit, 0x1);
    unset_interrupt_bit!(unset_lcdc_interrupt_bit, 0x2);
    unset_interrupt_bit!(unset_timer_interrupt_bit, 0x4);
    unset_interrupt_bit!(unset_serial_io_interrupt_bit, 0x8);
    unset_interrupt_bit!(unset_input_interrupt_bit, 0x10);

    get_interrupt!(get_vblank_interrupt_bit, 0x1);
    get_interrupt!(get_lcdc_interrupt_bit, 0x2);
    get_interrupt!(get_timer_interrupt_bit, 0x4);
    get_interrupt!(get_serial_io_interrupt_bit, 0x8);
    get_interrupt!(get_input_interrupt_bit, 0x10);

    /*
     * SOUND:
     */

    // TODO: read protection based on FF26 bit 7

    // TODO: bits 0-3 are supposed to be read only
    set_sound_on!(set_sound1, 0x1);
    set_sound_on!(set_sound2, 0x2);
    set_sound_on!(set_sound3, 0x4);
    set_sound_on!(set_sound4, 0x8);
    set_sound_on!(set_sound_all, 0x80);

    get_sound_on!(get_sound1, 0x1);
    get_sound_on!(get_sound2, 0x2);
    get_sound_on!(get_sound3, 0x4);
    get_sound_on!(get_sound4, 0x8);
    get_sound_on!(get_sound_all, 0x80);

    unset_sound_on!(unset_sound1_, 0x1u8);
    unset_sound_on!(unset_sound2_, 0x2u8);
    unset_sound_on!(unset_sound3_, 0x4u8);
    unset_sound_on!(unset_sound4_, 0x8u8);
    unset_sound_on!(unset_sound_all_, 0x80u8);

    /// FIXME: 0xFF26's lower 4 bits should be protected in set_mem
    pub fn unset_sound1(&mut self) {
        self.mem[0xFF26] &= !(1);
        // on reset, clear value at
        self.mem[0xFF13] = 0;
    }

    pub fn unset_sound2(&mut self) {
        self.mem[0xFF26] &= !(2);
        // on reset, clear value at
        //        self.mem[0xFF13] = 0;
    }

    pub fn unset_sound3(&mut self) {
        self.mem[0xFF26] &= !(4);
        // on reset, clear value at
        //self.mem[0xFF13] = 0;
    }

    pub fn unset_sound4(&mut self) {
        self.mem[0xFF26] &= !(8);
        // on reset, clear value at
        //self.mem[0xFF13] = 0;
    }

    pub fn unset_sound_all(&mut self) {
        self.unset_sound1();
        self.unset_sound2();
        self.unset_sound3();
        self.unset_sound4();
        self.mem[0xFF26] &= !(0x80);
        // zero all audio registers
        for i in 0xFF10..=0xFF25 {
            self.mem[i] = 0;
        }
    }

    set_interrupt_enabled!(set_vblank_interrupt_enabled, 0x1);
    set_interrupt_enabled!(set_lcdc_interrupt_enabled, 0x2);
    set_interrupt_enabled!(set_timer_interrupt_enabled, 0x4);
    set_interrupt_enabled!(set_serial_io_interrupt_enabled, 0x8);
    set_interrupt_enabled!(set_input_interrupt_enabled, 0x10);
    // Pretty sure this is wrong
    // set_interrupt_enabled!(set_interrupts_enabled, 0x1F);

    unset_interrupt_enabled!(unset_vblank_interrupt_enabled, 0x1);
    unset_interrupt_enabled!(unset_lcdc_interrupt_enabled, 0x2);
    unset_interrupt_enabled!(unset_timer_interrupt_enabled, 0x4);
    unset_interrupt_enabled!(unset_serial_io_interrupt_enabled, 0x8);
    unset_interrupt_enabled!(unset_input_interrupt_enabled, 0x10);
    unset_interrupt_enabled!(unset_interrupts_enabled, 0x1F);

    get_interrupt_enabled!(get_vblank_interrupt_enabled, 0x1);
    get_interrupt_enabled!(get_lcdc_interrupt_enabled, 0x2);
    get_interrupt_enabled!(get_timer_interrupt_enabled, 0x4);
    get_interrupt_enabled!(get_serial_io_interrupt_enabled, 0x8);
    get_interrupt_enabled!(get_input_interrupt_enabled, 0x10);
    // Pretty sure this is wrong
    // get_interrupt_enabled!(get_interrupts_enabled, 0x1F);

    pub fn get_interrupts_enabled(&self) -> bool {
        self.ime
    }

    set_stat!(set_coincidence_interrupt, 0x40);
    unset_stat!(unset_coincidence_interrupt, 0x40);
    get_stat!(get_coincidence_interrupt, 0x40);

    set_stat!(set_oam_interrupt, 0x20);
    unset_stat!(unset_oam_interrupt, 0x20);
    get_stat!(get_oam_interrupt, 0x20);

    set_stat!(set_vblank_interrupt_stat, 0x10);
    unset_stat!(unset_vblank_interrupt_stat, 0x10);
    get_stat!(get_vblank_interrupt_stat, 0x10);

    set_stat!(set_hblank_interrupt, 0x08);
    unset_stat!(unset_hblank_interrupt, 0x08);
    get_stat!(get_hblank_interrupt, 0x08);

    set_stat!(set_coincidence_flag, 0x04);
    unset_stat!(unset_coincidence_flag, 0x04);
    get_stat!(get_coincidence_flag, 0x04);

    pub fn set_hblank(&mut self) {
        //reset bottom two bits
        self.mem[STAT_ADDR] &= !0x3;
    }

    pub fn set_vblank(&mut self) {
        //setting LSB, reset next
        let old_val = self.mem[STAT_ADDR];
        self.mem[STAT_ADDR] = (old_val | 1) & (!2);
    }

    pub fn set_oam_lock(&mut self) {
        //reset LSB, set next
        let old_val = self.mem[STAT_ADDR];
        self.mem[STAT_ADDR] = (old_val | 2) & (!1);
    }

    /// A.K.A Transfering data to the LCD driver
    pub fn set_oam_and_display_lock(&mut self) {
        //set LSB and next
        self.mem[STAT_ADDR] |= 0x3;
    }

    pub fn lcdc_on(&self) -> bool {
        (self.mem[0xFF40_u16] >> 7) & 1 == 1
    }
    // confusing name, this is for window only
    pub fn lcdc_tile_map(&self) -> bool {
        (self.mem[0xFF40_u16] >> 6) & 1 == 1
    }
    pub fn lcdc_window_on(&self) -> bool {
        (self.mem[0xFF40_u16] >> 5) & 1 == 1
    }
    pub fn lcdc_bg_win_tile_data(&self) -> bool {
        (self.mem[0xFF40_u16] >> 4) & 1 == 1
    }
    pub fn lcdc_bg_tile_map(&self) -> bool {
        (self.mem[0xFF40_u16] >> 3) & 1 == 1
    }
    pub fn lcdc_sprite_size(&self) -> bool {
        (self.mem[0xFF40_u16] >> 2) & 1 == 1
    }
    pub fn lcdc_sprite_display(&self) -> bool {
        (self.mem[0xFF40_u16] >> 1) & 1 == 1
    }
    pub fn lcdc_bg_win_display(&self) -> bool {
        self.mem[0xFF40_u16] & 1 == 1
    }

    pub fn window_x_pos(&self) -> u8 {
        self.mem[0xFF4B_u16]
    }

    pub fn window_y_pos(&self) -> u8 {
        self.mem[0xFF4A_u16]
    }

    pub fn get_nth_background_tile_idx(&self, n: u16) -> MemAddr {
        if self.lcdc_bg_win_tile_data() {
            (n as u16) * 16
        } else {
            let n = 128 + ((n as i8 as i16) + 128) as u16;
            n * 16
        }
    }

    pub fn get_nth_background_tile(&self, n: u16) -> MemAddr {
        let idx = self.get_nth_background_tile_idx(n);
        0x8000 + idx
    }

    pub fn scy(&self) -> u8 {
        self.mem[0xFF42_u16]
    }
    pub fn scx(&self) -> u8 {
        self.mem[0xFF43_u16]
    }

    pub fn ly(&self) -> u8 {
        self.mem[0xFF44_u16]
    }

    pub fn inc_ly(&mut self) {
        let v = self.ly().wrapping_add(1) % 154;
        self.mem[0xFF44] = v as byte;
        // interrupt should only be thrown on the rising edge (when ly
        // turns to 144)
        //TODO: verify that this should only be done if the interrupt is enabled
        if v == 144 && self.get_interrupts_enabled() && self.get_vblank_interrupt_enabled() {
            self.set_vblank_interrupt_bit();
        }
        //LY check is done any time LY is updated
        self.lyc_compare();
    }

    pub fn lyc(&self) -> u8 {
        self.mem[0xFF45_u16]
    }

    fn lyc_compare(&mut self) {
        let ly = self.ly();
        let lyc = self.lyc();

        if ly == lyc {
            self.set_coincidence_flag();
            if self.get_interrupts_enabled()
                && self.get_lcdc_interrupt_enabled()
                && self.get_coincidence_interrupt()
            {
                // interrupts are only triggered on a rising edge
                if !self.get_lcdc_interrupt_bit() {
                    self.set_lcdc_interrupt_bit();
                }
            }
        } else {
            self.unset_coincidence_flag();
        }
    }

    /// Direct memory access, lets the CPU copy memory without being
    /// directly involve
    ///
    /// Should take 160 microseconds
    ///
    /// During DMA, everything but high memory should be blocked
    fn dma(&mut self) {
        let addr = (self.mem[0xFF46_u16] as MemAddr) << 8;

        for i in 0..=0x9F {
            let val = self.mem[addr + i];
            self.mem.oam[i as usize] = val;
        }
    }

    /// GBC only
    fn vram_dma(&mut self, suspend_resume_mode: bool) {
        if suspend_resume_mode {
            unimplemented!("suspend resume VRAM DMA");
        }
        let src_addr =
            ((self.mem[0xFF51_u16] as MemAddr) << 8 | (self.mem[0xFF52_u16] as MemAddr)) & !0xF;
        let dest_addr =
            ((self.mem[0xFF53_u16] as MemAddr) << 8 | (self.mem[0xFF54_u16] as MemAddr)) & 0x0FF0;

        let chunks_to_copy = {
            let cc = self.mem[0xFF55_u16] & 0x7F;
            let flag = (self.mem[0xFF55_u16] >> 7) & 1;
            if flag == 1 {
                panic!("HBLANK DMA!");
            }
            (cc + 1) as u16
        };
        // set high bit to indicate DMA in progress
        self.mem[0xFF55] = 0x80;
        debug!(
            "VRAM DMA from 0x{:X} to 0x{:X}, {} chunks",
            src_addr, dest_addr, chunks_to_copy
        );

        for i in 0..chunks_to_copy as u16 {
            for j in 0..16 {
                let val = self.mem[src_addr + (i * 16) + j];
                self.mem.video_ram[self.mem.gbc_vram_bank as usize]
                    [(dest_addr + (i * 16) + j) as usize] = val;
            }
        }
        self.mem[0xFF55] = 0xFF;
    }

    pub fn bgp(&self) -> (byte, byte, byte, byte) {
        let v4 = ((self.mem[0xFF47_u16] >> 6) & 0x3) as byte;
        let v3 = ((self.mem[0xFF47_u16] >> 4) & 0x3) as byte;
        let v2 = ((self.mem[0xFF47_u16] >> 2) & 0x3) as byte;
        let v1 = (self.mem[0xFF47_u16] & 0x3) as byte;

        (v1, v2, v3, v4)
    }

    pub fn obp0(&self) -> (byte, byte, byte, byte) {
        let v4 = ((self.mem[0xFF48_u16] >> 6) & 0x3) as byte;
        let v3 = ((self.mem[0xFF48_u16] >> 4) & 0x3) as byte;
        let v2 = ((self.mem[0xFF48_u16] >> 2) & 0x3) as byte;
        let v1 = (self.mem[0xFF48_u16] & 0x3) as byte;

        (v1, v2, v3, v4)
    }

    pub fn obp1(&self) -> (byte, byte, byte, byte) {
        let v4 = ((self.mem[0xFF49_u16] >> 6) & 0x3) as byte;
        let v3 = ((self.mem[0xFF49_u16] >> 4) & 0x3) as byte;
        let v2 = ((self.mem[0xFF49_u16] >> 2) & 0x3) as byte;
        let v1 = (self.mem[0xFF49_u16] & 0x3) as byte;

        (v1, v2, v3, v4)
    }

    pub fn wy(&self) -> u8 {
        self.mem[0xFF4A_u16]
    }

    pub fn wx(&self) -> u8 {
        self.mem[0xFF4B_u16]
    }

    // CGB registers:

    pub fn background_color_palette_auto_increment(&self) -> bool {
        self.mem[0xFF68_u16] & 0x80 != 0
    }

    pub fn background_color_palette_index(&self) -> u8 {
        self.mem[0xFF68_u16] & 0x3F
    }

    // little-endian RGB555
    pub fn background_color_palette_info(&self, palette_idx: u8) -> (u8, u8, u8) {
        debug_assert!(palette_idx & 0x1 == 0);
        debug_assert!(palette_idx < 0x40);
        let byte1 = self.mem.gbc_background_color_palette[palette_idx as usize];
        let byte2 = self.mem.gbc_background_color_palette[palette_idx as usize + 1];
        let red = byte1 & 0x1F;
        let green = (byte1 >> 5) | ((byte2 & 0x7) << 3);
        let blue = (byte2 >> 2) & 0x1F;

        (red << 3, green << 3, blue << 3)
    }

    pub fn sprite_color_palette_auto_increment(&self) -> bool {
        self.mem[0xFF6A_u16] & 0x80 != 0
    }

    pub fn sprite_color_palette_index(&self) -> u8 {
        self.mem[0xFF6A_u16] & 0x3F
    }

    // little-endian RGB555
    pub fn sprite_color_palette_info(&self, palette_idx: u8) -> (u8, u8, u8) {
        debug_assert!(palette_idx & 0x1 == 0);
        debug_assert!(palette_idx < 0x40);
        let byte1 = self.mem.gbc_sprite_color_palette[palette_idx as usize];
        let byte2 = self.mem.gbc_sprite_color_palette[palette_idx as usize + 1];
        let red = byte1 & 0x1F;
        let green = (byte1 >> 5) | ((byte2 & 0x7) << 3);
        let blue = (byte2 >> 2) & 0x1F;

        (red << 3, green << 3, blue << 3)
    }

    //input register for joypad
    /*
     * 0x80 = start
     * 0x40 = select
     * 0x20 = B
     * 0x10 = A
     * 0x8  = Down
     * 0x4  = Up
     * 0x2  = Left
     * 0x1  = Right
     */

    /* Input : */
    /* NOTE: see comment next to definition of button! for why this
    has to be done with so much boiler plate */
    button!(press_start, unpress_start, 0x80u8);
    button!(press_select, unpress_select, 0x40u8);
    button!(press_b, unpress_b, 0x20u8);
    button!(press_a, unpress_a, 0x10u8);
    button!(press_down, unpress_down, 0x8u8);
    button!(press_up, unpress_up, 0x4u8);
    button!(press_left, unpress_left, 0x2u8);
    button!(press_right, unpress_right, 0x1u8);

    pub fn get_game_name(&self) -> String {
        let mut name_data: Vec<u8> = vec![];
        for i in 0..16_u16 {
            if self.mem[0x134 + i] != 0 {
                name_data.push(self.mem[0x134 + i]);
            }
        }
        match String::from_utf8(name_data) {
            Ok(s) => s,
            _ => "Illegally named game".to_string(),
        }
    }

    pub fn get_cartridge_type(&self) -> u8 {
        self.mem[0x147_u16]
    }

    fn enable_interrupts(&mut self) {
        self.ime = true;
    }

    /// Disables interrupts if the `interrupt_next_inst` flag is on
    fn maybe_disable_interrupts(&mut self) {
        if self.interrupt_next_inst {
            self.disable_interrupts();
            self.interrupt_next_inst = false;
        }
    }

    fn disable_interrupts(&mut self) {
        self.ime = false;
    }

    fn af(&self) -> u16 {
        byte_to_u16(self.f, self.a)
    }

    fn set_af(&mut self, v: u16) {
        self.a = ((v >> 8) & 0xFF) as byte;
        // lower 4 bits are always zero
        self.f = (v & 0xF0) as byte;
    }

    fn hl(&self) -> u16 {
        byte_to_u16(self.l, self.h)
    }

    fn set_hl(&mut self, hlv: u16) {
        let shlv = hlv as i32;
        self.h = (((shlv & 0xFF00) >> 8) & 0xFF) as byte;
        self.l = (shlv & 0xFF) as byte;
    }

    fn bc(&self) -> u16 {
        //((self.b as u16) << 8) | ((self.c as u16) & 0xFF)
        byte_to_u16(self.c, self.b)
    }

    fn de(&self) -> u16 {
        //((self.d as u16) << 8) | ((self.e as u16) & 0xFF)
        byte_to_u16(self.e, self.d)
    }

    fn set_bc(&mut self, bcv: u16) {
        self.b = (((bcv & 0xFF00) >> 8) & 0xFF) as byte;
        self.c = (bcv & 0xFF) as byte;
    }

    fn set_de(&mut self, dev: u16) {
        self.d = (((dev & 0xFF00) >> 8) & 0xFF) as byte;
        self.e = (dev & 0xFF) as byte;
    }

    fn set_register16(&mut self, reg: CpuRegister16, val: u16) {
        match reg {
            CpuRegister16::BC => self.set_bc(val),
            CpuRegister16::DE => self.set_de(val),
            CpuRegister16::HL => self.set_hl(val),
            CpuRegister16::AF => self.set_af(val),
            CpuRegister16::SP => self.sp = val,
            _ => panic!("Invalid 16bit register!"),
        }
    }

    pub fn access_register16(&mut self, reg: CpuRegister16) -> u16 {
        match reg {
            CpuRegister16::BC => self.bc(),
            CpuRegister16::DE => self.de(),
            CpuRegister16::HL => self.hl(),
            CpuRegister16::SP => self.sp,
            CpuRegister16::AF => self.af(),
            CpuRegister16::Num(i) => i as u16,
        }
    }

    fn set_flags(&mut self, z: bool, n: bool, h: bool, c: bool) {
        let zn = (z as byte) << 7;
        let nn = (n as byte) << 6;
        let hn = (h as byte) << 5;
        let cn = (c as byte) << 4;

        self.f = zn | nn | hn | cn;
    }

    #[inline]
    fn is_flag_set(&self, mask: u8) -> u8 {
        if (self.f & mask) != 0 {
            1
        } else {
            0
        }
    }

    #[inline]
    pub fn get_mem(&self, address: MemAddr) -> byte {
        /*
        if let Some(ref mut logger) = self.mem.logger {
            logger.log_read(self.cycles, address);
        }
        */
        let address = address as usize;
        // TODO: make responsibility for where logic on memory access happens more clear
        match address {
            DISPLAY_RAM_START..=DISPLAY_RAM_END => {
                self.mem[address]
                /*
                if self.mem[STAT_ADDR] & 3 == 3 {
                    //error!("CPU cannot read address {} at this time", address);
                    0xFF
                } else {
                    self.mem[address]
                }
                */
            }
            OAM_START..=OAM_END => {
                self.mem[address]
                /*match self.mem[STAT_ADDR] & 3 {
                    0b10 | 0b11 => {
                        //error!("CPU cannot read address {} while the OAM is in use", address);
                        0xFF
                    }
                    _ => self.mem[address],
                },*/
            }
            0xFF10 => self.mem[0xFF10_u16] | 0x80,
            0xFF11 => self.mem[0xFF11_u16] | 0x3F, //& 0b1100_0000,
            // write only audio register
            0xFF13 => 0xFF,
            0xFF14 => self.mem[0xFF14_u16] | !0b0100_0000,
            // NR20: unused channel 2 register
            0xFF15 => 0xFF,
            0xFF16 => self.mem[0xFF16_u16] | !0b1100_0000,
            0xFF18 => 0xFF,
            0xFF19 => self.mem[0xFF19_u16] | !0b0100_0000,
            0xFF1A => self.mem[0xFF1A_u16] | !0b1000_0000,
            0xFF1B => 0xFF,
            0xFF1C => self.mem[0xFF1C_u16] | !0b0110_0000,
            0xFF1D => 0xFF,
            0xFF1E => self.mem[0xFF1E_u16] | !0b0100_0000,
            // NR40: unused channel 4 register
            0xFF1F => 0xFF,
            0xFF20 => 0xFF,
            0xFF23 => self.mem[0xFF23_u16] | !0b0100_0000,
            0xFF26 => self.mem[0xFF26_u16] | !0b1000_1111,
            0xFF27..=0xFF2F => 0xFF,
            0xFF69 if self.gbc_mode => {
                self.mem.gbc_background_color_palette[(self.mem[0xFF68_u16] & 0x3F) as usize]
            }
            0xFF6B if self.gbc_mode => {
                self.mem.gbc_sprite_color_palette[(self.mem[0xFF6A_u16] & 0x3F) as usize]
            }
            _ => self.mem[address],
        }
    }

    #[inline]
    pub fn set_mem(&mut self, address: MemAddr, value: byte) {
        /*!
        NOTE: serial I/O is done by accessing memory addresses.
        It is read at 8192Hz, one bit at a time if external clock is used.
        See documentation and read carefully before implementing this.
         */

        if let Some(ref mut logger) = self.mem.logger {
            logger.log_write(self.cycles, address, value);
        }
        // writes are ignored if APU is off.
        // TODO: DMG allows writing to part of length registers
        if !self.get_sound_all() && (0xFF10..=0xFF25).contains(&address) {
            return;
        }

        let address = address as usize;

        match address {
            v @ DISPLAY_RAM_START..=DISPLAY_RAM_END => {
                self.mem[v] = value as byte;
                // If in OAM and Display ram are both in use
                /*if self.mem[STAT_ADDR] & 3 == 3 {
                    error!("CPU cannot write to address {} at this time", v);
                } else {
                    self.mem[v] = value as byte;
                }*/
            }
            v @ OAM_START..=OAM_END => {
                //if OAM is in use
                self.mem[v] = value as byte;
                /*
                match self.mem[STAT_ADDR] & 3 {
                    0b10 | 0b11 => {
                        error!("CPU cannot write to address {} while the OAM is in use", v);
                    }
                    _ => self.mem[v] = value as byte,
                }
                */
            }
            /*  ad @ 0xE000..=0xFE00 | ad @ 0xC000..=0xDE00 => {
                self.mem[ad] = value;
                self.mem[ad ^ (0xE000 - 0xC000)] = value;
            }*/
            0xFF00 => {
                // (P1) Joypad Info
                if value & 0x10 == 0x10 {
                    // P14 is set to low
                    self.mem[0xFF00] = value | (self.input_state >> 4);
                } else if value & 0x20 == 0x20 {
                    // P15 is set to low
                    self.mem[0xFF00] = value | (self.input_state & 0x0F);
                }
            }
            0xFF04 => self.mem[0xFF04] = 0,
            // TODO: Check whether vblank should be turned off on
            // writes to 0xFF44
            0xFF10 => {
                let old_sweep_pace = self.channel1_sweep_pace();
                self.mem[0xFF10] = value & 0x7F;
                let new_sweep_pace = self.channel1_sweep_pace();

                if old_sweep_pace == 0 && new_sweep_pace != 0 {
                    // TODO: iterations should be instantly restarted and pace must be reread. we need to siganl this to the timing logic
                }
            }

            // channel 1: NR11
            0xFF11 => {
                if value & 0x3F == 0 {
                    //self.unset_sound1();
                    self.mem[0xFF11] = value; // | 0x3F;
                } else {
                    self.mem[0xFF11] = value;
                }
            }

            // channel 2: NR21
            0xFF16 => {
                if value & 0x3F == 0 {
                    //self.unset_sound2();
                    self.mem[0xFF16] = value; // | 0x3F;
                } else {
                    self.mem[0xFF16] = value;
                }
            }

            // channel 3: NR31
            0xFF1B => {
                if value == 0 {
                    //self.unset_sound3();
                }
                self.mem[0xFF1B] = value;
            }

            // channel 4: NR41
            0xFF20 => {
                if value & 0x3F == 0 {
                    //self.unset_sound4();
                }
                self.mem[0xFF20] = value;
            }

            // channel 1: NR12
            0xFF12 => {
                if value >> 3 == 0 {
                    self.unset_sound1();
                }
                // TODO: APU rewrite:
                // writes here require retriggering to take effect
                self.mem[0xFF12] = value;
            }

            // channel 2: NR22
            0xFF17 => {
                if value >> 3 == 0 {
                    self.unset_sound2();
                }
                // TODO: APU rewrite:
                // writes here require retriggering to take effect
                self.mem[0xFF17] = value;
            }

            // channel 4: NR42
            0xFF21 => {
                if value >> 3 == 0 {
                    self.unset_sound4();
                }
                // TODO: APU rewrite:
                // writes here require retriggering to take effect
                self.mem[0xFF21] = value;
            }

            // channel 3: NR30
            0xFF1A => {
                if value >> 7 == 1 {
                    // DAC does not trigger the channel
                    //self.set_sound3();
                } else {
                    self.unset_sound3();
                }
                self.mem[0xFF1A] = value;
            }

            // channel 1: NR14
            0xFF14 => {
                if value >> 7 == 1 {
                    // ensure DAC is enabled
                    if self.mem[0xFF12_u16] >> 3 != 0 {
                        self.set_sound1();
                    }
                }
                self.mem[0xFF14] = value;
            }

            // channel 2: NR24
            0xFF19 => {
                if value >> 7 == 1 {
                    // ensure that the DAC is enabled here before triggering
                    if self.mem[0xFF17_u16] >> 3 != 0 {
                        self.set_sound2();
                    }
                }
                self.mem[0xFF19] = value;
            }

            // channel 3: NR34
            0xFF1E => {
                if value >> 7 == 1 {
                    // ensure that the DAC is enabled here before triggering
                    if self.mem[0xFF1A_u16] >> 7 == 1 {
                        self.set_sound3();
                    }
                }
                self.mem[0xFF1E] = value;
            }

            // channel 4: NR44
            0xFF23 => {
                if value >> 7 == 1 {
                    // ensure that the DAC is enabled here before triggering
                    if self.mem[0xFF21_u16] >> 3 != 0 {
                        self.set_sound4();
                    }
                }
                self.mem[0xFF23] = value;
            }

            // Sound
            // NR52
            0xFF26 => {
                // NOTE: This currently ignores writes to any bit but the highest
                // This is probably incorrect behavior
                // The lowest 4 bits are documented as being read-only status bits
                // but it's implied that they can be written to, just that it will not affect
                // the logic.  Fixing this and emulating it completely will require keeping track
                // of sound in a different place than NR 52
                if (value >> 7) & 1 == 0 {
                    self.unset_sound_all();
                } else if (value >> 7) & 1 == 1 {
                    // TODO: clear wave ram on power on too
                    self.set_sound_all();
                }
            }
            0xFF44 => {
                // cannot write to LY
                //dbg!(self.mem[0xFF44] = 0),
            }
            0xFF45 => {
                //LY check is done every time LY or LYC value is updated
                self.mem[0xFF45] = value;
                //self.lyc_compare();
            }
            0xFF46 => {
                self.mem[0xFF46] = value;
                self.dma();
            }
            0xFF4F => {
                self.mem[0xFF4F] = value;
                if self.gbc_mode {
                    self.mem.gbc_vram_bank = value & 1;
                }
            }
            // GBC DMA
            // Missing suspend-resume DMA
            0xFF55 => {
                self.mem[0xFF55] = value;
                if self.gbc_mode {
                    self.vram_dma((value >> 7) != 0);
                }
            }
            0xFF6C => {
                self.mem[0xFF6C] = value;
                if self.gbc_mode {
                    // TODO: object priority mode, set by BIOS during boot, do we care about this?
                }
            }
            // cgb wram bank
            0xFF70 => {
                self.mem[0xFF70] = value;
                if self.gbc_mode {
                    trace!("Switching to WRAM bank {}", value & 0x7);
                    let mut bank_num = value & 0x7;
                    if bank_num == 0 {
                        bank_num = 1;
                    }
                    self.mem.gbc_wram_bank = bank_num;
                }
            }
            0xFF69 if self.gbc_mode => {
                // TODO: protect writes during mode 3 but still auto increment if set
                self.mem.gbc_background_color_palette[(self.mem[0xFF68_u16] & 0x3F) as usize] =
                    value;
                if self.background_color_palette_auto_increment() {
                    let v = self.mem[0xFF68_u16];
                    self.mem[0xFF68] = (v.wrapping_add(1) & 0x3F) | 0x80;
                }
            }
            0xFF6B if self.gbc_mode => {
                // TODO: protect writes during mode 3 but still auto increment if set
                self.mem.gbc_sprite_color_palette[(self.mem[0xFF6A_u16] & 0x3F) as usize] = value;
                if self.sprite_color_palette_auto_increment() {
                    let v = self.mem[0xFF6A_u16];
                    self.mem[0xFF6A] = (v.wrapping_add(1) & 0x3F) | 0x80;
                }
            }
            //switchable ram bank;
            /*i @ 0xA000...0xC000 => {
                self.mem.write_ram_value(i as u16, value);
            }*/
            n => self.mem.write_ram_value(n as u16, value),
        }
    }

    pub fn access_register(&self, reg: CpuRegister) -> Option<byte> {
        match reg {
            CpuRegister::A => Some(self.a),
            CpuRegister::B => Some(self.b),
            CpuRegister::C => Some(self.c),
            CpuRegister::D => Some(self.d),
            CpuRegister::E => Some(self.e),
            CpuRegister::H => Some(self.h),
            CpuRegister::L => Some(self.l),
            CpuRegister::HL => Some(self.get_mem(self.hl())),
            _ => None,
        }
    }

    fn set_register(&mut self, reg: CpuRegister, val: byte) {
        match reg {
            CpuRegister::A => self.a = val,
            CpuRegister::B => self.b = val,
            CpuRegister::C => self.c = val,
            CpuRegister::D => self.d = val,
            CpuRegister::E => self.e = val,
            CpuRegister::H => self.h = val,
            CpuRegister::L => self.l = val,
            CpuRegister::HL => {
                let hlv = self.hl();
                self.set_mem(hlv, val);
            }
            _ => panic!("Cannot set non-8bit values"),
        }
    }

    fn ldnnn(&mut self, nn: CpuRegister, n: u8) {
        self.set_register(nn, n as byte);
    }

    fn ldr1r2(&mut self, r1: CpuRegister, r2: CpuRegister) {
        let val = self.access_register(r2).expect("Invalid register");
        self.set_register(r1, val);
    }

    fn ldan16(&mut self, n: CpuRegister16) {
        let addr = self.access_register16(n);
        let val = self.get_mem(addr);

        self.set_register(CpuRegister::A, val);
    }

    fn ldan16c(&mut self, b1: u8, b2: u8) {
        let val = self.get_mem(byte_to_u16(b1, b2));
        self.set_register(CpuRegister::A, val);
    }

    fn ldna16(&mut self, n: CpuRegister16) {
        let val = self
            .access_register(CpuRegister::A)
            .expect("Invalid register");
        let addr = self.access_register16(n);

        self.set_mem(addr, val);
    }

    fn ldna16c(&mut self, b1: u8, b2: u8) {
        let val = self
            .access_register(CpuRegister::A)
            .expect("Invalid register");
        self.set_mem(byte_to_u16(b1, b2), val);
    }

    fn ldac(&mut self) {
        let reg_c = self.c;
        // TODO check if C should be unsigned
        let val = self.get_mem(0xFF00u16 + (reg_c as u16));
        self.set_register(CpuRegister::A, val);
    }

    fn ldca(&mut self) {
        let addr = 0xFF00u16 + (self.c as u16);
        let val = self.a;
        self.set_mem(addr, val);
    }

    fn lddahl(&mut self) {
        let addr = self.hl();
        let val = self.get_mem(addr);

        self.set_register(CpuRegister::A, val);
        self.dec16(CpuRegister16::HL);
    }

    fn lddhla(&mut self) {
        let val = self.a;
        let addr = self.hl();

        self.set_mem(addr, val);
        self.dec16(CpuRegister16::HL);
    }

    fn ldiahl(&mut self) {
        let addr = self.hl();
        let val = self.get_mem(addr);

        self.set_register(CpuRegister::A, val);
        self.inc16(CpuRegister16::HL);
    }

    fn ldihla(&mut self) {
        let val = self.a;
        let addr = self.hl();

        self.set_mem(addr, val);
        self.inc16(CpuRegister16::HL);
    }

    fn ldhna(&mut self, n: u8) {
        let val = self.a;
        self.set_mem(0xFF00u16 + (n as u16), val);
    }

    fn ldhan(&mut self, n: u8) {
        let val = self.get_mem(0xFF00u16 + (n as u16));
        self.set_register(CpuRegister::A, val);
    }

    fn ldnnn16(&mut self, n: CpuRegister16, b1: u8, b2: u8) {
        self.set_register16(n, ((b2 as u16) << 8) | (b1 as u16));
    }

    fn ldsphl(&mut self) {
        let val = self.hl();
        self.set_register16(CpuRegister16::SP, val);
    }

    fn ldhlspn(&mut self, n: i8) {
        let old_sp = self.sp;
        self.addspn(n);
        let new_sp = self.sp;
        self.sp = old_sp;
        self.set_register16(CpuRegister16::HL, new_sp);
    }

    fn ldnnsp(&mut self, b1: u8, b2: u8) {
        let old_sp = self.sp;
        let addr = byte_to_u16(b1, b2);
        // TODO function to write word (16 bit) to memory
        self.set_mem(addr, old_sp as u8);
        self.set_mem(addr.wrapping_add(1), (old_sp >> 8) as u8);
    }

    // fn pushnn(&mut self, nn: CpuRegister16) {
    //     let val = self.access_register16(nn);

    //     self.push_onto_stack(val);
    // }

    // fn popnn(&mut self, nn: CpuRegister16) {
    //     let val = self.pop_from_stack();
    //     self.set_register16(nn, val);
    // }

    //TODO: rename this awfully named function
    fn alu_dispatch<F>(&self, reg: CpuRegister, f: F) -> i16
    where
        F: FnOnce(byte, byte) -> i16,
    {
        f(
            self.a,
            match reg {
                CpuRegister::A => self.a,
                CpuRegister::B => self.b,
                CpuRegister::C => self.c,
                CpuRegister::D => self.d,
                CpuRegister::E => self.e,
                CpuRegister::H => self.h,
                CpuRegister::L => self.l,
                CpuRegister::HL => self.get_mem(self.hl()),
                CpuRegister::Num(i) => i,
            },
        )
    }

    //TODO: rename this awfully named function
    fn alu_dispatch16<F>(&self, reg: CpuRegister16, f: F) -> i32
    where
        F: FnOnce(i32, i32) -> i32,
    {
        f(
            self.hl() as i32,
            match reg {
                CpuRegister16::BC => self.bc() as i32,
                CpuRegister16::DE => self.de() as i32,
                CpuRegister16::HL => self.hl() as i32,
                CpuRegister16::SP => self.sp as i32,
                CpuRegister16::AF => self.af() as i32,
                CpuRegister16::Num(i) => i as i32,
            },
        )
    }

    fn reg_or_const(&mut self, reg: CpuRegister) -> i8 {
        if let Some(r) = self.access_register(reg) {
            r as i8
        } else if let CpuRegister::Num(v) = reg {
            v as i8
        } else {
            unreachable!()
        }
    }

    fn addspn(&mut self, n: i8) {
        let old_sp = self.sp;
        let new_sp = add_u16_i8(self.sp, n);
        self.sp = new_sp;

        self.set_flags(
            false,
            false,
            ((((old_sp as i16) & 0xF) + ((n as i16) & 0xF)) & 0xF0) != 0,
            ((((old_sp as i16) & 0xFF) + ((n as i16) & 0xFF)) & 0xF00) != 0,
        );
    }

    fn add(&mut self, reg: CpuRegister) {
        let old_a = self.a as i8;
        let old_b = self.reg_or_const(reg);

        let new_a = old_a.wrapping_add(old_b);
        self.a = new_a as byte;

        self.set_flags(
            new_a == 0,
            false,
            ((old_a & 0xF) + (old_b & 0xF)) & 0x10 == 0x10,
            (((old_a as u8) as u16) + ((old_b as u8) as u16)) & 0x100 == 0x100,
        );
    }

    fn adc(&mut self, reg: CpuRegister) {
        let old_a = self.a as i8;
        let old_b = self.reg_or_const(reg);
        let cf = (((self.f & CL) >> 4) & 1) as i8;

        let new_a = old_a.wrapping_add(old_b).wrapping_add(cf);
        self.a = new_a as byte;

        self.set_flags(
            new_a == 0,
            false,
            ((old_a & 0xF) + (old_b & 0xF) + cf) & 0x10 == 0x10,
            (((old_a as u8) as u16) + ((old_b as u8) as u16) + (cf as u16)) & 0x100 == 0x100,
        );
    }

    fn sub(&mut self, reg: CpuRegister) {
        let old_a = self.a as i8;
        let old_b = self.reg_or_const(reg);
        let new_a = old_a.wrapping_sub(old_b) as u8;
        self.a = new_a;

        self.set_flags(
            new_a == 0u8,
            true,
            ((((old_a & 0xF) - (old_b & 0xF)) as u8) & 0xF0) != 0,
            //                       (old_a & 0xF) >= (old_b & 0xF),
            //                       (old_a as i16) - (old_b as i16)
            (((((old_a as i16) & 0xFF) - ((old_b as i16) & 0xFF)) as u16) & 0xFF00) != 0,
        );
    }

    fn sbc(&mut self, reg: CpuRegister) {
        let old_a = self.a as i8;
        let old_b = self.reg_or_const(reg);
        let cf = ((self.f & CL) >> 4) as i8;

        let new_a = old_a.wrapping_sub(old_b).wrapping_sub(cf) as u8;
        self.a = new_a;

        self.set_flags(
            new_a == 0u8,
            true,
            ((((old_a & 0xF) - ((old_b & 0xF) + cf)) as u8) & 0xF0) != 0,
            //                       (old_a & 0xF) >= (old_b & 0xF),
            //                       (old_a as i16) - (old_b as i16)
            (((((old_a as i16) & 0xFF) - (((old_b as i16) & 0xFF) + (cf as i16))) as u16) & 0xFF00)
                != 0,
        );
    }

    fn and(&mut self, reg: CpuRegister) {
        let new_a: byte =
            self.alu_dispatch(reg, |a: byte, b: byte| (a as i16) & (b as i16)) as byte;

        self.a = new_a;
        self.set_flags(new_a == 0u8, false, true, false);
    }

    fn or(&mut self, reg: CpuRegister) {
        let new_a: byte = self.alu_dispatch(reg, |a: byte, b: byte| {
            (((a as u16) & 0xFF) | ((b as u16) & 0xFF)) as i16
        }) as byte;

        self.a = new_a;
        self.set_flags(new_a == 0u8, false, false, false);
    }

    fn xor(&mut self, reg: CpuRegister) {
        let new_a: byte =
            self.alu_dispatch(reg, |a: byte, b: byte| (a as i16) ^ (b as i16)) as byte;

        self.a = new_a;
        self.set_flags(new_a == 0u8, false, false, false);
    }

    fn cp(&mut self, reg: CpuRegister) {
        let old_a = self.a;
        self.sub(reg);
        self.a = old_a;
    }

    fn inc(&mut self, reg: CpuRegister) {
        let old_c = (self.f & CL) == CL;
        let old_3bit = self.access_register(reg).expect("invalid register") & 0x8;

        let old_val: i16 = self.access_register(reg).expect("invalid register") as i16;
        let new_val = (old_val + 1) as byte;
        self.set_register(reg, new_val);
        self.set_flags(
            new_val == 0u8, // this check fails if new_val is i16 :)
            false,
            (old_3bit == 0x8u8) && ((new_val & 0x8u8) != 0x8),
            old_c,
        );
    }

    //
    fn dec(&mut self, reg: CpuRegister) {
        let old_c = (self.f & CL) == CL;

        let reg_val = self.access_register(reg).expect("invalid register");

        let new_val: byte = reg_val.wrapping_sub(1) as byte;
        self.set_register(reg, new_val);

        self.set_flags(
            new_val == 0u8,
            true,
            ((reg_val & 0xF).wrapping_sub(1) & 0xF0) != 0,
            old_c,
        );
    }

    /*
    * Explanation for stream:
    *
    * xxx1 xxxx xxxx
    * xxx0 xxxx xxxx
    *
    * 0xFF + 0xFF
    * 0x1FE
    *
    * 0xEFF + 0xEFF
    * 0xFFE

    * One bit adder:
    * Circuit with three inputs and two outputs
    * (diagram does not include carry as input)
    * n1 + n2 = s c
    * 0  + 0  = 0 0
    * 0  + 1  = 1 0
    * 1  + 0  = 1 0
    * 1  + 1  = 0 1
    */

    fn add_hl(&mut self, reg: CpuRegister16) {
        let old_z = (self.f & ZL) == ZL;

        //TODO: Maybe properly convert if signed
        let hl_12bit = self.hl() & 0xFFF;
        let regval_12bit = self.access_register16(reg) & 0xFFF;
        let overflow12bit = (hl_12bit + regval_12bit) > 0xFFF;

        let hl_16bit = self.hl() as i32;
        let regval_16bit = self.access_register16(reg) as i32;
        let overflow16bit = (hl_16bit + regval_16bit) > 0xFFFF;

        self.set_register16(CpuRegister16::HL, (hl_16bit + regval_16bit) as u16);

        self.set_flags(old_z, false, overflow12bit, overflow16bit);
    }

    // addspn() is used instead
    // //Consider adding further restrictions to this type; argument must be an immediate value
    // fn add_sp(&mut self, reg: CpuRegister16) {
    //     if let CpuRegister16::Num(i) = reg {
    //         self.sp = ((self.sp as i16) + i )as u16;
    //         self.set_flags(
    //             false,
    //             false,
    //             false, //TODO: wat
    //             false);//TODO: wat
    //     }
    //     else {
    //         panic!("In add_sp, invalid argument.  It must be an immediate value");
    //     }
    // }

    fn inc16(&mut self, reg: CpuRegister16) {
        match reg {
            CpuRegister16::BC => {
                let old_v = (self.bc() as u32) + 1;
                self.set_bc(old_v as u16);
            }
            CpuRegister16::DE => {
                let old_v = (self.de() as u32) + 1;
                self.set_de(old_v as u16);
            }
            CpuRegister16::HL => {
                let old_v = (self.hl() as u32) + 1;
                self.set_hl(old_v as u16);
            }
            CpuRegister16::SP => {
                let old_v = (self.sp as u32) + 1;
                self.set_register16(CpuRegister16::SP, old_v as u16);
            }
            _ => panic!("inc16 cannot take numeric values as arguments"),
        }
    }

    fn dec16(&mut self, reg: CpuRegister16) {
        let val: i16 = self.access_register16(reg) as i16;
        self.set_register16(
            reg,
            if (val as u16) == 0 {
                u16::max_value()
            } else {
                (val as u16) - 1
            },
        );
    }

    fn swap(&mut self, reg: CpuRegister) {
        //Potentially can bitmask hl which is 16bit value
        let val = self
            .access_register(reg)
            .expect("couldn't access register value");
        let top = val & 0xF0u8;
        let bot = val & 0x0Fu8;
        self.set_register(reg, (((top >> 4) & 0xF) | (bot << 4)) as byte);

        self.f = if val == 0u8 { ZL } else { 0 };
    }

    fn daa(&mut self) {
        let nf = self.f & NLV == NLV;
        let hf = self.f & HL == HL;
        let cf = self.f & CL == CL;

        let mut new_cf = cf;

        if !nf {
            if cf || self.a > 0x99 {
                self.a = self.a.wrapping_add(0x60);
                new_cf = true;
            }
            if hf || (self.a & 0xF) > 0x9 {
                self.a = self.a.wrapping_add(0x06);
            }
        } else {
            if cf {
                self.a = self.a.wrapping_sub(0x60);
            }
            if hf {
                self.a = self.a.wrapping_sub(0x06);
            }
        }

        let new_a = self.a;
        self.set_flags(
            new_a == 0,
            nf, // unchanged
            false,
            new_cf,
        );
    }

    fn cpl(&mut self) {
        let new_val = !self.a;
        let old_flags = self.f & (ZL | CL);
        self.f = old_flags | NLV | HL;
        self.a = new_val;
    }

    fn ccf(&mut self) {
        let old_flags = self.f & (ZL | CL);
        self.f = old_flags ^ CL;
    }

    fn scf(&mut self) {
        let old_flags = self.f & ZL;
        self.f = old_flags | CL;
    }

    fn nop(&self) {}

    fn halt(&mut self) {
        debug!("HALT");
        self.state = CpuState::Halt;
    }

    fn stop(&mut self) {
        debug!("STOP");
        if self.gbc_mode {
            // maybe track the state and check if in "waiting for speed switch" state
            let speed_mode = (self.mem[0xFF4D_u16] & 1) == 1;
            self.double_speed = speed_mode;
            self.mem[0xFF4D] = (speed_mode as u8) << 7;
        } else {
            // reset div
            self.mem[0xFF04] = 0;
            self.state = CpuState::Stop;
        }
    }

    fn di(&mut self) {
        self.interrupt_next_inst = true;
    }

    fn ei(&mut self) {
        self.enable_interrupts();
    }

    fn rlca(&mut self) {
        let old_bit7 = (self.a >> 7) & 1;

        let new_a = (self.a << 1) | old_bit7;
        self.a = new_a;

        self.set_flags(false, false, false, old_bit7 == 1);
    }

    fn rla(&mut self) {
        let old_bit7 = (self.a >> 7) & 1;
        let old_flags = ((self.f & CL) >> 4) & 0x1;

        let new_a = (self.a << 1) | old_flags;
        self.a = new_a;

        self.set_flags(false, false, false, old_bit7 == 1);
    }

    fn rrca(&mut self) {
        let old_bit0 = self.a & 1;

        let new_a = ((self.a >> 1) & 0x7F) | (old_bit0 << 7);
        self.a = new_a;

        self.set_flags(false, false, false, old_bit0 == 1);
    }

    fn rra(&mut self) {
        let old_bit0 = self.a & 1;
        let old_flags = ((self.f & CL) >> 4) & 0xF;

        let new_a = ((self.a >> 1) & 0x7F) | (old_flags << 7);
        self.a = new_a;

        self.set_flags(false, false, false, old_bit0 == 1);
    }

    fn rlc(&mut self, reg: CpuRegister) {
        let reg_val = self.access_register(reg).expect("invalid register");
        let old_bit7 = (reg_val >> 7) & 1;

        let new_reg = ((reg_val << 1) & 0xFEu8) | old_bit7; // | old_carry;
        self.set_register(reg, new_reg as byte);

        self.set_flags(new_reg == 0u8, false, false, old_bit7 == 1);
    }

    fn rl(&mut self, reg: CpuRegister) {
        let reg_val = self.access_register(reg).expect("invalid register");
        let old_bit7 = (reg_val >> 7) & 1;
        let old_flags = ((self.f & CL) >> 4) & 0xF;

        let new_reg = (reg_val << 1) | old_flags;
        self.set_register(reg, new_reg);

        self.set_flags(new_reg == 0u8, false, false, old_bit7 == 1);
    }

    fn rrc(&mut self, reg: CpuRegister) {
        let reg_val = self.access_register(reg).expect("invalid register");
        let old_bit0 = reg_val & 1;

        let new_val = ((reg_val >> 1) & 0x7F) | (old_bit0 << 7);
        self.set_register(reg, new_val);

        self.set_flags(new_val == 0u8, false, false, old_bit0 == 1);
    }

    /// Rotate n right through Carry flag.
    fn rr(&mut self, reg: CpuRegister) {
        let reg_val = self.access_register(reg).expect("invalid register");
        let old_bit0 = reg_val & 1;
        let old_flags = (self.f & CL) << 3;

        let new_val = (reg_val >> 1) | old_flags;
        self.set_register(reg, new_val);

        self.set_flags(new_val == 0u8, false, false, old_bit0 == 1);
    }

    fn sla(&mut self, reg: CpuRegister) {
        let reg_val = self.access_register(reg).expect("invalid register");
        let old_bit7 = (reg_val >> 7) & 1;
        self.set_register(reg, reg_val << 1);

        self.set_flags((reg_val << 1) == 0u8, false, false, old_bit7 == 1);
    }

    fn sra(&mut self, reg: CpuRegister) {
        let reg_val = self.access_register(reg).expect("invalid register");
        let old_bit0 = reg_val & 1;
        let old_bit7 = reg_val & 0x80;
        self.set_register(reg, (reg_val >> 1) | old_bit7);

        self.set_flags((reg_val >> 1) == 0u8, false, false, old_bit0 == 1);
    }

    fn srl(&mut self, reg: CpuRegister) {
        let reg_val = self.access_register(reg).expect("invalid register");
        let old_bit0 = reg_val & 1;

        self.set_register(reg, (reg_val >> 1) as byte);

        self.set_flags((reg_val >> 1) == 0u8, false, false, old_bit0 == 1);
    }

    fn bit(&mut self, b: u8, reg: CpuRegister) {
        let reg_val = self.access_register(reg).expect("invalid register");
        let old_flags = (self.f & CL) >> 4;

        self.set_flags(((reg_val >> b) & 1) != 1, false, true, (old_flags & 1) == 1);
    }

    fn set(&mut self, b: u8, reg: CpuRegister) {
        let reg_val = self.access_register(reg).expect("invalid register");
        self.set_register(reg, reg_val | (1 << b));
    }

    fn res(&mut self, b: u8, reg: CpuRegister) {
        let reg_val = self.access_register(reg).expect("invalid register");
        self.set_register(reg, reg_val & (!(1 << b)));
    }

    fn jpnn(&mut self, nn: u16) {
        let old_pc = self.pc;
        let new_pc = (Wrapping(nn) - Wrapping(3)).0;

        if let Some(ref mut logger) = self.mem.logger {
            logger.log_jump(self.cycles, old_pc, new_pc);
        }

        self.pc = new_pc; //NOTE: Verify this byte order
    }

    fn jpccnn(&mut self, cc: Cc, nn: u16) -> bool {
        if 1 == match cc {
            Cc::NZ => (!(self.f >> 7)) & 1,
            Cc::Z => (self.f >> 7) & 1,
            Cc::NC => (!(self.f >> 4)) & 1,
            Cc::C => (self.f >> 4) & 1,
        } {
            self.jpnn(nn);
            true
        } else {
            false
        }
    }

    //TODO: Double check (HL) HL thing
    fn jphl(&mut self) {
        let old_pc = self.pc;
        let new_pc = self.hl().wrapping_sub(1);

        if let Some(ref mut logger) = self.mem.logger {
            logger.log_jump(self.cycles, old_pc, new_pc);
        }

        self.pc = new_pc;
    }

    fn jrn(&mut self, n: i8) {
        let old_pc = self.pc;
        let new_pc = add_u16_i8(old_pc, n); //.wrapping_sub(2);
        if let Some(ref mut logger) = self.mem.logger {
            logger.log_jump(self.cycles, old_pc, new_pc);
        }
        self.pc = new_pc;
    }

    //Double check run time length
    fn jrccn(&mut self, cc: Cc, n: i8) -> bool {
        if 1 == match cc {
            Cc::NZ => (!(self.f >> 7)) & 1,
            Cc::Z => (self.f >> 7) & 1,
            Cc::NC => (!(self.f >> 4)) & 1,
            Cc::C => (self.f >> 4) & 1,
        } {
            self.jrn(n);
            true
        } else {
            false
        }
    }

    fn callnn(&mut self, nn: u16) {
        let old_pc = self.pc;
        self.push_onto_stack(old_pc + 3);
        let new_pc = nn;
        if let Some(ref mut logger) = self.mem.logger {
            logger.log_jump(self.cycles, old_pc, new_pc);
        }
        //nn -3 to account for pc inc in dispatch_opcode
        self.pc = (Wrapping(new_pc) - Wrapping(3)).0;
    }

    fn push_onto_stack(&mut self, nn: u16) {
        let first_half = ((nn >> 8) & 0xFF) as byte;
        let second_half = (nn & 0xFF) as byte;

        let mut sp_idx = Wrapping(self.sp);
        sp_idx -= Wrapping(1);
        self.set_mem(sp_idx.0, first_half);
        sp_idx -= Wrapping(1);
        self.set_mem(sp_idx.0, second_half);

        self.sp = (Wrapping(self.sp) - Wrapping(2)).0;
    }

    fn callccnn(&mut self, cc: Cc, nn: u16) -> bool {
        if 1 == match cc {
            Cc::NZ => (!(self.f >> 7)) & 1,
            Cc::Z => (self.f >> 7) & 1,
            Cc::NC => (!(self.f >> 4)) & 1,
            Cc::C => (self.f >> 4) & 1,
        } {
            self.callnn(nn);
            true
        } else {
            false
        }
    }

    fn rst(&mut self, n: u8) {
        let old_pc = self.pc;

        // Should store PC post-increment for RET from handler to work
        self.push_onto_stack(old_pc.wrapping_add(1));

        // (TW) TODO: verify this is okay
        self.pc = (n as u16).wrapping_sub(1);
    }

    fn pop_from_stack(&mut self) -> u16 {
        let mut sp_idx = Wrapping(self.sp as MemAddr);
        let second_half = self.get_mem(sp_idx.0);
        sp_idx += Wrapping(1);
        let first_half = self.get_mem(sp_idx.0);

        self.sp = (Wrapping(self.sp) + Wrapping(2)).0;
        byte_to_u16(second_half, first_half)
    }

    fn ret(&mut self) {
        let old_pc = self.pc;
        let new_pc = self.pop_from_stack();
        if let Some(ref mut logger) = self.mem.logger {
            logger.log_jump(self.cycles, old_pc, new_pc);
        }
        self.pc = (Wrapping(new_pc) - Wrapping(1)).0;
    }

    fn retcc(&mut self, cc: Cc) -> bool {
        if 1 == match cc {
            Cc::NZ => (!(self.f >> 7)) & 1,
            Cc::Z => (self.f >> 7) & 1,
            Cc::NC => (!(self.f >> 4)) & 1,
            Cc::C => (self.f >> 4) & 1,
        } {
            self.ret();
            true
        } else {
            false
        }
    }

    fn reti(&mut self) {
        self.ret();
        self.ei();
    }

    fn read_instruction(&self) -> (u8, u8, u8, u8) {
        // if self.pc > (0xFFFF - 3) {
        //     panic!("Less than 4bytes to read!!!\nNote: this may not be a problem with the ROM; if the ROM is correct, this is the result of lazy programming on my part -- sorry");
        // }
        (
            self.mem[self.pc],
            self.mem[self.pc + 1],
            self.mem[self.pc + 2],
            self.mem[self.pc + 3],
        )
    }

    fn inc_pc(&mut self) {
        self.pc = (Wrapping(self.pc) + Wrapping(1)).0;
    }

    fn handle_interrupts(&mut self) {
        if !self.get_interrupts_enabled() {
            return;
        }
        if self.state == CpuState::Halt {
            self.state = CpuState::Normal;
        } else if self.state == CpuState::Stop {
            //TODO: handle interrupt on stop
            //self.state = CpuState::Normal;
        }

        //Then handle interrupts
        if self.get_vblank_interrupt_enabled() && self.get_vblank_interrupt_bit() {
            //handle vblank interrupt
            trace!("INT: handle vblank interrupt");
            let old_pc = self.pc;

            self.disable_interrupts();
            self.unset_vblank_interrupt_bit();
            self.push_onto_stack(old_pc);

            self.pc = VBLANK_INTERRUPT_ADDRESS;
        } else if self.get_lcdc_interrupt_enabled() && self.get_lcdc_interrupt_bit() {
            //handle lcdc interrupt
            trace!("INT: handle lcdc interrupt");
            let old_pc = self.pc;

            self.disable_interrupts();
            self.unset_lcdc_interrupt_bit();
            self.push_onto_stack(old_pc);

            self.pc = LCDC_INTERRUPT_ADDRESS;
        } else if self.get_timer_interrupt_enabled() && self.get_timer_interrupt_bit() {
            //handle timer interrupt
            trace!("INT: handle timer interrupt");
            let old_pc = self.pc;

            self.disable_interrupts();
            self.unset_timer_interrupt_bit();
            self.push_onto_stack(old_pc);

            self.pc = TIMER_OVERFLOW_INTERRUPT_ADDRESS;
        } else if self.get_serial_io_interrupt_enabled() && self.get_serial_io_interrupt_bit() {
            //handle serial interrupt
            trace!("INT: handle serial inturrupt");
            let old_pc = self.pc;

            self.disable_interrupts();
            self.unset_serial_io_interrupt_bit();
            self.push_onto_stack(old_pc);

            self.pc = SERIAL_TRANSFER_INTERRUPT_ADDRESS;
        } else if self.get_input_interrupt_enabled() && self.get_input_interrupt_bit() {
            debug!("INT: handle input");

            let old_pc = self.pc;

            self.disable_interrupts();
            self.unset_input_interrupt_bit();
            self.push_onto_stack(old_pc);

            self.pc = P1013_INTERRUPT_ADDRESS;
        }
    }

    /*
    Handles running opcodes
    including handling of interrupts

    Opcodes are prefixed or unprefixed
    1. [prefix byte,] opcode [,displacement byte] [,immediate data]
    2. prefix byte, prefix byte, displacement byte, opcode

    ASSUMPTION: Gameboy only uses the CB prefix codes of the Z80

    Returned value is number of cycles that the instruction took
     */
    pub fn dispatch_opcode(&mut self) -> u8 {
        if self.state == CpuState::Crashed {
            panic!("Attempt to run a crashed cpu PC={}", self.pc);
        }
        // This may change PC, so should be called before fetching instruction
        self.handle_interrupts();

        let mut inst_time = 4;
        let (first_byte, second_byte, third_byte, _) = self.read_instruction();
        let x = (first_byte >> 6) & 0x3;
        let y = (first_byte >> 3) & 0x7;
        let z = first_byte & 0x7;

        //First check if CPU is in a running state
        #[allow(clippy::if_same_then_else)]
        if self.state == CpuState::Halt {
            //TODO: Needs extra handling with interupts
            return inst_time; //unsure of this
        } else if self.state == CpuState::Stop {
            return inst_time; //unsure of this
        } //otherwise it's in normal state:

        {
            let cur_pc = self.pc;
            if let Some(ref mut logger) = self.mem.logger {
                logger.log_exec(self.cycles, cur_pc);
            }
        }

        /*println!("PC:{:x} SP:{:x} A:{:x} F:{:x} B:{:x} C:{:x} D:{:x} E:{:x} H:{:x} L:{:x}",
            self.pc, self.sp, self.a, self.f, self.b, self.c, self.d, self.e, self.h, self.l
        );*/
        trace!("REG: A:{:02X} B:{:02X} C:{:02X} D:{:02X} E:{:02X} H:{:02X} L:{:02X} SP:{:04X} Z:{} N:{} H:{} C:{} (SP):{:02X}{:02X} (HL):{:02X}",
               self.a,
               self.b,
               self.c,
               self.d,
               self.e,
               self.h,
               self.l,
               self.sp,
               self.is_flag_set(ZL),
               self.is_flag_set(NLV),
               self.is_flag_set(HL),
               self.is_flag_set(CL),
               self.mem[self.sp as usize + 1],
               self.mem[self.sp as usize],
               self.mem[self.hl() as usize]);

        let old_pc = self.pc;

        if log::log_enabled!(log::Level::Trace) {
            //Then execute instruction
            let (inst_name, inst_len) = pp_opcode(first_byte, second_byte, third_byte, self.pc);
            let next_pc = (Wrapping(old_pc) + Wrapping(inst_len as u16)).0;
            match inst_len {
                1 => {
                    trace!(
                        "Running {:02x}    :  -> 0x{:04X}: {:<20}",
                        first_byte,
                        self.pc,
                        inst_name
                    )
                }
                2 => {
                    trace!(
                        "Running {:02x}{:02x}  :  -> 0x{:04X}: {:<20} 0x{:02X}  ; next 0x{:04X}",
                        first_byte,
                        second_byte,
                        self.pc,
                        inst_name,
                        second_byte,
                        next_pc
                    )
                }
                3 => {
                    trace!("Running {:02x}{:02x}{:02x}:  -> 0x{:04X}: {:<20} 0x{:02X} 0x{:02X}  ; next 0x{:04X}",
                       first_byte,
                       second_byte,
                       third_byte,
                       self.pc,
                       inst_name,
                       second_byte,
                       third_byte,
                       next_pc)
                }
                n => error!("Instruction with impossible length: {:?}", n),
            }
        }

        let uf = "The impossible happened!";

        if first_byte == 0xCB {
            //prefixed instruction
            let x = (second_byte >> 6) & 0x3;
            let y = (second_byte >> 3) & 0x7;
            let z = second_byte & 0x7;
            let mut inst_time_set = false;

            match x {
                // xxyy yzzz
                0 => {
                    match y {
                        //(cpu_dispatch(z))
                        0 => self.rlc(cpu_dispatch(z)),
                        1 => self.rrc(cpu_dispatch(z)),
                        2 => self.rl(cpu_dispatch(z)),
                        3 => self.rr(cpu_dispatch(z)),
                        4 => self.sla(cpu_dispatch(z)),
                        5 => self.sra(cpu_dispatch(z)),
                        6 => self.swap(cpu_dispatch(z)),
                        7 => self.srl(cpu_dispatch(z)),
                        _ => unreachable!("{}", uf),
                    }
                }

                1 => {
                    self.bit(y, cpu_dispatch(z));
                    inst_time = if CpuRegister::HL == cpu_dispatch(z) {
                        12
                    } else {
                        8
                    };
                    inst_time_set = true;
                }

                2 => self.res(y, cpu_dispatch(z)),

                3 => self.set(y, cpu_dispatch(z)),

                _ => unreachable!("{}", uf),
            }

            if !inst_time_set {
                inst_time = if CpuRegister::HL == cpu_dispatch(z) {
                    16
                } else {
                    8
                };
            }

            self.inc_pc();
        } else {
            //unprefixed instruction
            match x {
                0 => match z {
                    0 => match y {
                        0 => self.nop(), //0x00
                        1 => {
                            self.ldnnsp(second_byte, third_byte);
                            self.inc_pc();
                            self.inc_pc();
                            inst_time = 20;
                        } //0x08
                        2 => self.stop(), //0x10
                        3 => {
                            self.jrn(second_byte as i8);
                            self.inc_pc();
                            inst_time = 12;
                        } //0x18
                        v @ 4..=7 => {
                            inst_time = 8 + if self.jrccn(cc_dispatch(v - 4), second_byte as i8) {
                                4
                            } else {
                                0
                            };
                            self.inc_pc();
                        } //0x20, 0x28, 0x30, 0x38
                        _ => unreachable!("{}", uf),
                    },

                    1 =>
                    //00yy y001
                    {
                        inst_time = if y % 2 == 0 {
                            self.ldnnn16(cpu16_dispatch(y / 2), second_byte, third_byte);
                            self.inc_pc();
                            self.inc_pc();
                            12
                        } else {
                            self.add_hl(cpu16_dispatch(y / 2));
                            8
                        };
                    }

                    2 =>
                    //00yy y010
                    {
                        match y {
                            0 | 2 => self.ldna16(cpu16_dispatch(y / 2)),
                            1 | 3 => self.ldan16(cpu16_dispatch(y / 2)),
                            4 => self.ldihla(),
                            5 => self.ldiahl(),
                            6 => self.lddhla(),
                            7 => self.lddahl(),
                            _ => unreachable!("{}", uf),
                        }
                        inst_time = 8;
                    }

                    3 =>
                    //00yy y011
                    {
                        even_odd_dispatch!(
                            y,
                            self,
                            inc16,
                            dec16,
                            cpu16_dispatch,
                            cpu16_dispatch,
                            1,
                            1
                        );
                        inst_time = 8;
                    }

                    4 =>
                    //00yy y100
                    {
                        self.inc(cpu_dispatch(y));
                        inst_time = if cpu_dispatch(y) == CpuRegister::HL {
                            12
                        } else {
                            4
                        };
                    }

                    5 => {
                        self.dec(cpu_dispatch(y));
                        inst_time = if cpu_dispatch(y) == CpuRegister::HL {
                            12
                        } else {
                            4
                        };
                    }

                    6 => {
                        self.ldnnn(cpu_dispatch(y), second_byte);
                        self.inc_pc();
                        inst_time = if cpu_dispatch(y) == CpuRegister::HL {
                            12
                        } else {
                            8
                        };
                    }

                    7 => match y {
                        //00yy y111
                        0 => self.rlca(),
                        1 => self.rrca(),
                        2 => self.rla(),
                        3 => self.rra(),
                        4 => self.daa(),
                        5 => self.cpl(),
                        6 => self.scf(),
                        7 => self.ccf(),
                        _ => unreachable!("{}", uf),
                    },

                    _ => unreachable!("{}", uf),
                }, //end x=0

                1 => match (z, y) {
                    (6, 6) => self.halt(),
                    (n, m) => {
                        self.ldr1r2(cpu_dispatch(m), cpu_dispatch(n));
                        inst_time = match (cpu_dispatch(m), cpu_dispatch(n)) {
                            (CpuRegister::HL, _) | (_, CpuRegister::HL) => 8,
                            _ => 4,
                        }
                    }
                }, //end x = 1

                2 => {
                    match y //10yy y000
                       {
                           0 => self.add(cpu_dispatch(z)),
                           1 => self.adc(cpu_dispatch(z)),
                           2 => self.sub(cpu_dispatch(z)),
                           3 => self.sbc(cpu_dispatch(z)),
                           4 => self.and(cpu_dispatch(z)),
                           5 => self.xor(cpu_dispatch(z)),
                           6 => self.or(cpu_dispatch(z)),
                           7 => self.cp(cpu_dispatch(z)),
                           _ => unreachable!("{}", uf),
                       };
                    //TODO: double check the line below
                    inst_time = if z == 6 { 8 } else { 4 };
                } //end x = 2

                3 => {
                    match z //11yy y000
                {

                    0 => match y {
                        v @ 0..=3 => inst_time = if
                            self.retcc(cc_dispatch(v)) {20} else {8},
                        4 => {
                            self.ldhna(second_byte);
                            self.inc_pc();
                            inst_time = 12;
                        },
                        5 => { //0xE8
                            self.addspn(second_byte as i8);
                            self.inc_pc();
                            inst_time = 16;
                        },
                        6 => {
                            // F0
                            self.ldhan(second_byte);
                            self.inc_pc();
                            inst_time = 12;
                        },
                        7 => {
                            self.ldhlspn(second_byte as i8);
                            self.inc_pc();
                            inst_time = 12;
                        },
                        _ => unreachable!("{}", uf),
                    },

                    1 => if y % 2 == 0 { //11yy y001
                        let adjusted_value = y / 2;
                        let val = self.pop_from_stack();
                        self.set_register16(cpu16_dispatch_push_pop(adjusted_value), val);
                        inst_time = 12;
                    } else {
                        let adjusted_value = y / 2;
                        match adjusted_value {
                            0 => {
                                self.ret();
                                inst_time = 16;
                            },
                            1 => {
                                self.reti();
                                inst_time = 16;
                            },
                            2 => self.jphl(),
                            3 => {
                                self.ldsphl();
                                inst_time = 8;
                            },
                            _ => unreachable!("{}", uf),
                        }
                    },

                    2 => match y {
                        v @ 0..=3 => { // 11yy y010
                            let const_val = (second_byte as u16) | ((third_byte as u16) << 8);
                            inst_time = if
                                self.jpccnn(cc_dispatch(v),
                                            const_val) {16} else {12};
                            self.inc_pc();
                            self.inc_pc();
                        },
                        4 => { // 0xE2
                            self.ldca();
                            inst_time = 8;
                        },
                        5 => { //0xEA
                            self.ldna16c(second_byte, third_byte);
                            self.inc_pc();
                            self.inc_pc();
                            inst_time = 16;
                        },
                        6 => { //0xF2
                            self.ldac();
                            inst_time = 8;
                        },
                        7 => { //0xFA
                            self.ldan16c(second_byte, third_byte);
                            self.inc_pc();
                            self.inc_pc();
                            inst_time = 16;
                        },
                        _ => unreachable!("{}", uf),
                    },

                    3 => match y { //11yy y011
                        0 => {
                            let const_val = (second_byte as u16) | ((third_byte as u16) << 8);
                            self.jpnn(const_val);
                            self.inc_pc();
                            self.inc_pc();

                            inst_time = 16;
                        },
                        6 => self.di(),
                        7 => self.ei(),
                        _ => {
                            let pc = self.pc;
                            self.crash(format!("Invalid opcode: {:X} at {:X}", first_byte, pc));
                        },
                    },

                    4 => {
                        match y {
                            0..=3 => {
                                let const_val = (second_byte as u16) | ((third_byte as u16) << 8);
                                inst_time =
                                    if self.callccnn(cc_dispatch(y),
                                                     const_val) {24} else {12};
                                self.inc_pc();
                                self.inc_pc();
                            },
                            _ => {
                                let pc = self.pc;
                                self.crash(format!("Invalid opcode: {:X} at {:X}", first_byte, pc));
                            },
                        }
                    },

                    5 => {
                        if y % 2 == 0 {
                            let value = self.access_register16(cpu16_dispatch_push_pop(y / 2));
                            self.push_onto_stack(value);
                            inst_time = 16;
                        } else if y == 1 {
                            let const_val = (second_byte as u16) | ((third_byte as u16) << 8);
                            self.callnn(const_val);
                            self.inc_pc();
                            self.inc_pc();
                            inst_time = 24;
                        } else {
                            let pc = self.pc;
                            self.crash(format!("Invalid opcode: {:X} at {:X}", first_byte, pc));
                        }
                    },

                    6 => {
                        match y {
                            0 => self.add(CpuRegister::Num(second_byte as byte)),
                            1 => self.adc(CpuRegister::Num(second_byte as byte)),
                            2 => self.sub(CpuRegister::Num(second_byte as byte)),
                            3 => self.sbc(CpuRegister::Num(second_byte as byte)),
                            4 => self.and(CpuRegister::Num(second_byte as byte)),
                            5 => self.xor(CpuRegister::Num(second_byte as byte)),
                            6 => self.or(CpuRegister::Num(second_byte as byte)),
                            7 => self.cp(CpuRegister::Num(second_byte as byte)),
                            _ => unreachable!("{}", uf),
                        };
                        inst_time = 8;
                        self.inc_pc();
                    },

                    7 => {
                        self.rst(8*y);
                        inst_time = 16;
                    },

                    _ => unreachable!("{}", uf),
                }
                }
                _ => panic!("The impossible happened!"),
            }
        }

        self.inc_pc();

        self.cycles = (Wrapping(self.cycles) + Wrapping(inst_time as u64)).0;
        self.maybe_disable_interrupts();

        inst_time
    }

    pub fn crash(&mut self, info: String) {
        error!("{}", info);
        self.state = CpuState::Crashed;
    }

    /// Loads the ROM with saved RAM if its available
    pub fn load_rom(&mut self, rom_bytes: Vec<u8> /*, data_path: Option<PathBuf>*/) {
        trace!("Loading ROM");
        self.mem.load(rom_bytes);
        self.gbc_mode = self.mem.gbc_mode();
        if self.gbc_mode {
            self.mem.set_gbc_mode();
        }
        self.sgb_mode = self.mem.gbc_mode();
        self.reset();

        // TODO: revisit where this code should live when properly implementing
        // resuming from saves / savestates.
        /*
        // load RAM if it exists
        if let Some(data_location) = data_path {
            let game_name = self.get_game_name();
            self.mem.load_saved_ram(data_location, game_name.as_ref());
        }
        */

        // disable for now
        //self.mem.initialize_logger();
    }

    pub fn save_ram(&self, data_path: Option<PathBuf>) {
        trace!("Saving RAM");

        if let Some(data_location) = data_path {
            let game_name = self.get_game_name();
            self.mem.save_ram(data_location, game_name.as_ref());
        }
    }

    pub fn remove_old_events(&mut self) {
        use crate::io::constants::FADE_DELAY;

        let event_logger = match self.mem.logger {
            Some(ref mut logger) => logger,
            None => return,
        };

        // Remove events that are too old
        while !event_logger.events_deq.is_empty() {
            let timestamp = event_logger.events_deq.front().unwrap().timestamp;
            if (Wrapping(self.cycles) - Wrapping(timestamp)).0 >= FADE_DELAY {
                event_logger.events_deq.pop_front();
            } else {
                break;
            }
        }
    }
}
