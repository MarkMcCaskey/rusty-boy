//! The wrapper around the information needed to meaningfully run this program
//!
//! NOTE: in the process of further abstracting IO logic with this --
//! expect things to break

use std;

use crate::cpu;
use crate::io::constants::*;

use crate::io::deferred_renderer::deferred_renderer_draw_scanline;
use crate::io::graphics::renderer::Renderer;

use std::num::Wrapping;

/// Holds all the data needed to use the emulator in meaningful ways
pub struct ApplicationState {
    pub gameboy: cpu::Cpu,
    //sound_system: AudioDevice<GBSound>,
    //renderer: render::Renderer<'static>,
    cycle_count: u64,
    prev_time: u64,
    /// counts cycles since last timer update
    timer_cycles: u64,
    /// counts cycles since last divider register update
    div_timer_cycles: u64,
    /// counts cycles since last sound update
    _sound_cycles: u64,
    _screenshot_frame_num: Wrapping<u64>,
    pub renderer: Box<dyn Renderer>,
}

impl ApplicationState {
    //! Sets up the environment for running in memory visualization mode
    pub fn new(renderer: Box<dyn Renderer>) -> Result<ApplicationState, String> {
        // Set up gameboy and other state
        let gameboy = cpu::Cpu::new();

        Ok(ApplicationState {
            gameboy,
            //sound_system: device,
            cycle_count: 0,
            prev_time: 0,
            // FIXME sound_cycles is probably wrong or not needed
            _sound_cycles: 0,
            timer_cycles: 0,
            div_timer_cycles: 0,
            _screenshot_frame_num: Wrapping(0),
            renderer,
        })
    }

