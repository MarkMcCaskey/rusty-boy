//! Audio logic

const APU_BASE: usize = 0xFF10;

#[derive(Clone)]
pub struct Apu {
    pub channel1_sweep_pace: u8,
    pub channel1_sweep_counter: u8,
    pub channel1_sweep_enabled: bool,
    pub channel1_frequency: u16,
    pub channel1_negate_executed: bool,
    pub channel1_envelope_pace: u8,
    pub channel1_envelope_counter: u8,
    pub channel1_envelope_increasing: bool,
    pub channel1_envelope_volume: u8,
    pub channel2_envelope_pace: u8,
    pub channel2_envelope_counter: u8,
    pub channel2_envelope_increasing: bool,
    pub channel2_envelope_volume: u8,
    pub channel4_envelope_pace: u8,
    pub channel4_envelope_counter: u8,
    pub channel4_envelope_increasing: bool,
    pub channel4_envelope_volume: u8,
    // TODO: each channel's NRX2, etc must be cached as changes to registers
    // don't take effect until the channel is triggered again
    pub div_apu: u8,
    /// 0xFF10..=0xFF3F
    pub apu_mem: [u8; 0x30],
}

impl Apu {
    pub fn new() -> Self {
        Self {
            channel1_sweep_pace: 0,
            channel1_sweep_counter: 8,
            channel1_sweep_enabled: true,
            channel1_frequency: 0,
            channel1_negate_executed: false,
            channel1_envelope_pace: 0,
            channel1_envelope_counter: 8,
            channel1_envelope_increasing: true,
            channel1_envelope_volume: 0,
            channel2_envelope_pace: 0,
            channel2_envelope_counter: 8,
            channel2_envelope_increasing: true,
            channel2_envelope_volume: 0,
            channel4_envelope_pace: 0,
            channel4_envelope_counter: 8,
            channel4_envelope_increasing: true,
            channel4_envelope_volume: 0,
            div_apu: 7,
            //div_apu: 0,
            apu_mem: [0; 0x30],
        }
    }

    pub fn reset(&mut self, sgb_mode: bool) {
        self.apu_mem[0xFF10 - APU_BASE] = 0x80;
        self.apu_mem[0xFF11 - APU_BASE] = 0xBF;
        self.apu_mem[0xFF12 - APU_BASE] = 0xF3;
        self.apu_mem[0xFF13 - APU_BASE] = 0xFF;
        self.apu_mem[0xFF14 - APU_BASE] = 0xBF;
        self.apu_mem[0xFF16 - APU_BASE] = 0x3F;
        self.apu_mem[0xFF17 - APU_BASE] = 0x00;
        self.apu_mem[0xFF18 - APU_BASE] = 0xFF;
        self.apu_mem[0xFF19 - APU_BASE] = 0xBF;
        self.apu_mem[0xFF1A - APU_BASE] = 0x7F;
        self.apu_mem[0xFF1B - APU_BASE] = 0xFF;
        self.apu_mem[0xFF1C - APU_BASE] = 0x9F;
        self.apu_mem[0xFF1E - APU_BASE] = 0xBF;
        self.apu_mem[0xFF20 - APU_BASE] = 0xFF;
        self.apu_mem[0xFF21 - APU_BASE] = 0x00;
        self.apu_mem[0xFF22 - APU_BASE] = 0x00;
        self.apu_mem[0xFF23 - APU_BASE] = 0xBF;
        self.apu_mem[0xFF24 - APU_BASE] = 0x77;
        self.apu_mem[0xFF25 - APU_BASE] = 0xF3;
        self.apu_mem[0xFF26 - APU_BASE] = if sgb_mode { 0xF0 } else { 0xF1 };

        self.channel1_sweep_pace = self.channel1_sweep_pace();
        self.channel1_envelope_pace = self.channel1_envelope_sweep_pace();
        self.channel1_envelope_increasing = self.channel1_envelope_increasing();
        self.channel1_envelope_volume = self.channel1_envelope_volume();
        self.channel1_frequency = self.channel1_frequency();
        self.channel2_envelope_pace = self.channel2_envelope_sweep_pace();
        self.channel2_envelope_increasing = self.channel2_envelope_increasing();
        self.channel2_envelope_volume = self.channel2_envelope_volume();
        self.channel4_envelope_pace = self.channel4_envelope_sweep_pace();
        self.channel4_envelope_increasing = self.channel4_envelope_increasing();
        self.channel4_envelope_volume = self.channel4_envelope_volume();

        self.div_apu = 7;
        //self.div_apu = 0;
        self.channel1_sweep_counter = 8;
        self.channel1_envelope_counter = 8;
        self.channel2_envelope_counter = 8;
        self.channel4_envelope_counter = 8;

        // TOOD: rest of the reset
    }

