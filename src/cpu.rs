const zl: i8 = 0x80;
const nl: i8 = 0x40;
const hl: i8 = 0x20;
const cl: i8 = 0x10;

struct Cpu {
    a:   i8,
    b:   i8,
    c:   i8,
    d:   i8,
    e:   i8,
    f:   i8, //NOTE: bit 7: zero flag; bit 6: subtract flag; bit 5: half carry; bit 4: carry flag
    h:   i8,
    l:   i8,
    sp:  u16,
    pc:  u16,
    mem: [i8; 0xFFFF + 1] //TODO: Verify this
}

/*
enum ControllerInput {

}
*/

#[derive(Clone,Copy)]
enum CpuRegister {
    A, B, C, D, E, H, L, HL, Num(i8),
}

#[derive(Clone,Copy)]
enum CpuRegister16 {
    BC, DE, HL, SP, Num(i16),
}

impl Cpu {
    fn new() -> Cpu {
        Cpu {
            a:   0,
            b:   0,
            c:   0,
            d:   0,
            e:   0,
            f:   0,
            h:   0,
            l:   0,
            sp:  0xFFFE,
            pc:  0x100,
            mem: [0; 0xFFFF + 1]
        }
    }

    fn hl(&self) -> u16 {
        (self.h as u16) << 8 | (self.l as u16)
    }

    fn set_hl(&mut self, hlv:u16) {
        self.h = ((hlv & 0xFF00) >> 8) as i8;
        self.l = (hlv & 0xFF)          as i8;
    }

    fn bc(&self) -> u16 {
        (self.b as u16) << 8 | (self.c as u16)
    }

    fn de(&self) -> u16 {
        (self.d as u16) << 8 | (self.e as u16)
    }

    fn set_bc(&mut self, bcv: u16) {
        self.b = ((bcv & 0xFF00) >> 8) as i8;
        self.c = (bcv & 0xFF)          as i8;
    }

    fn set_de(&mut self, dev: u16) {
        self.d = ((dev & 0xFF00) >> 8) as i8;
        self.e = (dev & 0xFF)          as i8;
    }


    fn set_mem(&mut self, address: usize, value: i8) {
        match address {
            ad @ 0xE000 ... 0xFE00 | ad @ 0xC000 ... 0xDE00
                => {
                    self.mem[ad]         = value;
                    self.mem[ad ^ (0xE000 - 0xC000)] = value;
                },
            n => self.mem[n] = value,
        }
    }

    fn access_register(&self, reg: CpuRegister) -> Option<i8> {
        match reg {
            CpuRegister::A  => Some(self.a),
            CpuRegister::B  => Some(self.b),
            CpuRegister::C  => Some(self.c),
            CpuRegister::D  => Some(self.d),
            CpuRegister::E  => Some(self.e),
            CpuRegister::H  => Some(self.h),
            CpuRegister::L  => Some(self.l),
            CpuRegister::HL => Some(self.mem[self.hl() as usize]),
            _               => None,
        } 
    }

    fn set_register(&mut self, reg: CpuRegister, val:i8) {
        match reg {
            CpuRegister::A  => self.a = val,
            CpuRegister::B  => self.b = val,
            CpuRegister::C  => self.c = val,
            CpuRegister::D  => self.d = val,
            CpuRegister::E  => self.e = val,
            CpuRegister::H  => self.h = val,
            CpuRegister::L  => self.l = val,
            CpuRegister::HL => {
                let hlv = self.hl();
                self.set_mem(hlv as usize, val);
            },
            _               => panic!("Cannot set non-8bit values"),
        } 
    }

    //TODO: rename this awfully named function
    fn alu_dispatch<F>(&self, reg: CpuRegister, f: F) -> i16 where
        F: FnOnce(i8, i8) -> i16 {
        f(self.a,
          match reg {
              CpuRegister::A      => self.a,
              CpuRegister::B      => self.b,
              CpuRegister::C      => self.c,
              CpuRegister::D      => self.d,
              CpuRegister::E      => self.e,
              CpuRegister::H      => self.h,
              CpuRegister::L      => self.l,
              CpuRegister::HL     => self.mem[self.hl() as usize],
              CpuRegister::Num(i) => i, // :)
          })
    }