    /// Runs the emulator for 1 frame and requests that frame to be drawn.
    pub fn step(&mut self) {
        let (
            _cycles_per_vblank,
            _cycles_per_hsync,
            cycles_per_second,
            cycles_per_divider_step,
            oam_scan_cycles,
            vram_scan_cycles,
            hblank_cycles,
        ) = if self.gameboy.gbc_mode && self.gameboy.double_speed {
            (
                CPU_CYCLES_PER_VBLANK * 2,
                CYCLES_PER_HSYNC * 2,
                CPU_CYCLES_PER_SECOND * 2,
                CPU_CYCLES_PER_DIVIDER_STEP * 2,
                80 * 2,
                168 * 2,
                208 * 2,
            )
        } else {
            (
                CPU_CYCLES_PER_VBLANK,
                CYCLES_PER_HSYNC,
                CPU_CYCLES_PER_SECOND,
                CPU_CYCLES_PER_DIVIDER_STEP,
                80,
                168,
                208,
            )
        };
        let mut scanline_cycles: u32 = 0;
        let mut y = 0;
        let mut window_counter: u16 = 0;
        let mut vblank_iterations = 0;

        #[derive(Debug, Clone, Copy)]
        enum GameBoyMode {
            /// Mode 2
            OamScan,
            /// Mode 3
            VramScan,
            /// Mode 0
            HBlank,
            /// Mode 1
            VBlank,
        }

        let mut mode = GameBoyMode::OamScan;
        self.gameboy.set_oam_lock();
        if self.gameboy.get_interrupts_enabled()
            && self.gameboy.get_lcdc_interrupt_enabled()
            && self.gameboy.get_oam_interrupt()
        {
            // TODO: I don't think any of this `if` stuff matters given how it's done
            // clean up:
            // interrupts are only triggered on a rising edge
            if !self.gameboy.get_lcdc_interrupt_bit() {
                self.gameboy.set_lcdc_interrupt_bit();
            }
        }
        if self.gameboy.ly() != 0 {
            self.gameboy.inc_ly();
            assert_eq!(self.gameboy.ly(), 0);
        }
        let mut frame = [[(0u8, 0u8, 0u8); GB_SCREEN_WIDTH]; GB_SCREEN_HEIGHT];
        'steploop: loop {
            let mut cycles_this_loop = 0;
            match mode {
                GameBoyMode::OamScan => {
                    if scanline_cycles < oam_scan_cycles {
                        cycles_this_loop = self.gameboy.dispatch_opcode() as u32;
                        scanline_cycles += cycles_this_loop;
                        self.cycle_count += cycles_this_loop as u64;
                    } else {
                        mode = GameBoyMode::VramScan;
                        self.gameboy.set_oam_and_display_lock();
                    }
                }
                GameBoyMode::VramScan => {
                    if scanline_cycles < (vram_scan_cycles + oam_scan_cycles) {
                        cycles_this_loop = self.gameboy.dispatch_opcode() as u32;
                        scanline_cycles += cycles_this_loop;
                        self.cycle_count += cycles_this_loop as u64;
                    } else {
                        mode = GameBoyMode::HBlank;
                        self.gameboy.set_hblank();
                        if self.gameboy.get_interrupts_enabled()
                            && self.gameboy.get_lcdc_interrupt_enabled()
                            && self.gameboy.get_hblank_interrupt()
                        {
                            // interrupts are only triggered on a rising edge
                            if !self.gameboy.get_lcdc_interrupt_bit() {
                                self.gameboy.set_lcdc_interrupt_bit();
                            }
                        }
                    }
                }
                GameBoyMode::HBlank => {
                    if scanline_cycles < (hblank_cycles + vram_scan_cycles + oam_scan_cycles) {
                        cycles_this_loop = self.gameboy.dispatch_opcode() as u32;
                        scanline_cycles += cycles_this_loop;
                        self.cycle_count += cycles_this_loop as u64;
                    } else {
                        // TODO: split out render logic into here so we can maintain timer state, etc.
                        // All dispatch_opcodes need to be overseen by the appropriate external timing stuff
                        let scanline = deferred_renderer_draw_scanline(
                            y,
                            &mut self.gameboy,
                            &mut window_counter,
                        );

                        frame[y as usize] = scanline;
                        y += 1;
                        self.gameboy.inc_ly();
                        //run_inc_ly_logic(&mut self.gameboy);

                        scanline_cycles -= hblank_cycles + vram_scan_cycles + oam_scan_cycles;
                        assert!(
                            scanline_cycles < hblank_cycles + vram_scan_cycles + oam_scan_cycles
                        );
                        if y == (GB_SCREEN_HEIGHT as u8) {
                            self.gameboy.set_vblank();
                            if self.gameboy.get_interrupts_enabled() {
                                if self.gameboy.get_vblank_interrupt_enabled() {
                                    self.gameboy.set_vblank_interrupt_bit();
                                }
                                if self.gameboy.get_lcdc_interrupt_enabled()
                                    && self.gameboy.get_vblank_interrupt_stat()
                                {
                                    self.gameboy.set_lcdc_interrupt_bit();
                                }
                            }
                            mode = GameBoyMode::VBlank;
                        } else {
                            mode = GameBoyMode::OamScan;
                            self.gameboy.set_oam_lock();
                            if self.gameboy.get_interrupts_enabled()
                                && self.gameboy.get_lcdc_interrupt_enabled()
                                && self.gameboy.get_oam_interrupt()
                            {
                                if !self.gameboy.get_lcdc_interrupt_bit() {
                                    self.gameboy.set_lcdc_interrupt_bit();
                                }
                            }
                        }
                    }
                }
                GameBoyMode::VBlank => {
                    // TOOD: verify if this is correct, should LY roll over to 0 at the end of the frame or the start of a new frame?
                    if vblank_iterations == 9 {
                        //10 {
                        //assert_eq!(self.gameboy.ly(), 153);

                        // REVIEW: why is this here?
                        //self.gameboy.set_oam_and_display_lock();

                        self.prev_time = self.cycle_count;

                        //for memory visualization
                        self.gameboy.remove_old_events();

                        // do render of frame to screen here
                        self.renderer.draw_frame(&frame);

                        break 'steploop;
                    }
                    if scanline_cycles < (hblank_cycles + vram_scan_cycles + oam_scan_cycles) {
                        cycles_this_loop = self.gameboy.dispatch_opcode() as u32;
                        scanline_cycles += cycles_this_loop;
                        self.cycle_count += cycles_this_loop as u64;
                    } else {
                        scanline_cycles -= hblank_cycles + vram_scan_cycles + oam_scan_cycles;
                        self.gameboy.inc_ly();
                        //run_inc_ly_logic(&mut self.gameboy);
                        vblank_iterations += 1;
                    }
                }
            }

            // FF04 (DIV) Divider Register stepping
            self.div_timer_cycles += cycles_this_loop as u64;
            while self.div_timer_cycles >= cycles_per_divider_step {
                self.gameboy.inc_div();
                self.div_timer_cycles -= cycles_per_divider_step;
            }

            // FF05 (TIMA) Timer counter stepping
            self.timer_cycles += cycles_this_loop as u64;
            let timer_hz = self.gameboy.timer_frequency_hz();
            let cpu_cycles_per_timer_counter_step =
                (cycles_per_second as f64 / (timer_hz as f64)) as u64;
            while self.timer_cycles >= cpu_cycles_per_timer_counter_step {
                self.gameboy.timer_cycle();
                self.timer_cycles -= cpu_cycles_per_timer_counter_step;
            }
        }
    }
}