    pub fn step(&mut self) {
        self.div_apu = (self.div_apu + 1) & 0x7;
        if self.div_apu == 7 {
            // envelope sweep
            if self.get_sound1() {
                self.channel1_envelope_counter -= 1;
                if self.channel1_envelope_counter == 0 {
                    if self.channel1_envelope_pace != 0 {
                        self.channel1_envelope_pace = self.channel1_envelope_sweep_pace();
                        self.channel1_envelope_counter = self.channel1_envelope_pace;
                        self.channel1_step_envelope();
                    } else {
                        self.channel1_envelope_counter = 8;
                    }
                }
            }
            if self.get_sound2() {
                self.channel2_envelope_counter -= 1;
                if self.channel2_envelope_counter == 0 {
                    if self.channel2_envelope_pace != 0 {
                        self.channel2_envelope_pace = self.channel2_envelope_sweep_pace();
                        self.channel2_envelope_counter = self.channel2_envelope_pace;
                        self.channel2_step_envelope();
                    } else {
                        self.channel2_envelope_counter = 8;
                    }
                }
            }
            if self.get_sound4() {
                self.channel4_envelope_counter -= 1;
                if self.channel4_envelope_counter == 0 {
                    if self.channel4_envelope_pace != 0 {
                        self.channel4_envelope_pace = self.channel4_envelope_sweep_pace();
                        self.channel4_envelope_counter = self.channel4_envelope_pace;
                        self.channel4_step_envelope();
                    } else {
                        self.channel4_envelope_counter = 8;
                    }
                }
            }
        }
        // trigger on 2 and 6
        if (self.div_apu & 0x3) == 0x2 {
            // channel1 sweep logic
            if self.channel1_sweep_enabled && self.get_sound1() {
                self.channel1_sweep_counter = self.channel1_sweep_counter.wrapping_sub(1);
                if self.channel1_sweep_counter == 0 {
                    // checking the regiter value and not the cached value is required
                    // to pass CGB sound test 5
                    if self.channel1_sweep_pace() != 0 {
                        self.channel1_sweep_pace = self.channel1_sweep_pace();
                        self.channel1_sweep_counter = self.channel1_sweep_pace;
                        self.channel1_sweep_step();
                    } else {
                        self.channel1_sweep_counter = 8;
                    }
                }
            }
        }
        // trigger on every other time
        if self.div_apu & 1 == 0 {
            self.channel1_inc_sound_length();
            self.channel2_inc_sound_length();
            self.channel3_inc_sound_length();
            self.channel4_inc_sound_length();
        }
    }

    /// CPU visible mem
    pub fn get_mem(&self, addr: u16) -> u8 {
        match addr {
            0xFF10 => self.apu_mem[0xFF10 - APU_BASE] | 0x80,
            0xFF11 => self.apu_mem[0xFF11 - APU_BASE] | 0x3F,
            // write only audio register
            0xFF13 => 0xFF,
            0xFF14 => self.apu_mem[0xFF14 - APU_BASE] | !0b0100_0000,
            // NR20: unused channel 2 register
            0xFF15 => 0xFF,
            0xFF16 => self.apu_mem[0xFF16 - APU_BASE] | !0b1100_0000,
            0xFF18 => 0xFF,
            0xFF19 => self.apu_mem[0xFF19 - APU_BASE] | !0b0100_0000,
            0xFF1A => self.apu_mem[0xFF1A - APU_BASE] | !0b1000_0000,
            0xFF1B => 0xFF,
            0xFF1C => self.apu_mem[0xFF1C - APU_BASE] | !0b0110_0000,
            0xFF1D => 0xFF,
            0xFF1E => self.apu_mem[0xFF1E - APU_BASE] | !0b0100_0000,
            // NR40: unused channel 4 register
            0xFF1F => 0xFF,
            0xFF20 => 0xFF,
            0xFF23 => self.apu_mem[0xFF23 - APU_BASE] | !0b0100_0000,
            0xFF26 => self.apu_mem[0xFF26 - APU_BASE] | !0b1000_1111,
            0xFF27..=0xFF2F => 0xFF,
            _ => self.apu_mem[addr as usize - APU_BASE],
        }
    }

