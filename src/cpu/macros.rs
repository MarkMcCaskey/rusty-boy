macro_rules! setter_unsetter_and_getter {
    ($name_setter:ident, $name_unsetter:ident, $name_getter:ident,
     $memory_location:expr) => {
        macro_rules! $name_setter {
            ($name:ident, $location:expr) => {
                //TODO: maybe add an option for setting them public?
                pub fn $name(&mut self) {
                    let orig_val = self.mem[$memory_location] as u8;

                    self.mem[$memory_location] = (orig_val | $location) as byte;
                }
            }
        }

        macro_rules! $name_unsetter {
            ($name:ident, $location:expr) => {
                fn $name(&mut self) {
                    let orig_val = self.mem[$memory_location] as u8;

                    self.mem[$memory_location] = (orig_val & (!$location)) as byte;
                }
            }
        }

        macro_rules! $name_getter {
            ($name:ident, $location:expr) => {
                pub fn $name(&self) -> bool{
                    ((self.mem[$memory_location] as u8) & $location)
                        == $location
                }
            }
        }
    }
}

macro_rules! make_getter {
    ($name_m:ident, $memory_location:expr) => {
        macro_rules! $ident {
            ($name:ident, $location:expr) => {
                fn $name(&self) -> bool {
                    (self.mem[$memory_location] & $location)
                        == $location
                }
            }
        }
    }
}


//NOTE: look into separate sound on/off storage outside of
// GB memory to prevent subtle "bug"/non-correct behavior

setter_unsetter_and_getter!(set_sound_on, unset_sound_on, get_sound_on, 0xFF26);
setter_unsetter_and_getter!(set_interrupt_bit, unset_interrupt_bit, get_interrupt, 0xFF0F);
setter_unsetter_and_getter!(set_interrupt_enabled, unset_interrupt_enabled, get_interrupt_enabled, 0xFFFF);
setter_unsetter_and_getter!(set_stat, unset_stat, get_stat, 0xFF41);


//macro for dispatching on opcodes where the LSB of the "y" set of
// octets determines which opcode to run
/*
(bit layout is xxyy yzzz) so LSB of y bits is MSB of first nibble
*/
macro_rules! even_odd_dispatch {
    ($num:expr, $cpu:ident, $func0:ident, $func1:ident,
     $f0dispfunc:ident, $f1dispfunc:ident, $f0pcincs:expr,
     $f1pcincs:expr) => {

        if $num % 2 == 0 {
            let adjusted_number:u8 = $num / 2;
            $cpu.$func0($f0dispfunc(adjusted_number));
            
            // TODO: Verify this executes it n-1 times
            for _ in 1..($f0pcincs) {
                $cpu.inc_pc();
            }
        } else {
            let adjusted_number:u8 = $num / 2;
            $cpu.$func1($f1dispfunc(adjusted_number));
            
            for _ in 1..($f1pcincs) {
                $cpu.inc_pc();
            }
        }
    }
}


/* Unfortunately, there's just no way to prevent this boiler plate in
Rust right now... The concat_idents! does not work for new identifiers
and interpolate_idents! seems to have problems and only supports nightly 
anyway.
TODO: fix Rust macro system to allow this or update interpolate_idents
 */
macro_rules! button {
    ($press_button:ident, $unpress_button:ident, $location:expr) => {
        pub fn $press_button(&mut self) {
            let old_val = self.input_state;
            self.input_state = (old_val & (!$location)) as byte;
            if self.state == CpuState::Stop {
                self.state = CpuState::Normal;
            }
            self.set_input_interrupt_bit();
        }
        
        pub fn $unpress_button(&mut self) {
            let old_val = self.input_state;
            self.input_state = (old_val | $location) as byte;
        }
    }
}