    //TODO: rename this awfully named function
    fn alu_dispatch16<F>(&self, reg: CpuRegister16, f: F) -> i32 where
        //TODO: Maybe an issue here?
        F: FnOnce(i32, i32) -> i32 {
        f(self.hl() as i32,
          match reg {
              CpuRegister16::BC     => self.bc() as i32,
              CpuRegister16::DE     => self.de() as i32,
              CpuRegister16::HL     => self.hl() as i32,
              CpuRegister16::SP     => self.sp   as i32,
              CpuRegister16::Num(i) => i as i32, 
          }) as i32
    }

        
    //TODO: set flags based on results
    fn add(&mut self, reg: CpuRegister) {
        let mut flags = 0;

        let new_a = self.alu_dispatch(reg, |a: i8, b: i8| (a as i16) + (b as i16));
        self.a = new_a as i8;
        
        flags |= if (new_a as i8) == 0 { zl } else { 0 };
        flags |= if new_a > 0xFF       { hl } else { 0 };
        //implement half carry bitflag

        self.f = flags;
    }

    fn adc(&mut self, reg: CpuRegister) {
        let cf: i8 = self.f & hl >> 5;
        self.add(reg);

        let new_a: i16 = (cf + self.a) as i16;

        self.f |= if new_a > (u8::max_value() as i16) { hl } else { 0 };
    }

    fn sub(&mut self, reg: CpuRegister) {
        let mut flags = nl;

        let new_a: i16 = self.alu_dispatch(reg, |a: i8, b: i8| (a as i16) - (b as i16));
        self.a = new_a as i8;

        flags |= if (new_a as i8) == 0 { zl } else { 0 };
        //TODO: review this after sleeping
 //       flags |= if new_a > 0xFF       { hl } else { 0 };
        //implement half carry bitflag

        self.f = flags;

    }

    fn sbc(&mut self, reg: CpuRegister) {
        let cf: i8 = self.f & hl >> 5;
        self.add(reg);

        //NOTE: find out whether this should be self.a - cf
        let new_a: i16 = (cf + self.a) as i16;

        self.f |= if new_a > (i8::max_value() as i16) { hl } else { 0 };
    }

    fn and(&mut self, reg: CpuRegister) {
        let new_a: i16 = self.alu_dispatch(reg, |a: i8, b: i8| (a as i16) & (b as i16));

        self.f = hl | if new_a == 0 { zl } else { 0 };
    }

    fn or(&mut self, reg: CpuRegister) {
        let new_a: i16 = self.alu_dispatch(reg, |a: i8, b: i8| (a as i16) | (b as i16));
        self.a = new_a as i8;

        self.f = if new_a == 0 { zl } else { 0 };
    }

    fn xor(&mut self, reg: CpuRegister) {
        let new_a: i16 = self.alu_dispatch(reg, |a: i8, b: i8| (a as i16) ^ (b as i16));
        self.a = new_a as i8;

        self.f = if new_a == 0 { zl } else { 0 };
    }

    fn cp(&mut self, reg: CpuRegister) {
    }

    fn inc(&mut self, reg: CpuRegister) {
        let old_c = self.f & hl;
        let old_3bit = self.a & 0x8; //TODO: rename this/redo this
        //old_3bit is used to detect overflow of 3rd bit

        let new_a: i16 = self.alu_dispatch(reg, |_, b: i8| (b + 1) as i16);
        self.a = new_a as i8;

        self.f = if new_a == 0 { zl } else { 0 };
        self.f |= old_c;
        self.f |= if old_3bit == 0x8 && (self.a & 0x8 == 0) { nl } else { 0 };
    }

    fn dec(&mut self, reg: CpuRegister) {
        let old_c = self.f & hl;
        let old_4bit = self.a & 0x10; //TODO: rename this/redo this
        //old_4bit is used to detect overflow of 4th bit

        let new_a: i16 = self.alu_dispatch(reg, |_, b: i8| (b - 1) as i16);
        self.a = new_a as i8;

        self.f = if new_a == 0 { zl } else { 0 };
        self.f |= old_c;
       //self.f |= if old_4bit == 0x8 && (self.a & 0x8 == 0) { nl } else { 0 };
        //TODO: borrowing of 4th bit flag
    }