    pub fn set_mem(&mut self, addr: u16, value: u8) {
        // writes are ignored if APU is off.
        // TODO: DMG allows writing to part of length registers
        if !self.get_sound_all() && (0xFF10..=0xFF25).contains(&addr) {
            return;
        }
        match addr {
            0xFF10 => {
                let old_sweep_pace = self.channel1_sweep_pace();
                let old_direction_is_subtract = !self.channel1_sweep_increase();
                self.apu_mem[0xFF10 - APU_BASE] = value & 0x7F;
                let new_sweep_pace = self.channel1_sweep_pace();
                let new_direction_is_add = self.channel1_sweep_increase();

                if old_sweep_pace == 0 && new_sweep_pace != 0 {
                    self.channel1_sweep_pace = self.channel1_sweep_pace();
                }
                if old_direction_is_subtract
                    && new_direction_is_add
                    && self.channel1_negate_executed
                {
                    self.unset_sound1();
                }
            }

            // channel 1: NR11
            0xFF11 => {
                self.apu_mem[0xFF11 - APU_BASE] = value;
            }

            // channel 2: NR21
            0xFF16 => {
                self.apu_mem[0xFF16 - APU_BASE] = value;
            }

            // channel 3: NR31
            0xFF1B => {
                self.apu_mem[0xFF1B - APU_BASE] = value;
            }

            // channel 4: NR41
            0xFF20 => {
                self.apu_mem[0xFF20 - APU_BASE] = value;
            }

            // channel 1: NR12
            0xFF12 => {
                if value >> 3 == 0 {
                    self.unset_sound1();
                }
                // TODO: writes here require retriggering to take effect
                self.apu_mem[0xFF12 - APU_BASE] = value;
            }

            // channel 2: NR22
            0xFF17 => {
                if value >> 3 == 0 {
                    self.unset_sound2();
                }
                self.apu_mem[0xFF17 - APU_BASE] = value;
            }

            // channel 4: NR42
            0xFF21 => {
                if value >> 3 == 0 {
                    self.unset_sound4();
                }
                self.apu_mem[0xFF21 - APU_BASE] = value;
            }

            // channel 3: NR30
            0xFF1A => {
                // DAC does not trigger the channel
                // but it can unset it
                if value >> 7 == 0 {
                    self.unset_sound3();
                }
                self.apu_mem[0xFF1A - APU_BASE] = value;
            }

            // channel 1: NR14
            0xFF14 => {
                let old_sound_length_enabled = self.channel1_sound_length_enabled();
                self.apu_mem[0xFF14 - APU_BASE] = value;

                if self.channel1_sound_length_enabled()
                    && !old_sound_length_enabled
                    && self.div_apu & 1 != 1
                    && self.channel1_sound_length() != 0
                {
                    self.channel1_inc_sound_length();
                }
                if value >> 7 == 1 {
                    if self.channel1_sound_length() == 0 {
                        //self.set_channel1_sound_length(0);

                        if self.channel1_sound_length_enabled() && self.div_apu & 1 != 1 {
                            self.channel1_inc_sound_length();
                        }
                    }
                    self.set_sound1();
                    // ensure DAC is enabled
                    if self.apu_mem[0xFF12 - APU_BASE] >> 3 == 0 {
                        self.unset_sound1();
                    }
                }
            }

            // channel 2: NR24
            0xFF19 => {
                let old_sound_length_enabled = self.channel2_sound_length_enabled();
                self.apu_mem[0xFF19 - APU_BASE] = value;

                if self.channel2_sound_length_enabled()
                    && !old_sound_length_enabled
                    && self.div_apu & 1 != 1
                    && self.channel2_sound_length() != 0
                {
                    self.channel2_inc_sound_length();
                }
                if value >> 7 == 1 {
                    if self.channel2_sound_length() == 0 {
                        //self.set_channel1_sound_length(0);

                        if self.channel2_sound_length_enabled() && self.div_apu & 1 != 1 {
                            self.channel2_inc_sound_length();
                        }
                    }
                    // ensure that the DAC is enabled here before triggering
                    if self.apu_mem[0xFF17 - APU_BASE] >> 3 != 0 {
                        self.set_sound2();
                    }
                }
            }

            // channel 3: NR34
            0xFF1E => {
                if value >> 7 == 1 {
                    // ensure that the DAC is enabled here before triggering
                    if self.apu_mem[0xFF1A - APU_BASE] >> 7 == 1 {
                        self.set_sound3();
                    }
                }
                self.apu_mem[0xFF1E - APU_BASE] = value;
            }

            // channel 4: NR44
            0xFF23 => {
                if value >> 7 == 1 {
                    // ensure that the DAC is enabled here before triggering
                    if self.apu_mem[0xFF21 - APU_BASE] >> 3 != 0 {
                        self.set_sound4();
                    }
                }
                self.apu_mem[0xFF23 - APU_BASE] = value;
            }

            // Sound
            // NR52
            0xFF26 => {
                if (value >> 7) & 1 == 0 {
                    self.unset_sound_all();
                } else if (value >> 7) & 1 == 1 {
                    // TODO: clear wave ram on power on too
                    self.set_sound_all();
                }
            }
            _ => {
                self.apu_mem[addr as usize - APU_BASE] = value;
            } //_ => panic!("unimplemented apu write: {:04X}", addr),
        }
    }

