//! The wrapper around the information needed to meaningfully run this program
//!
//! NOTE: in the process of further abstracting IO logic with this --
//! expect things to break

use std;

use crate::cpu;
use crate::gba::DmaStartTiming;
use crate::io::constants::*;

use crate::io::deferred_renderer::deferred_renderer_draw_scanline;
use crate::io::deferred_renderer_gba::deferred_renderer_draw_gba_scanline;
use crate::io::graphics::renderer::Renderer;

use std::num::Wrapping;

/// Holds all the data needed to use the emulator in meaningful ways
pub struct ApplicationState {
    pub gameboy: cpu::Cpu,
    //pub gba: Option<crate::gba::GameboyAdvance>,
    pub gba: crate::gba::GameboyAdvance,
    //sound_system: AudioDevice<GBSound>,
    //renderer: render::Renderer<'static>,
    cycle_count: u64,
    prev_time: u64,
    /// counts cycles since last timer update
    timer_cycles: u64,
    /// counts cycles since last divider register update
    div_timer_cycles: u64,
    /// How many CPU cycles per second
    cycles_per_second: u64,
    /// counts cycles since last sound update
    sound_cycles: u64,
    /// counts cycles since last GBA sound update
    gba_sound_cycles: u64,
    _screenshot_frame_num: Wrapping<u64>,
    gba_timers: [u32; 4],
    debug_gba_last_seen_ppu_bg_mode: Option<u8>,
    pub renderer: Box<dyn Renderer>,
}

impl ApplicationState {
    //! Sets up the environment for running in memory visualization mode
    pub fn new(renderer: Box<dyn Renderer>) -> Result<ApplicationState, String> {
        // Set up gameboy and other state
        let gameboy = cpu::Cpu::new();
        let gba = crate::gba::GameboyAdvance::new(false);

        Ok(ApplicationState {
            gameboy,
            //gba: Some(gba),
            gba,
            //sound_system: device,
            cycle_count: 0,
            prev_time: 0,
            timer_cycles: 0,
            div_timer_cycles: 0,
            cycles_per_second: CPU_CYCLES_PER_SECOND,
            sound_cycles: 0,
            gba_sound_cycles: 0,
            _screenshot_frame_num: Wrapping(0),
            gba_timers: [0, 0, 0, 0],
            debug_gba_last_seen_ppu_bg_mode: None,
            renderer,
        })
    }

    /*
    fn update_channel_vars(&mut self) {
        self.channel1_sweep_pace = self.gameboy.channel1_sweep_pace();
        self.channel1_envelope_pace = self.gameboy.channel1_envelope_sweep_pace();
        self.channel2_envelope_pace = self.gameboy.channel2_envelope_sweep_pace();
        self.channel4_envelope_pace = self.gameboy.channel4_envelope_sweep_pace();
    }
    */