    fn add_hl(&mut self, reg: CpuRegister16) {
        let old_z = self.f & zl;
        //TODO: review after sleeping if this actually makes sense for checking middle-ish overflow (carefully consider negative numbmers)
        let old_11bit = self.a & 0x800;

        let new_hl = self.alu_dispatch16(reg, |a:i32, b:i32| (a as i32) + (b as i32));

        self.set_hl(new_hl as u16);

        self.f = old_z;
        self.f |= if old_11bit == 1 && (self.a & 0x800 == 0) { hl } else { 0 };
        self.f |= if new_hl > (u16::max_value() as i32) { cl } else { 0 };
    }

    //Consider adding further restrictions to this type; argument must be an immediate value
    fn add_sp(&mut self, reg: CpuRegister16) {
        if let CpuRegister16::Num(i) = reg {
            self.sp = ((self.sp as i16) + i )as u16;
            //TODO: figure out what happens to the bottom two flags
            self.f &= 0x3F;
        }
        else {
            panic!("In add_sp, invalid argument.  It must be an immediate value");
        }

    }

    fn inc16(&mut self, reg: CpuRegister16) {
        match reg {
            CpuRegister16::BC => { let old_v = self.bc()+1; self.set_bc(old_v); },
            CpuRegister16::DE => { let old_v = self.de()+1; self.set_de(old_v); },
            CpuRegister16::HL => { let old_v = self.hl()+1; self.set_hl(old_v); },
            CpuRegister16::SP => self.sp += 1,
            _ => panic!("inc16 cannot take numeric values as arguments"),
        } 
    }

    fn dec16(&mut self, reg: CpuRegister16) {
        match reg {
            CpuRegister16::BC => { let old_v = self.bc()-1; self.set_bc(old_v); },
            CpuRegister16::DE => { let old_v = self.de()-1; self.set_de(old_v); },
            CpuRegister16::HL => { let old_v = self.hl()-1; self.set_hl(old_v); },
            CpuRegister16::SP => self.sp -= 1,
            _ => panic!("dec16 cannot take numeric values as arguments"),
        } 
    }

    fn swap(&mut self, reg: CpuRegister) {
        //Potentially can bitmask hl which is 16bit value
        let val = self.access_register(reg).expect("couldn't access register value");
        let top = val & 0xF0;
        let bot = val & 0x0F;
        self.set_register(reg, (top >> 4) | (bot << 4));
         
        self.f = if val == 0 { zl } else { 0 };
    }

    /*
    DAA
    CPL
    CCF 
    SCF
    NOP
    HALT
    STOP
    DI
    EI
    RLCA
    RLA
    RRCA
    RRA
    RLC
    RL
    RRC
    RR
    SLA
    SRA
    SRL
    BIT
    SET
    RES
    JP
    JP
    JP
    JP
    JR
    CALL
    CALL
    RST
    RET
    RET
    RETI
     */


//    fn controller_input(&mut self, value)
}


    /*
    LD:
    B: 06 = 00000110
    C: 0E = 00001110
    D: 16 = 00010110
    E: 1E = 00011110
    H: 26 = 00100110
    L: 2E = 00101110
*/

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn add() {
        let mut cpu = Cpu::new();

        cpu.a = 5;
        cpu.add(CpuRegister::Num(5));

        assert_eq!(cpu.a, 10);
        assert_eq!(cpu.f & zl, 0);

        cpu.b = -10;
        cpu.add(CpuRegister::B);

        assert_eq!(cpu.a, 0);
        assert_eq!(cpu.f & zl, zl);
    }

    #[test]
    fn sub() {
        let mut cpu = Cpu::new();

        cpu.a = 5;
        cpu.sub(CpuRegister::Num(5));

        assert_eq!(cpu.a, 0);
        assert_eq!(cpu.f & zl, zl);

        cpu.b = -10;
        cpu.sub(CpuRegister::B);

        assert_eq!(cpu.a, 10);
        assert_eq!(cpu.f & zl, 0);
    }
}
