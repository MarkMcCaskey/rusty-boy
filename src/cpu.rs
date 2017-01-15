const zl: i8 = 0x80;
const nl: i8 = 0x40;
const hl: i8 = 0x20;
const cl: i8 = 0x10;

pub struct Cpu {
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

#[derive(Clone,Copy)]
enum Cc {
    NZ, Z, NC, C,
}

impl Cpu {
    pub fn new() -> Cpu {
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

    fn set_register16(&mut self, reg: CpuRegister16, val: u16) {
        match reg {
            CpuRegister16::BC => self.set_bc(val),
            CpuRegister16::DE => self.set_de(val),
            CpuRegister16::HL => self.set_hl(val),
            CpuRegister16::SP => self.sp = val,
            _  => panic!("Invalid 16bit register!"),
        }
    }

    fn access_register16(&mut self, reg: CpuRegister16) -> u16 {
        match reg {
            CpuRegister16::BC     => self.bc(),
            CpuRegister16::DE     => self.de(),
            CpuRegister16::HL     => self.hl(),
            CpuRegister16::SP     => self.sp, 
            CpuRegister16::Num(i) => i as u16,
        }
    }

    fn set_flags(&mut self, z: bool, n: bool, h: bool, c: bool) {
        let zn = (z as i8) << 7;
        let nn = (n as i8) << 6;
        let hn = (h as i8) << 5;
        let cn = (c as i8) << 4;

        self.f = zn | nn | hn | cn;
        println!("Hello from set_flags!  Value is: {}.  Booleans: {}, {}, {}, {}", self.f, z, n, h, c);
    }


    fn set_mem(&mut self, address: usize, value: i8) {
        match address {
            ad @ 0xE000 ... 0xFE00 | ad @ 0xC000 ... 0xDE00
                => {
                    self.mem[ad]                     = value;
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


    fn ldnnn(&mut self, nn: CpuRegister, n: u8) {
        self.set_register(nn, n as i8);
    }

    fn ldr1r2(&mut self, r1: CpuRegister, r2:CpuRegister) {
        let val = self.access_register(r2).expect("Invalid register");
        self.set_register(r1, val);
    }

    fn ldan(&mut self, n: CpuRegister) {
        let val = self.access_register(n).expect("Invalid register");
        self.set_register(CpuRegister::A, val);
    }

    fn ldan16(&mut self, n: CpuRegister16) {
        let addr = self.access_register16(n);
        let val = self.mem[addr as usize];

        self.set_register(CpuRegister::A, val);
    }

    fn ldna(&mut self, n: CpuRegister) {
        let val = self.access_register(CpuRegister::A).expect("Invalid register");
        self.set_register(n, val);
    }

    fn ldna16(&mut self, n: CpuRegister16) {
        let val = self.access_register(CpuRegister::A).expect("Invalid register");
        let addr = self.access_register16(n);

        self.set_mem(addr as usize, val);
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
              CpuRegister::Num(i) => i, 
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

    fn reg_or_const(&mut self, reg: CpuRegister) -> i16 {
        if let Some(r) = self.access_register(reg) {
            r as i16
        } else { //constant value
            if let CpuRegister::Num(v) = reg {
                v as i16
            } else { unreachable!("The impossible happened!") }
        }
    }

    
    fn add(&mut self, reg: CpuRegister) {
        let old_a = self.a as i16;
        let old_b = self.reg_or_const(reg);

        let new_a = self.alu_dispatch(reg, |a: i8, b: i8| (a as i16) + (b as i16));

        self.a = new_a as i8;

        self.set_flags((new_a as i8) == 0,
                       false,
                       ((old_a % 16) + (old_b % 16)) > 15,
                       (old_a + old_b) > i8::max_value() as i16);
    }

    fn adc(&mut self, reg: CpuRegister) {
        let old_a = self.a as i16;
        let old_b = self.reg_or_const(reg);
        let cf: i8 = self.f & hl >> 5;
        self.add(reg);

        let new_a: i16 = (cf + self.a) as i16;


        self.f |= if (old_a + old_b) > (i8::max_value() as i16) { hl } else { 0 };
        self.f |= if ((old_a % 16) + (old_b % 16)) > 15 { cl } else { 0 };
    }

    fn sub(&mut self, reg: CpuRegister) {
        let old_a = self.a as i16;
        let old_b = self.reg_or_const(reg);
        let new_a: i16 = self.alu_dispatch(reg, |a: i8, b: i8| (a as i16) - (b as i16));

        self.a = new_a as i8;
        self.set_flags((new_a as i8) == 0,
                       true,
                       (old_a & 0xF) >= (old_b & 0xF),
                       old_b <= old_a as i16);
    }

    fn sbc(&mut self, reg: CpuRegister) {
        let old_a = self.a as i16;
        let old_b = self.reg_or_const(reg);
        let old_c = (old_a - old_b) as i8;
        let cf: i8 = self.f & hl >> 5;
        self.sub(reg);

        //NOTE: find out whether this should be self.a - cf
        let new_a: i16 = (self.a - cf) as i16;
        self.a = new_a as i8;
        self.set_flags((new_a as i8) == 0,
                       true,
                       (old_c & 0xF) >= cf,
                       cf <= old_c);
    }

    fn and(&mut self, reg: CpuRegister) {
        let new_a: i16 = self.alu_dispatch(reg, |a: i8, b: i8| (a as i16) & (b as i16));

        self.a = new_a as i8;
        self.set_flags(new_a == 0, false, true, false);
    }

    fn or(&mut self, reg: CpuRegister) {
        let new_a: i16 = self.alu_dispatch(reg, |a: i8, b: i8| ((a as u16) | (b as u16)) as i16);

        self.a = new_a as i8;
        self.set_flags(new_a == 0, false, false, false);
    }

    fn xor(&mut self, reg: CpuRegister) {
        let new_a: i16 = self.alu_dispatch(reg, |a: i8, b: i8| (a as i16) ^ (b as i16));

        self.a = new_a as i8;
        self.set_flags(new_a == 0, false, false, false);
    }

    fn cp(&mut self, reg: CpuRegister) {
        let old_a = self.a;
        self.sub(reg);
        self.a = old_a;
        self.f |= nl;
    }

    fn inc(&mut self, reg: CpuRegister) {
        let old_c = (self.f & hl) == hl;
        let old_3bit = self.a & 0x8; //TODO: rename this/redo this
        //old_3bit is used to detect overflow of 3rd bit

        let new_a: i16 = self.alu_dispatch(reg, |_, b: i8| (b + 1) as i16);
        self.a = new_a as i8;
        self.set_flags(new_a == 0,
                       false,
                       old_c,
                       old_3bit == 0x8 && (new_a & 0x8 == 0));
    }

    fn dec(&mut self, reg: CpuRegister) {
        let old_c = (self.f & hl) == hl;
        let old_4bit = self.a & 0x10; //TODO: rename this/redo this
        //old_4bit is used to detect overflow of 4th bit

        let new_a: i16 = self.alu_dispatch(reg, |_, b: i8| (b - 1) as i16);
        self.a = new_a as i8;

        self.f = if new_a == 0 { zl } else { 0 };
        //self.f |= old_c;
        //self.f |= if old_4bit == 0x8 && (self.a & 0x8 == 0) { nl } else { 0 };
        //TODO: borrowing of 4th bit flag
        /*        self.set_flags(new_a == 0,
        false,
        old_c
         */
    }

    fn add_hl(&mut self, reg: CpuRegister16) {
        let old_z = (self.f & zl) == zl;
        //TODO: review after sleeping if this actually makes sense for checking middle-ish overflow (carefully consider negative numbmers)
        let old_11bit = self.a & 0x800;

        let new_hl = self.alu_dispatch16(reg, |a:i32, b:i32| (a as i32) + (b as i32));

        self.set_hl(new_hl as u16);
        self.set_flags(old_z,
                       false,
                       old_11bit == 1 && (new_hl & 0x800 == 0),
                       new_hl > (u16::max_value() as i32));
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


    fn daa(&mut self) {
        let reduced_a = self.a as u16;

        let lowest_bits = reduced_a & 0xF;

        let lowest_digit = if lowest_bits > 9 {(lowest_bits + 6) & 0xF} else {lowest_bits};
        let highest_bits = ((reduced_a & 0xF0) + (if lowest_digit == lowest_bits {0} else {0x10})) & 0xF0;
        let highest_digit = if highest_bits > 0x90 {(highest_bits + 0x60) & 0xF0} else {highest_bits & 0xF0};

        self.a = (highest_digit | lowest_digit) as i8;
        let old_nflag = (self.f & nl) == nl;
        self.set_flags((highest_digit | lowest_digit) == 0,
                       old_nflag,
                       false,
                       0x99 < reduced_a); //NOTE: weird documentation, unclear value
    }

    fn cpl(&mut self) {
        let new_val = !self.a;
        let old_flags = self.f & (zl | cl);
        self.f = old_flags | nl | hl;
        self.a = new_val;
    }

    fn ccf(&mut self) {
        let old_flags = self.f & (zl | cl);
        self.f = old_flags ^ cl;
    }

    fn scf(&mut self) {
        let old_flags = self.f & zl;
        self.f = old_flags | cl;
    }

    fn nop(&self) {
        ()
    }

    //TODO:
    fn halt(&mut self) {
        
    }

    //TODO:
    fn stop(&mut self) {

    }

    //TODO:
    fn di(&mut self) {
    }

    //TODO:
    fn ei(&mut self) {
    }

    fn rlca(&mut self) {
        let old_bit7 = (self.a >> 7) & 1;

        let new_a = (self.a << 1) | old_bit7;
        self.a = new_a;

        self.set_flags(new_a == 0,
                       false,
                       false,
                       old_bit7 == 1);
    }

    fn rla(&mut self) {
        let old_bit7 = (self.a >> 7) & 1;
        let old_flags = (self.f & cl) >> 4;
        

        let new_a = (self.a << 1) | old_flags;
        self.a = new_a;

        self.set_flags(new_a == 0,
                       false,
                       false,
                       old_bit7 == 1);       
    }

    fn rrca(&mut self) {
        let old_bit0 = self.a & 1;

        let new_a = (self.a >> 1) | (old_bit0 << 7);
        self.a = new_a;

        self.set_flags(new_a == 0,
                       false,
                       false,
                       old_bit0 == 1);
    }

    fn rra(&mut self) {
        let old_bit0 = self.a & 1;
        let old_flags = (self.f & cl) >> 4;

        let new_a = (self.a >> 1) | (old_flags << 7);
        self.a = new_a;

        self.set_flags(new_a == 0,
                       false,
                       false,
                       old_bit0 == 1);
    }

    fn rlc(&mut self, reg: CpuRegister) {
        let reg_val = self.access_register(reg).expect("invalid register");
        let old_bit7 = (reg_val >> 7) & 1;

        let new_reg = (reg_val << 1) | old_bit7;
        self.set_register(reg, new_reg);

        self.set_flags(new_reg == 0,
                       false,
                       false,
                       old_bit7 == 1);
    }

    fn rl(&mut self, reg: CpuRegister) {
        let reg_val = self.access_register(reg).expect("invalid register");
        let old_bit7 = (reg_val >> 7) & 1;
        let old_flags = (self.f & cl) >> 4;

        let new_reg = (reg_val << 1) | old_flags;
        self.set_register(reg, new_reg);

        self.set_flags(new_reg == 0,
                       false,
                       false,
                       old_bit7 == 1);
    }

    fn rrc(&mut self, reg: CpuRegister) {
        let reg_val = self.access_register(reg).expect("invalid register");
        let old_bit0 = reg_val & 1;

        let new_val = (reg_val >> 1) | (old_bit0 << 7);
        self.set_register(reg, new_val);

        self.set_flags(new_val == 0,
                       false,
                       false,
                       old_bit0 == 1);
    }

    fn rr(&mut self, reg: CpuRegister) {
        let reg_val = self.access_register(reg).expect("invalid register");
        let old_bit0 = reg_val & 1;
        let old_flags = (self.f & cl) >> 4;

        let new_val = (reg_val >> 1) | old_flags;
        self.set_register(reg, new_val);

        self.set_flags(new_val == 0,
                       false,
                       false,
                       old_bit0 == 1);
    }

    fn sla(&mut self, reg: CpuRegister) {
        let reg_val = self.access_register(reg).expect("invalid register");
        let old_bit7 = reg_val >> 7;
        self.set_register(reg, reg_val << 1);

        self.set_flags((reg_val << 1) == 0,
                       false,
                       false,
                       old_bit7 == 1);
    }

    fn sra(&mut self, reg: CpuRegister) {
        let reg_val = self.access_register(reg).expect("invalid register");
        let old_bit0 = reg_val & 1;
        self.set_register(reg, reg_val >> 1);

        self.set_flags((reg_val >> 1) == 0,
                       false,
                       false,
                       old_bit0 == 1);
    }

    fn srl(&mut self, reg: CpuRegister) {
        let reg_val = self.access_register(reg).expect("invalid register");
        let old_bit0 = reg_val & 1;

        self.set_register(reg, reg_val >> 1);

        self.set_flags((reg_val >> 1) == 0,
                       false,
                       false,
                       old_bit0 == 1);
    }

    fn bit(&mut self, b: u8, reg: CpuRegister) {
        let reg_val = self.access_register(reg).expect("invalid register");
        let old_flags = (self.f & cl) >> 4;
        
        self.set_flags((reg_val >> b) & 1 == 1,
                       false,
                       true,
                       old_flags & 1 == 1);
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
        self.pc = nn; //NOTE: Verify this byte order
    }

    fn jpccnn(&mut self, cc: Cc, nn: u16) {
        let will_jump = 1 ==
            match cc {
                Cc::NZ => !((self.f >> 7) & 1),
                Cc::Z  => (self.f >> 7) & 1,
                Cc::NC => !((self.f >> 4) & 1),
                Cc::C  => (self.f >> 4) & 1,
            };

        if will_jump {
            self.pc = nn;
        } else {
            ()
        }
    }

    //TODO: Double check (HL) HL thing
    fn jphl(&mut self) {
        let addr = self.hl();

        self.pc = addr;
    }

    fn jrn(&mut self, n: i8) {
        let old_pc = self.pc;

        self.pc = ((old_pc as i32) + (n as i32)) as u16;
    }

    fn jrccn(&mut self, cc: Cc, n: i8) {
        let will_jump = 1 ==
            match cc {
                Cc::NZ => !((self.f >> 7) & 1),
                Cc::Z  => (self.f >> 7) & 1,
                Cc::NC => !((self.f >> 4) & 1),
                Cc::C  => (self.f >> 4) & 1,
            };

        let old_pc = self.pc;
        if will_jump {
            self.pc = ((old_pc as i32) + (n as i32)) as u16;
        } else {
            ()
        }
    }

    //TODO: Verify if SP should be incremented first
    fn callnn(&mut self, nn: u16) {
        self.push_onto_stack(nn);
        self.pc = nn;
    }

    fn push_onto_stack(&mut self, nn: u16) {
        let first_half = (nn >> 8) as i8;
        let second_half = (nn & 0xFF) as i8;
        let old_sp = self.sp;

        self.set_mem((old_sp-1) as usize, first_half);
        self.set_mem((old_sp-2) as usize, second_half);

        self.sp -= 2;
    }

    fn callccnn(&mut self, cc: Cc, nn: u16) {
        let will_jump = 1 ==
            match cc {
                Cc::NZ => !((self.f >> 7) & 1),
                Cc::Z  => (self.f >> 7) & 1,
                Cc::NC => !((self.f >> 4) & 1),
                Cc::C  => (self.f >> 4) & 1,
            };

        if will_jump {
            self.callnn(nn);
        } else {
            ()
        }
    }

    fn rst(&mut self, n: u8) {
        let old_pc = self.pc;
        
        self.push_onto_stack(old_pc);

        self.pc = n as u16;
    }

    fn pop_from_stack(&mut self) -> u16 {
        let old_sp = self.sp;
        let val1 = self.mem[old_sp as usize];
        let val2 = self.mem[(old_sp+1) as usize];

        ((val2 as u16) << 8) | (val1 as u16)
    }

    fn ret(&mut self) {
        let new_addr = self.pop_from_stack();

        self.pc = new_addr;
    }

    fn retcc(&mut self, cc: Cc) {
        let will_jump = 1 ==
            match cc {
                Cc::NZ => !((self.f >> 7) & 1),
                Cc::Z  => (self.f >> 7) & 1,
                Cc::NC => !((self.f >> 4) & 1),
                Cc::C  => (self.f >> 4) & 1,
            };

        if will_jump {
            self.ret();
        } else {
            ()
        }
    }

    fn reti(&mut self) {
        let new_addr = self.pop_from_stack();

        self.pc = new_addr;
        self.ei();
    }

    /*
    HALT
    STOP
    DI
    EI
     */


    fn read_instruction(&self) -> i8 {
        self.mem[self.pc as usize]
    }

    fn inc_pc(&mut self) {
        self.pc += 1;
    }
    /*
    Handles running opcodes
    Opcodes are prefixed or unprefixed 
    1. [prefix byte,] opcode [,displacement byte] [,immediate data]
    2. prefix byte, prefix byte, displacement byte, opcode
    
    ASSUMPTION: Gameboy only uses the CB prefix codes of the Z80
     */
    pub fn dispatch_opcode(&mut self) {
        /*
        let first_byte = self.read_instruction();
        let x = (first_byte >> 6) & 0x3;
        let y = (first_byte >> 3) & 0x7;
        let z = first_byte        & 0x7;
        let p = (y >> 1)          & 0x3;
        let q = y                 & 0x1;

        let uf = "The impossible happened!";

        if first_byte == 0xCB { //prefixed instruction

    } else { //unprefixed instruction
        match x {
        0 =>
        match z {
        0 =>
        match y {
        0        => self.nop(),
        1        => /*AF*/ (),
        2        => /*DJNZ*/(),
        3        => (),//self.jr(),
        v @ 4...7 => /*jr cc*/(),
        _        => unreachable!(uf),
    },

        1 =>
        match q {
        0 => (),//ld
        1 => (),//add hl
        _ => unreachable!(uf),
    },

        2 =>
        match q {
        0 => (),
        1 => (),
        _ => unreachable!(uf),
    },

        _ => unreachable!(uf),
    },
        _ => panic!("The impossible happened!"),
    }
    }

        self.inc_pc();
         */
    }

    pub fn load_rom(&mut self, file_path: &str) {
        use std::fs::File;
        use std::io::Read;
        

        let mut rom = File::open(file_path).expect("Could not open rom file");
        let mut rom_buffer: [u8; 0x8000] = [0u8; 0x8000];

        rom.read(&mut rom_buffer).unwrap();

        for i in 0..0x8000 {
            self.set_mem(i, rom_buffer[i] as i8);
        }

    }


    pub fn play(&mut self) {
        loop {
            self.dispatch_opcode();
        }
    }

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
    use cpu;

    macro_rules! test_op {
        ($func:ident, $method:ident, $input:expr, $output_reg:ident,
         $expected_output:expr, $flag_find_value:expr,
         $flag_expected_value:expr, $pre_exec:expr) => {

            #[test]
            fn $func() {
                let mut cpu = Cpu::new();

                let (in_set, in_arg) = $input;

                //skip register preset if constant value
                if let CpuRegister::Num(_) = in_arg {
                    ()
                } else {
                    cpu.set_register(in_arg, $pre_exec);
                }

                cpu.$output_reg = in_set;
                cpu.$method(in_arg);

                println!("{:?} `op` {:?} = {:?}", in_set, $pre_exec, cpu.$output_reg);
                
                assert_eq!(cpu.$output_reg, $expected_output);
                assert_eq!($flag_find_value(cpu.f), $flag_expected_value);
            }
        }
    }
     
    test_op!(add_const, add, (5, CpuRegister::Num(5)), a, 5 + 5, |flags| flags & zl, 0,5);
    test_op!(add_reg, add, (5, CpuRegister::B), a, 5 + -5, |flags| flags, zl, -5);
    test_op!(add_mem_half_carry, add, (1, CpuRegister::HL), a, 15 + 1, |flags| flags, hl, 15);
    test_op!(add_mem_half_carry1, add, (15, CpuRegister::HL), a, 15 + 1, |flags| flags, hl, 1);

    test_op!(adc_test, adc, (10, CpuRegister::E), a, 10 + 1, |flags| flags, 0, 1);
//    test_op!(adc_carry, adc, (10, CpuRegister::F), a, 10 + 1, |flags| flags, 0, cl);

        

    test_op!(sub_const, sub, (5, CpuRegister::Num(5)), a, 5 - 5, |flags| flags, zl | nl | hl | cl, 0);
    test_op!(sub_reg, sub, (10, CpuRegister::B), a, 20, |flags| flags, nl | hl | cl, -10);
    test_op!(sub_reg2, sub, (10, CpuRegister::C), a, 0, |flags| flags, zl | nl | hl | cl, 10);
    test_op!(sub_reg_borrow, sub, (0, CpuRegister::D), a, -10, |flags| flags, nl, 10);
    //TODO: write half carry only test:
//    test_op!(sub_reg_borrow_half, sub, (130, CpuRegister::D), a, 40, |flags| flags, nl | hl, 32);

    test_op!(and_test, and, (0xF, CpuRegister::B), a, 0xF, |flags| flags, hl, 0xF);
    test_op!(and_test1, and, (0xF, CpuRegister::C), a, 0x1, |flags| flags, hl, 0x1);
    test_op!(and_test2, and, (0xF, CpuRegister::E), a, 0, |flags| flags, zl | hl, 0x70);

    test_op!(or_test, or, (0xF, CpuRegister::C), a, 0xF, |flags| flags, 0, 0xF);
    test_op!(or_test1, or, (0xF, CpuRegister::D), a, 0xF, |flags| flags, 0, 0);
    test_op!(or_test2, or, (0xF, CpuRegister::B), a, 0xFF, |flags| flags, 0, 0xF0);
    test_op!(or_test3, or, (0, CpuRegister::D), a, 0, |flags| flags, zl, 0);

    test_op!(xor_test, xor, (0xF, CpuRegister::C), a, 0, |flags| flags, zl, 0xF);
    test_op!(xor_test1, xor, (0xF, CpuRegister::D), a, 0xF, |flags| flags, 0, 0);
    test_op!(xor_test2, xor, (0xF, CpuRegister::B), a, 0xFF, |flags| flags, 0, 0xF0);
    test_op!(xor_test3, xor, (0, CpuRegister::D), a, 0, |flags| flags, zl, 0);

    
    test_op!(cp_test, cp, (0xF, CpuRegister::C), a, 0xF, |flags| flags, zl | nl  | hl | cl, 0xF);
    test_op!(cp_test1, cp, (0xF, CpuRegister::D), a, 0xF, |flags| flags, nl | hl | cl, 0);
    // TODO: verify hl and cl flags here make sense:
    test_op!(cp_test2, cp, (0xF, CpuRegister::B), a, 0xF, |flags| flags, nl | hl | cl , 0xF0);
    test_op!(cp_test3, cp, (0, CpuRegister::D), a, 0, |flags| flags, zl | nl | hl | cl, 0);

//    test_op!(addhl_test, add_hl, (0xFFFF, CpuRegister16::AB), a, 0x10000, |flags| flags, zl | nl  | hl | cl, 1);
  //  test_op!(addhl_test1, add_hl, (1256, CpuRegister16::CD), a, 1256, |flags| flags, nl | hl | cl, 0);


    /*
    test_op!(inc_test,  inc, (0xF, CpuRegister::C), a, 0xF, |flags| flags, zl | nl  | hl | cl, 0xF);
    test_op!(inc_test1, inc, (0xF, CpuRegister::D), a, 0xF, |flags| flags, nl | hl | cl, 0);
    test_op!(inc_test2, inc, (0xF, CpuRegister::B), a, 0xF, |flags| flags, nl | hl | cl , 0xF0);

    */

    //dec



   /* //TODO: flag tests on BCD
    test_op!(bcd_test1, daa, (0x15, ), a, 0x15, |_| 0, 0, 0x15);
    test_op!(bcd_test1, daa, (0x70, ), a, 0x70, |_| 0, 0, 0x70);
    test_op!(bcd_test1, daa, (0x79, ), a, 0x79, |_| 0, 0, 0x79);
    test_op!(bcd_test1, daa, (0x3F, ), a, 0x3F, |_| 0, 0, 0x3F);
    */
}

