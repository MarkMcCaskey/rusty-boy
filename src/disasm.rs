#[macro_use]
extern crate nom;

#[derive(Debug,PartialEq)]
enum Instruction{
    Arithmetic {i: ArithmeticInstruction},
    ControlFlow {i: JumpInstruction},
};

#[derive(Debug,PartialEq)]
enum ArithmeticInstruction{
    Add {},
    Sub {},
};

#[derive(Debug,PartialEq)]
enum JumpInstruction {};

fn main() {
    print!("Kappa");
}

fn explode_instruction(inst: u8) -> (u8,u8,u8) {
    let x = (inst >> 6) & 0x3;
    let y = (inst >> 3) & 0x7;
    let z = (inst     ) & 0x7;

    (x,y,z)
}

pub fn prefix_inst(input: &[u8]) -> IResult<&[u8], Vec<Instructions>> {
    //0xCB = 11 001 011
    let (x,y,z) = explode_instruction();
    
}

pub fn (&mut cpu: cpu::cpu) -> String {
    let (first_byte, second_byte, third_byte, fourth_byte)
        = cpu.read_instruction();
    let x = (first_byte >> 6) & 0x3;
    let y = (first_byte >> 3) & 0x7;
    let z = first_byte        & 0x7;

    let uf = "The impossible happened!";

    if first_byte == 0xCB { //prefixed instruction
        let x = (second_byte >> 6) & 0x3;
        let y = (second_byte >> 3) & 0x7;
        let z = second_byte        & 0x7;
        
        match x { // xxyy yzzz
            0 => match y {
                //(cpu_dispatch(z))
                0 => "RLC " + cpu_dispatch(z),
                1 => self.rrc(cpu_dispatch(z)),
                2 => self.rl(cpu_dispatch(z)),
                3 => self.rr(cpu_dispatch(z)),
                4 => self.sla(cpu_dispatch(z)),
                5 => self.sra(cpu_dispatch(z)),
                6 => self.swap(cpu_dispatch(z)),
                7 => self.srl(cpu_dispatch(z)),
                _ => unreachable!(uf),
            },

                1 => self.bit(y, cpu_dispatch(z)),

                2 => self.res(y, cpu_dispatch(z)),

                3 => self.set(y, cpu_dispatch(z)),

                _ => unreachable!(uf),
            }

            inst_time =
                if CpuRegister::HL == cpu_dispatch(z) {16} else {8};

            self.inc_pc();
        } else { //unprefixed instruction
            match x {
                0 =>
                    match z {
                        0 =>
                            match y {
                                0        => self.nop(), //0x00
                                1        => {
                                    self.ldnnsp(second_byte, third_byte);
                                    self.inc_pc();
                                    self.inc_pc();
                                    inst_time = 20;
                                }, //0x08
                                2        => self.stop(), //0x10
                                3        => {
                                    self.jrn(second_byte as i8);
                                    self.inc_pc();
                                    inst_time = 8;
                                },  //0x18
                                v @ 4...7 => {
                                    inst_time = 8 + if
                                        self.jrccn(cc_dispatch(v-4),
                                                   second_byte as i8) {4}
                                    else {0};
                                    self.inc_pc();
                                },  //0x20, 0x28, 0x30, 0x38
                                _        => unreachable!(uf),
                            },
                        
                        1 =>  //00yy y001
                        {
                            inst_time = if y % 2 == 0 {
                                self.ldnnn16(cpu16_dispatch(y/2),
                                             second_byte, third_byte);
                                self.inc_pc();
                                self.inc_pc();
                                12
                            } else {
                                self.add_hl(cpu16_dispatch(y/2));
                                8
                            };
                        },
                        
                        2 => //00yy y010
                        {
                            match y {
                                0 | 2 => self.ldna16(cpu16_dispatch(y/2)),
                                1 | 3 => self.ldan16(cpu16_dispatch(y/2)),
                                4 => self.ldihla(),
                                5 => self.ldiahl(),
                                6 => self.lddhla(),
                                7 => self.lddahl(),
                                _ => unreachable!(uf),
                            }
                            inst_time = 8;
                        },

                        3 => //00yy y011
                        {
                            even_odd_dispatch!(y, self, inc16, dec16, cpu16_dispatch, cpu16_dispatch, 1, 1);
                            inst_time = 8;
                        },

                        4 => //00yy y100
                        {
                            self.inc(cpu_dispatch(y));
                            inst_time =
                                if cpu_dispatch(y) == CpuRegister::HL {
                                    12} else {4};
                        },
                        
                        5 =>
                        {
                            self.dec(cpu_dispatch(y));
                            inst_time =
                                if cpu_dispatch(y) == CpuRegister::HL {
                                    12} else {4};
                        },

                        6 =>
                        {
                            self.ldnnn(cpu_dispatch(y), second_byte);
                            self.inc_pc();
                            inst_time =
                                if cpu_dispatch(y) == CpuRegister::HL {
                                    12} else {8};
                        },

                        7 => match y { //00yy y111
                            0 => self.rlca(),
                            1 => self.rrca(),
                            2 => self.rla(),
                            3 => self.rra(),
                            4 => self.daa(),
                            5 => self.cpl(),
                            6 => self.scf(),
                            7 => self.ccf(),
                            _ => unreachable!(uf),
                        },

                        _ => unreachable!(uf),
                    }, //end x=0

                1 => match (z,y) {
                    (6,6) => self.halt(),
                    (n,m) => {
                        self.ldr1r2(cpu_dispatch(m), cpu_dispatch(n));
                        inst_time = match (cpu_dispatch(m), cpu_dispatch(n)) {
                            (CpuRegister::HL,_) => 8,
                            (_,CpuRegister::HL) => 8,
                            _                   => 4,
                        }
                    },
                }, //end x = 1

                2 => { match y //10yy y000
                       {
                           0 => self.add(cpu_dispatch(z)),
                           1 => self.adc(cpu_dispatch(z)),
                           2 => self.sub(cpu_dispatch(z)),
                           3 => self.sbc(cpu_dispatch(z)),
                           4 => self.and(cpu_dispatch(z)),
                           5 => self.xor(cpu_dispatch(z)),
                           6 => self.or(cpu_dispatch(z)),
                           7 => self.cp(cpu_dispatch(z)),
                           _ => unreachable!(uf),
                       };
                       //TODO: double check the line below 
                       inst_time = if z == 6 {8} else {4};
                }, //end x = 2

                3 => match z //11yy y000
                {

                    0 => match y {
                        v @ 0...3 => inst_time = if
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
                            self.ldhan(second_byte);
                            self.inc_pc();
                            inst_time = 12;
                        },
                        7 => {
                            self.ldhlspn(second_byte);
                            self.inc_pc();
                            inst_time = 12;
                        },
                        _ => unreachable!(uf),
                    },

                    1 => if y % 2 == 0 { //11yy y001
                        let adjusted_value = y / 2;
                        let val = self.pop_from_stack();
                        self.set_register16(cpu16_dispatch(adjusted_value), val);
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
                            _ => unreachable!(uf),
                        }
                    },

                    2 => match y {
                        v @ 0...3 => { // 11yy y010
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
                            self.ldan16c(second_byte, third_byte);
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
                        _ => unreachable!(uf),
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
                        _ => panic!("Illegal opcode"),
                    },

                    4 => {

                        let const_val = (second_byte as u16) | ((third_byte as u16) << 8); 
                        inst_time =
                            if self.callccnn(cc_dispatch(y),
                                             const_val) {24} else {12};
                        self.inc_pc();
                        self.inc_pc();
                    },

                    5 => {
                        if y % 2 == 0 {
                            let value = self.access_register16(cpu16_dispatch(y / 2));
                            self.push_onto_stack(value);
                            inst_time = 16;
                        } else if y == 1 {
                            let const_val = (second_byte as u16) | ((third_byte as u16) << 8); 
                            self.callnn(const_val);
                            self.inc_pc();
                            self.inc_pc();
                            inst_time = 24;
                        } else {
                            panic!("Invalid opcode: {}", first_byte)
                        }
                    },

                    6 => {
                        match y {
                            0 => self.add(CpuRegister::Num(second_byte as i8)),
                            1 => self.adc(CpuRegister::Num(second_byte as i8)),
                            2 => self.sub(CpuRegister::Num(second_byte as i8)),
                            3 => self.sbc(CpuRegister::Num(second_byte as i8)),
                            4 => self.and(CpuRegister::Num(second_byte as i8)),
                            5 => self.xor(CpuRegister::Num(second_byte as i8)),
                            6 => self.or(CpuRegister::Num(second_byte as i8)),
                            7 => self.cp(CpuRegister::Num(second_byte as i8)),
                            _ => unreachable!(uf),
                        };
                        inst_time = 8;
                        self.inc_pc();
                    },

                    7 => {
                        self.rst(8*y);
                        inst_time = 16;
                    },
                        
                    _ => unreachable!(uf),
                },
                _ => panic!("The impossible happened!"),
            }
        }
        
        self.inc_pc();

        inst_time
    }