    /* sound */
    pub fn channel1_sweep_time(&self) -> f32 {
        (((self.apu_mem[0xFF10 - APU_BASE] >> 4) & 0x7) as f32) / 128.0
    }

    // number is multiplied by 128 and is the hz of how often it's updated.
    pub fn channel1_sweep_pace(&self) -> u8 {
        (self.apu_mem[0xFF10 - APU_BASE] >> 4) & 0x7
    }

    pub fn channel1_sweep_increase(&self) -> bool {
        ((self.apu_mem[0xFF10 - APU_BASE] >> 3) & 1) == 0
    }

    pub fn channel1_sweep_shift(&self) -> u8 {
        self.apu_mem[0xFF10 - APU_BASE] & 0x7
    }

    fn channel1_sweep_step_overflow_logic(&mut self) {
        // extra compute on shadow register
        let freq = self.channel1_frequency;
        let shift = self.channel1_sweep_shift();
        if self.channel1_sweep_increase() {
            let n = freq + (freq >> shift);
            if n > 0x7FF {
                self.unset_sound1();
                return;
            }
        } else {
            self.channel1_negate_executed = true;
        }
    }

    // Runs the sweep logic
    pub fn channel1_sweep_step(&mut self) {
        let freq = self.channel1_frequency;
        let shift = self.channel1_sweep_shift();
        let new_value = if self.channel1_sweep_increase() {
            let n = freq + (freq >> shift);
            if n > 0x7FF {
                self.unset_sound1();
                return;
            } else {
                n
            }
        } else {
            self.channel1_negate_executed = true;
            freq - (freq >> shift)
        };
        if shift != 0 {
            self.channel1_frequency = new_value;
            self.apu_mem[0xFF13 - APU_BASE] = (new_value & 0xFF) as u8;
            self.apu_mem[0xFF14 - APU_BASE] &= !0x7;
            self.apu_mem[0xFF14 - APU_BASE] |= ((new_value >> 8) & 0x7) as u8;

            self.channel1_sweep_step_overflow_logic();
        }
    }

    pub fn channel1_wave_pattern_duty(&self) -> f32 {
        match (self.apu_mem[0xFF11 - APU_BASE] >> 6) & 0x3 {
            0 => 0.125,
            1 => 0.25,
            2 => 0.5,
            3 => 0.75,
            _ => unreachable!(),
        }
    }

    pub fn channel1_sound_length(&self) -> u8 {
        self.apu_mem[0xFF11 - APU_BASE] & 0x3F
    }
    fn set_channel1_sound_length(&mut self, v: u8) {
        self.apu_mem[0xFF11 - APU_BASE] &= !0x3F;
        self.apu_mem[0xFF11 - APU_BASE] |= v & 0x3F;
    }

    pub fn channel1_inc_sound_length(&mut self) {
        if !self.channel1_sound_length_enabled() {
            return;
        }
        let mut val = self.apu_mem[0xFF11 - APU_BASE] & 0x3F;
        val += 1;
        if val >= 64 {
            self.unset_sound1();
        }
        self.apu_mem[0xFF11 - APU_BASE] &= !0x3F;
        self.apu_mem[0xFF11 - APU_BASE] |= val & 0x3F;
    }