    pub fn step_gba(&mut self) {
        //let cycles_per_frame = 83776 + (160 * /*1232*/ 960);
        // TODO: derive this more properly
        let cycles_per_frame = 279709;
        let audio_timing_cycles = (cycles_per_frame * 60) / 512;
        let gba_audio_timing_cycles = 512; //(cycles_per_frame * 60) / 32768;
                                           //let audio_timing_cycles = (cycles_per_frame * 60) / 32768;
        let mut fifo_a_sample_counter = 0;
        let mut fifo_b_sample_counter = 0;
        //let audio_timing_cycles = (cycles_per_frame * 4) / 32768;
        let mut cycles = 0;
        let mut frame = [[(0u8, 0u8, 0u8); GBA_SCREEN_WIDTH]; GBA_SCREEN_HEIGHT];
        let mut y = 0;

        #[derive(Debug, Clone, Copy)]
        enum GameBoyAdvanceMode {
            HBlank,
            VBlank,
        }
        let mut in_hblank = false;
        let mut hblank_cycles = 0;
        // we need this?
        self.gba.ppu_set_vblank(false);
        self.gba.ppu_set_readonly_vcounter(0);
        if y == self.gba.ppu_vcounter_setting()
            && self.gba.ppu_vcounter_irq_enabled()
            && self.gba.master_interrupts_enabled()
        {
            self.gba.set_lcdc_vcounter_interrupt(true);
        }

        while y < 227 {
            let cycles_from_opcode = self.gba.dispatch() as u64;
            cycles += cycles_from_opcode; // * 2;
            hblank_cycles += cycles_from_opcode; // * 2;

            for (i, timer) in self.gba_timers.iter_mut().enumerate() {
                if !self.gba.io_registers.timer_enabled(i as u8) {
                    continue;
                }
                let timer_prescaler = match i {
                    0 => self.gba.io_registers.timer0_prescaler() as u32,
                    1 => self.gba.io_registers.timer1_prescaler() as u32,
                    2 => self.gba.io_registers.timer2_prescaler() as u32,
                    3 => self.gba.io_registers.timer3_prescaler() as u32,
                    _ => unreachable!(),
                };
                //*timer = timer.saturating_add(cycles_from_opcode as u32);
                *timer += cycles_from_opcode as u32;
                while *timer >= timer_prescaler {
                    *timer -= timer_prescaler;
                    if self.gba.io_registers.increment_timer(i as u8) {
                        //dbg!(i, timer_prescalers[i]);
                        // NOTE: this condition may be incorrect,
                        // we may want to still `inc_note` even if DMA is enabled.
                        // This should be a very small edge case though and if this is, in fact,
                        // incorerct then we would just expect to drop at most 32 samples which would
                        // be hard to notice.
                        if self.gba.io_registers.dma1_enabled || self.gba.io_registers.dma2_enabled
                        {
                            if i == 1 && self.gba.io_registers.sound_a_timer1 {
                                //debug_assert!(self.gba.io_registers.apu.gba_sound_a_enabled.0 || self.gba.io_registers.apu.gba_sound_a_enabled.1);
                                self.gba.io_registers.apu.gba_fifo_a.inc_note();
                                fifo_a_sample_counter += 1;
                                if self.gba.io_registers.apu.gba_fifo_a.ready_for_more_data() {
                                    self.gba.io_registers.trigger_sound_a_dma();
                                }
                            } else if i == 0 && !self.gba.io_registers.sound_a_timer1 {
                                //debug_assert!(self.gba.io_registers.apu.gba_sound_a_enabled.0 || self.gba.io_registers.apu.gba_sound_a_enabled.1);
                                self.gba.io_registers.apu.gba_fifo_a.inc_note();
                                fifo_a_sample_counter += 1;
                                if self.gba.io_registers.apu.gba_fifo_a.ready_for_more_data() {
                                    self.gba.io_registers.trigger_sound_a_dma();
                                }
                            }
                            if i == 1 && self.gba.io_registers.sound_b_timer1 {
                                //debug_assert!(self.gba.io_registers.apu.gba_sound_b_enabled.0 || self.gba.io_registers.apu.gba_sound_b_enabled.1);
                                self.gba.io_registers.apu.gba_fifo_b.inc_note();
                                fifo_b_sample_counter += 1;
                                if self.gba.io_registers.apu.gba_fifo_b.ready_for_more_data() {
                                    self.gba.io_registers.trigger_sound_b_dma();
                                }
                            } else if i == 0 && !self.gba.io_registers.sound_b_timer1 {
                                //debug_assert!(self.gba.io_registers.apu.gba_sound_b_enabled.0 || self.gba.io_registers.apu.gba_sound_b_enabled.1);
                                self.gba.io_registers.apu.gba_fifo_b.inc_note();
                                fifo_b_sample_counter += 1;
                                if self.gba.io_registers.apu.gba_fifo_b.ready_for_more_data() {
                                    self.gba.io_registers.trigger_sound_b_dma();
                                }
                            }
                        }
                        if self.gba.io_registers.timer_irq_enabled(i as u8)
                            && self.gba.master_interrupts_enabled()
                        {
                            //dbg!("TIMER INTERRUPT!");
                            self.gba.set_timer_interrupt(i as u8, true);
                        }
                    }
                }
            }

            if in_hblank {
                if hblank_cycles >= 272 {
                    in_hblank = false;
                    hblank_cycles -= 272;
                    self.gba.ppu_set_hblank(false);
                    y += 1;
                    self.gba.ppu_set_readonly_vcounter(y);
                    if y == self.gba.ppu_vcounter_setting()
                        && self.gba.ppu_vcounter_irq_enabled()
                        && self.gba.master_interrupts_enabled()
                    {
                        self.gba.set_lcdc_vcounter_interrupt(true);
                    }
                    if y == 160 {
                        self.gba.ppu_set_vblank(true);
                        if self.gba.ppu_vblank_irq_enabled() && self.gba.master_interrupts_enabled()
                        {
                            self.gba.set_lcdc_vblank_interrupt(true);
                        }
                        self.gba.io_registers.bg2_rotation.cached_x =
                            self.gba.io_registers.bg2_rotation.x;
                        self.gba.io_registers.bg2_rotation.cached_y =
                            self.gba.io_registers.bg2_rotation.y;
                        self.gba.io_registers.bg3_rotation.cached_x =
                            self.gba.io_registers.bg3_rotation.x;
                        self.gba.io_registers.bg3_rotation.cached_y =
                            self.gba.io_registers.bg3_rotation.y;
                    }
                }
            }

            if hblank_cycles >= 960 {
                hblank_cycles -= 960;
                if y < 160 {
                    let scanline = deferred_renderer_draw_gba_scanline(y, &mut self.gba);
                    frame[y as usize] = scanline;
                    self.gba.io_registers.bg2_rotation.cached_x +=
                        self.gba.io_registers.bg2_rotation.pb as i32;
                    self.gba.io_registers.bg2_rotation.cached_y +=
                        self.gba.io_registers.bg2_rotation.pd as i32;
                    self.gba.io_registers.bg3_rotation.cached_x +=
                        self.gba.io_registers.bg3_rotation.pb as i32;
                    self.gba.io_registers.bg3_rotation.cached_y +=
                        self.gba.io_registers.bg3_rotation.pd as i32;
                }

                in_hblank = true;
                self.gba.ppu_set_hblank(true);
                if self.gba.ppu_hblank_irq_enabled() && self.gba.master_interrupts_enabled() {
                    self.gba.set_lcdc_hblank_interrupt(true);
                }

                /*
                if y == 160 {
                    self.gba.ppu_set_vblank(true);
                    if self.gba.ppu_vblank_irq_enabled() && self.gba.master_interrupts_enabled() {
                        self.gba.set_lcdc_vblank_interrupt(true);
                    }
                    self.gba.io_registers.bg2_rotation.cached_x =
                        self.gba.io_registers.bg2_rotation.x;
                    self.gba.io_registers.bg2_rotation.cached_y =
                        self.gba.io_registers.bg2_rotation.y;
                    self.gba.io_registers.bg3_rotation.cached_x =
                        self.gba.io_registers.bg3_rotation.x;
                    self.gba.io_registers.bg3_rotation.cached_y =
                        self.gba.io_registers.bg3_rotation.y;
                }
                */

                if y >= 2 && y <= 163 {
                    if self.gba.io_registers.dma3_enabled {
                        let dma3 = self.gba.io_registers.dma3();
                        if dma3.start_timing == DmaStartTiming::Special {
                            self.gba.io_registers.trigger_dma(3);
                        }
                    }
                }
                if self.gba.io_registers.dma1_enabled {
                    let dma1 = self.gba.io_registers.dma1();
                    if dma1.start_timing == DmaStartTiming::HBlank {
                        self.gba.io_registers.trigger_dma(1);
                    }
                }
            }
            self.sound_cycles += cycles_from_opcode as u64;
            if self.sound_cycles >= audio_timing_cycles as u64 {
                self.renderer.audio_step(&self.gba.io_registers.apu);
                self.sound_cycles -= audio_timing_cycles as u64;

                //self.gba_sound_cycles -= gba_audio_timing_cycles as u64;
                //let rate = 4000. / 512.;
                let rate = 64. * 8.; // * 18.;//2000.;// / 512.;
                self.gba.io_registers.apu.gba_fifo_a_sample_rate =
                    fifo_a_sample_counter as f32 * rate; // / 0.00025;
                self.gba.io_registers.apu.gba_fifo_b_sample_rate =
                    fifo_b_sample_counter as f32 * rate; // / 0.00025;
                if fifo_b_sample_counter != 0
                    || self.gba.io_registers.apu.gba_fifo_b_sample_rate != 0.
                {
                    /*
                    dbg!(
                        fifo_b_sample_counter,
                        self.gba.io_registers.apu.gba_fifo_b_sample_rate
                    );
                    */
                }
                self.renderer.gba_audio_step(&self.gba.io_registers.apu);
                self.gba.io_registers.apu.gba_fifo_a.clear_sound_buffer();
                fifo_a_sample_counter = 0;
                fifo_b_sample_counter = 0;
                self.gba.io_registers.apu.gba_fifo_a.reset_flag = false;
                self.gba.io_registers.apu.gba_fifo_b.reset_flag = false;
            }
            /*
            self.gba_sound_cycles += cycles_from_opcode as u64;
            if self.gba_sound_cycles >= gba_audio_timing_cycles as u64 {
                self.gba_sound_cycles -= gba_audio_timing_cycles as u64;
                //let rate = 4000. / 512.;
                let rate = 64.;//4000.;// / 512.;
                self.gba.io_registers.apu.gba_fifo_a_sample_rate = fifo_a_sample_counter as f32 * rate; // / 0.00025;
                self.gba.io_registers.apu.gba_fifo_b_sample_rate = fifo_b_sample_counter as f32 * rate; // / 0.00025;
                if fifo_b_sample_counter != 0 ||  self.gba.io_registers.apu.gba_fifo_b_sample_rate != 0. {
                    //dbg!(fifo_b_sample_counter, self.gba.io_registers.apu.gba_fifo_b_sample_rate);
                }
                fifo_a_sample_counter = 0;
                fifo_b_sample_counter = 0;
                self.renderer.gba_audio_step(&self.gba.io_registers.apu);
                self.gba.io_registers.apu.gba_fifo_a.clear_sound_buffer();
                self.gba.io_registers.apu.gba_fifo_b.clear_sound_buffer();
            }
            */
        }
        self.gba.ppu_set_vblank(false);
        if Some(self.gba.ppu_bg_mode()) != self.debug_gba_last_seen_ppu_bg_mode {
            debug!("PPU BG mode is {}", self.gba.ppu_bg_mode());
            self.debug_gba_last_seen_ppu_bg_mode = Some(self.gba.ppu_bg_mode());
        }
        /*
        if self.gba.ppu_bg_mode() != 0
            || self.gba.ppu_bg0_enabled()
            || self.gba.ppu_bg1_enabled()
            || self.gba.ppu_bg2_enabled()
            || self.gba.ppu_bg3_enabled()
            || self.gba.ppu_obj_enabled()
            || self.gba.ppu_win0_enabled()
            || self.gba.ppu_win1_enabled()
            || self.gba.ppu_obj_win_enabled()
        {
            dbg!(
                self.gba.ppu_bg_mode(),
                self.gba.ppu_bg0_enabled(),
                self.gba.ppu_bg1_enabled(),
                self.gba.ppu_bg2_enabled(),
                self.gba.ppu_bg3_enabled(),
                self.gba.ppu_obj_enabled(),
                self.gba.ppu_win0_enabled(),
                self.gba.ppu_win1_enabled(),
                self.gba.ppu_obj_win_enabled(),
            );
        }
        */
        self.renderer.draw_gba_frame(&frame);
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
        self.cycles_per_second = cycles_per_second;
        let audio_timing_cycles = cycles_per_second / 512; //256;
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
                        vblank_iterations += 1;
                    }
                }
            }

            // Audio timing
            self.sound_cycles += cycles_this_loop as u64;
            if self.sound_cycles >= audio_timing_cycles as u64 {
                // TODO: trigger this properly based on writes to registers
                //   and APU state (i.e. not here, somewhere CPU accessible)
                // HACK: we just update it randomly
                //self.update_channel_vars();
                self.renderer.audio_step(&self.gameboy.apu);
                self.sound_cycles -= audio_timing_cycles as u64;
            }

            // FF04 (DIV) Divider Register stepping
            self.div_timer_cycles += cycles_this_loop as u64;
            while self.div_timer_cycles >= cycles_per_divider_step {
                let old_div_val = self.gameboy.get_div();
                self.gameboy.inc_div();
                self.div_timer_cycles -= cycles_per_divider_step;

                let div_bit = if self.gameboy.double_speed { 5 } else { 4 };
                // TODO: div can be reset on write, this falling-edge
                //  detection logic does not see that

                // Update APU-DIV
                if (old_div_val >> div_bit) & 1 == 1 {
                    let new_div_val = self.gameboy.get_div();
                    if (new_div_val >> div_bit) & 1 == 0 {
                        // falling edge
                        self.gameboy.apu.step();
                    }
                }
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