    fn channel1_envelope_volume(&self) -> u8 {
        (self.apu_mem[0xFF12 - APU_BASE] >> 4) & 0xF
    }

    pub fn channel1_step_envelope(&mut self) {
        let val = self.channel1_envelope_volume;
        let new_val = if self.channel1_envelope_increasing {
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
        self.channel1_envelope_volume = new_val;
    }

    fn channel1_envelope_increasing(&self) -> bool {
        ((self.apu_mem[0xFF12 - APU_BASE] >> 3) & 0x1) == 1
    }

    pub fn channel1_envelope_sweep_pace(&self) -> u8 {
        self.apu_mem[0xFF12 - APU_BASE] & 0x7
    }

    fn channel1_frequency(&self) -> u16 {
        let lower = self.apu_mem[0xFF13 - APU_BASE];
        let higher = self.apu_mem[0xFF14 - APU_BASE] & 0x7;
        (higher as u16) << 8 | (lower as u16)
    }

    pub fn channel1_sound_length_enabled(&self) -> bool {
        ((self.apu_mem[0xFF14 - APU_BASE] >> 6) & 1) == 1
    }

    pub fn channel2_wave_pattern_duty(&self) -> f32 {
        match (self.apu_mem[0xFF16 - APU_BASE] >> 6) & 0x3 {
            0 => 0.125,
            1 => 0.25,
            2 => 0.5,
            3 => 0.75,
            _ => unreachable!(),
        }
    }

    pub fn channel2_sound_length(&self) -> u8 {
        self.apu_mem[0xFF16 - APU_BASE] & 0x3F
    }
    fn set_channel2_sound_length(&mut self, v: u8) {
        self.apu_mem[0xFF16 - APU_BASE] &= !0x3F;
        self.apu_mem[0xFF16 - APU_BASE] |= v & 0x3F;
    }

    pub fn channel2_inc_sound_length(&mut self) {
        if !self.channel2_sound_length_enabled() {
            return;
        }
        let mut val = self.apu_mem[0xFF16 - APU_BASE] & 0x3F;
        val += 1;
        if val >= 64 {
            self.unset_sound2();
        }
        self.apu_mem[0xFF16 - APU_BASE] &= !0x3F;
        self.apu_mem[0xFF16 - APU_BASE] |= val & 0x3F;
    }

    fn channel2_envelope_volume(&self) -> u8 {
        (self.apu_mem[0xFF17 - APU_BASE] >> 4) & 0xF
    }

    pub fn channel2_step_envelope(&mut self) {
        let val = self.channel2_envelope_volume;
        let new_val = if self.channel2_envelope_increasing {
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
        self.channel2_envelope_volume = new_val;
    }

    fn channel2_envelope_increasing(&self) -> bool {
        ((self.apu_mem[0xFF17 - APU_BASE] >> 3) & 0x1) == 1
    }

    pub fn channel2_envelope_sweep_pace(&self) -> u8 {
        self.apu_mem[0xFF17 - APU_BASE] & 0x7
    }

    pub fn channel2_frequency(&self) -> u16 {
        let lower = self.apu_mem[0xFF18 - APU_BASE];
        let higher = self.apu_mem[0xFF19 - APU_BASE] & 0x7;

        (higher as u16) << 8 | (lower as u16)
    }

    pub fn channel2_sound_length_enabled(&self) -> bool {
        ((self.apu_mem[0xFF19 - APU_BASE] >> 6) & 1) == 1
    }

    pub fn channel3_on(&self) -> bool {
        ((self.apu_mem[0xFF1A - APU_BASE] >> 7) & 1) == 1
    }

    pub fn channel3_sound_length(&self) -> u8 {
        self.apu_mem[0xFF1B - APU_BASE]
    }
    fn set_channel3_sound_length(&mut self, v: u8) {
        self.apu_mem[0xFF1B - APU_BASE] = v;
    }

    pub fn channel3_inc_sound_length(&mut self) {
        // REVIEW: do we care about DAC here?
        if !self.channel3_sound_length_enabled()
        /*|| self.apu_mem[0xFF1A - APU_BASE] >> 7 == 0*/
        {
            return;
        }
        let mut val = self.apu_mem[0xFF1B - APU_BASE];
        if val == 0xFF {
            val = 0;
            self.unset_sound3();
        } else {
            val += 1;
        }
        self.apu_mem[0xFF1B - APU_BASE] = val;
    }

    pub fn channel3_output_level(&self) -> f32 {
        match (self.apu_mem[0xFF1C - APU_BASE] >> 5) & 0x3 {
            0 => 0.0,
            1 => 1.0,
            2 => 0.5,
            3 => 0.25,
            _ => unreachable!(),
        }
    }

    pub fn channel3_shift_amount(&self) -> u8 {
        match (self.apu_mem[0xFF1C - APU_BASE] >> 5) & 0x3 {
            0 => 4,
            1 => 0,
            2 => 1,
            3 => 2,
            _ => unreachable!(),
        }
    }

    pub fn channel3_frequency(&self) -> u16 {
        let lower = self.apu_mem[0xFF1D - APU_BASE];
        let higher = self.apu_mem[0xFF1E - APU_BASE] & 0x7;

        (higher as u16) << 8 | (lower as u16)
    }

    pub fn channel3_sound_length_enabled(&self) -> bool {
        ((self.apu_mem[0xFF1E - APU_BASE] >> 6) & 1) == 1
    }

    pub fn channel3_wave_pattern_ram(&self) -> [u8; 32] {
        let mut ret = [0u8; 32];
        for i in 0..32 {
            ret[i] = (self.apu_mem[0xFF30 + (i / 2) - APU_BASE] >> (((i + 1) % 2) * 4)) & 0xF;
        }

        ret
    }

    pub fn channel4_sound_length(&self) -> u8 {
        self.apu_mem[0xFF20 - APU_BASE] & 0x3F
    }
    fn set_channel4_sound_length(&mut self, v: u8) {
        self.apu_mem[0xFF20 - APU_BASE] &= !0x3F;
        self.apu_mem[0xFF20 - APU_BASE] |= v & 0x3F;
    }

    pub fn channel4_inc_sound_length(&mut self) {
        if !self.channel4_sound_length_enabled() {
            return;
        }
        let mut val = self.apu_mem[0xFF20 - APU_BASE] & 0x3F;
        val += 1;
        if val >= 64 {
            self.unset_sound4();
        }
        self.apu_mem[0xFF20 - APU_BASE] &= !0x3F;
        self.apu_mem[0xFF20 - APU_BASE] |= val & 0x3F;
    }

    pub fn channel4_sound_length_enabled(&self) -> bool {
        ((self.apu_mem[0xFF23 - APU_BASE] >> 6) & 1) == 1
    }

    fn channel4_envelope_volume(&self) -> u8 {
        (self.apu_mem[0xFF21 - APU_BASE] >> 4) & 0xF
    }

    pub fn channel4_step_envelope(&mut self) {
        let val = self.channel4_envelope_volume;
        let new_val = if self.channel4_envelope_increasing {
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
        self.channel4_envelope_volume = new_val;
    }

    fn channel4_envelope_increasing(&self) -> bool {
        ((self.apu_mem[0xFF21 - APU_BASE] >> 3) & 0x1) == 1
    }

    pub fn channel4_envelope_sweep_pace(&self) -> u8 {
        self.apu_mem[0xFF21 - APU_BASE] & 0x7
    }

    pub fn channel4_clock_shift(&self) -> u8 {
        (self.apu_mem[0xFF22 - APU_BASE] >> 4) & 0xF
    }

    pub fn channel4_lfsr_width(&self) -> bool {
        ((self.apu_mem[0xFF22 - APU_BASE] >> 3) & 0x1) == 1
    }

    pub fn channel4_clock_divider(&self) -> f32 {
        match self.apu_mem[0xFF22 - APU_BASE] & 0x7 {
            0 => 0.5,
            n => n as f32,
        }
    }

    pub fn get_sound1(&self) -> bool {
        (self.apu_mem[0xFF26 - APU_BASE] & 1) == 1
    }
    pub fn get_sound2(&self) -> bool {
        ((self.apu_mem[0xFF26 - APU_BASE] >> 1) & 1) == 1
    }
    pub fn get_sound3(&self) -> bool {
        ((self.apu_mem[0xFF26 - APU_BASE] >> 2) & 1) == 1
    }
    pub fn get_sound4(&self) -> bool {
        ((self.apu_mem[0xFF26 - APU_BASE] >> 3) & 1) == 1
    }
    pub fn get_sound_all(&self) -> bool {
        ((self.apu_mem[0xFF26 - APU_BASE] >> 7) & 1) == 1
    }
    pub fn set_sound1(&mut self) {
        self.apu_mem[0xFF26 - APU_BASE] |= 1;
        self.channel1_frequency = self.channel1_frequency();
        self.channel1_sweep_pace = self.channel1_sweep_pace();
        self.channel1_envelope_pace = self.channel1_envelope_sweep_pace();
        self.channel1_sweep_counter = if self.channel1_sweep_pace != 0 {
            self.channel1_sweep_pace
        } else {
            8
        };
        self.channel1_envelope_counter = if self.channel1_envelope_pace != 0 {
            self.channel1_envelope_pace
        } else {
            8
        };
        self.channel1_envelope_increasing = self.channel1_envelope_increasing();
        self.channel1_envelope_volume = self.channel1_envelope_volume();
        self.channel1_negate_executed = false;
        self.channel1_sweep_enabled =
            self.channel1_sweep_shift() > 0 || self.channel1_sweep_pace > 0;

        if self.channel1_sweep_shift() > 0 {
            self.channel1_sweep_step_overflow_logic();
        }
    }
    pub fn set_sound2(&mut self) {
        self.apu_mem[0xFF26 - APU_BASE] |= 1 << 1;
        self.channel2_envelope_pace = self.channel2_envelope_sweep_pace();
        self.channel2_envelope_counter = if self.channel2_envelope_pace != 0 {
            self.channel2_envelope_pace
        } else {
            8
        };
        self.channel2_envelope_increasing = self.channel2_envelope_increasing();
        self.channel2_envelope_volume = self.channel2_envelope_volume();
    }
    pub fn set_sound3(&mut self) {
        self.apu_mem[0xFF26 - APU_BASE] |= 1 << 2;
    }
    pub fn set_sound4(&mut self) {
        self.apu_mem[0xFF26 - APU_BASE] |= 1 << 3;
        self.channel4_envelope_pace = self.channel4_envelope_sweep_pace();
        self.channel4_envelope_counter = if self.channel4_envelope_pace != 0 {
            self.channel4_envelope_pace
        } else {
            8
        };
        self.channel4_envelope_increasing = self.channel4_envelope_increasing();
        self.channel4_envelope_volume = self.channel4_envelope_volume();
    }
    pub fn set_sound_all(&mut self) {
        if !self.get_sound_all() {
            self.div_apu = 7;
            //self.div_apu = 0;
            //self.reset(false);
        }
        self.apu_mem[0xFF26 - APU_BASE] |= 1 << 7;
    }
    pub fn unset_sound1(&mut self) {
        self.apu_mem[0xFF26 - APU_BASE] &= !1;
        // on reset, clear value at
        //self.apu_mem[0xFF13 - APU_BASE] = 0;
        /*
        if self.channel1_sound_length() == 0 {
            self.set_channel1_sound_length(64);
        }
        */
    }
    pub fn unset_sound2(&mut self) {
        self.apu_mem[0xFF26 - APU_BASE] &= !(1 << 1);
        // on reset, clear value at
        //        self.apu_mem[0xFF13 - APU_BASE] = 0;
    }
    pub fn unset_sound3(&mut self) {
        self.apu_mem[0xFF26 - APU_BASE] &= !(1 << 2);
        // on reset, clear value at
        //self.apu_mem[0xFF13 - APU_BASE] = 0;
    }
    pub fn unset_sound4(&mut self) {
        self.apu_mem[0xFF26 - APU_BASE] &= !(1 << 3);
        // on reset, clear value at
        //self.apu_mem[0xFF13 - APU_BASE] = 0;
    }
    pub fn unset_sound_all(&mut self) {
        self.unset_sound1();
        self.unset_sound2();
        self.unset_sound3();
        self.unset_sound4();
        self.apu_mem[0xFF26 - APU_BASE] &= !(0x80);
        // zero all audio registers
        for i in 0xFF10..=0xFF2F {
            self.apu_mem[i - APU_BASE] = 0;
        }
    }
}

/*
fn update_channel_vars(&mut self) {
    self.channel1_sweep_pace = self.gameboy.channel1_sweep_pace();
    self.channel1_envelope_pace = self.gameboy.channel1_envelope_sweep_pace();
    self.channel2_envelope_pace = self.gameboy.channel2_envelope_sweep_pace();
    self.channel4_envelope_pace = self.gameboy.channel4_envelope_sweep_pace();
}
*/
