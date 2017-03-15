// Code courtesy of spawnedartifact

extern crate clap;

use std::num::Wrapping;

#[allow(unknown_lints)]
#[allow(many_single_char_names)]
pub fn pp_opcode(first_byte: u8, second_byte: u8, third_byte: u8, pc: u16) -> (String, u8) {
    let x = (first_byte >> 6) & 0b011;
    let y = (first_byte >> 3) & 0b111;
    let z = first_byte & 0b111;
    let p = (first_byte >> 4) & 0b011;
    let q = (first_byte >> 3) & 0b001;

    let mut instruction_size = 1;

    // This garbage is here because two closures can't borrow same var.
    let mut prefix_used = 0;
    let mut used_a8 = 0;
    let mut used_d8 = 0;
    let mut used_r8 = 0;
    let mut used_a16 = 0;
    let mut used_d16 = 0;

    // This moved here to create a scope for closures to allow accessing
    // "used_*" vars later.
    let mnemonic = {
        let mut prefix = || prefix_used += 1;

        // Argument accessors. Will brake if used twice in a
        // row. Because of this, there is almost no point in using
        // them.
        let mut a8 = || {
            used_a8 += 1; //instruction_size += 1;
            format!("${:02X}", second_byte)
        };

        let mut d8 = || {
            used_d8 += 1; //instruction_size += 1;
            format!("${:02X}", second_byte)
        };

        // Because jump is relative to post pc increment we need to
        // know instruction size here.
        let mut r8 = |insize| {
            used_r8 += 1; //instruction_size += 1;
            // jump is relative to post pc increment!
            let addr = (Wrapping(pc) + Wrapping(insize) + Wrapping((second_byte as i8) as u16)).0;
            format!("Addr_{:04X}", addr)
        };

        let mut a16 = || {
            used_a16 += 2; //instruction_size += 2;
            format!("${:04X}",
                    (((third_byte as u16) << 8) | (second_byte as u16)))
        };

        let mut d16 = || {
            used_d16 += 2; //instruction_size += 2;
            format!("${:04X}",
                    (((third_byte as u16) << 8) | (second_byte as u16)))
        };


        // Converting indexes encoded in commands to symbolic arguments
        fn idx_r(i: u8) -> &'static str {
            ["B", "C", "D", "E", "H", "L", "(HL)", "A"][i as usize]
        };

        fn idx_rp(i: u8) -> &'static str {
            ["BC", "DE", "HL", "SP"][i as usize]
        };

        fn idx_rp2(i: u8) -> &'static str {
            ["BC", "DE", "HL", "AF"][i as usize]
        };

        fn idx_cc(i: u8) -> &'static str {
            ["NZ", "Z", "NC", "C"][i as usize]
        };

        fn idx_alu(i: u8) -> &'static str {
            ["ADD A,", "ADC A,", "SUB", "SBC A,", "AND", "XOR", "OR", "CP"][i as usize]
        };

        fn idx_rot(i: u8) -> &'static str {
            ["RLC", "RRC", "RL", "RR", "SLA", "SRA", "SWAP", "SRL"][i as usize]
        };

        fn illegal_op(byte: u8) -> String {
            format!(".DB ${:02X}", byte)
        }

        // The value of mnemonic =
        match x {
            0 => {
                match z {
                    0 => {
                        match y {
                            0 => "NOP".to_string(),
                            1 => format!("LD ({}),SP", a16()),
                            2 => format!("STOP {}", d8()),
                            3 => format!("JR {}", r8(2)),
                            4 => format!("JR NZ,{}", r8(2)),
                            5 => format!("JR Z,{}", r8(2)),
                            6 => format!("JR NC,{}", r8(2)),
                            7 => format!("JR C,{}", r8(2)),
                            _ => unreachable!("Impossible opcode"),
                        }
                    }
                    1 => {
                        match q {
                            0 => format!("LD {},{}", idx_rp(p), d16()),
                            1 => format!("ADD HL,{}", idx_rp(p)),
                            _ => unreachable!("Impossible opcode"),
                        }
                    }
                    2 => {
                        match q {
                            0 => {
                                match p {
                                    0 => "LD (BC),A".to_string(),
                                    1 => "LD (DE),A".to_string(),
                                    2 => "LD (HL+),A".to_string(),
                                    3 => "LD (HL-),A".to_string(),
                                    _ => unreachable!("Impossible opcode"),
                                }
                            }
                            1 => {
                                match p {
                                    0 => "LD A,(BC)".to_string(),
                                    1 => "LD A,(DE)".to_string(),
                                    2 => "LD A,(HL+)".to_string(),
                                    3 => "LD A,(HL-)".to_string(),
                                    _ => unreachable!("Impossible opcode"),
                                }
                            }
                            _ => unreachable!("Impossible opcode"),
                        }
                    }
                    3 => {
                        match q {
                            0 => format!("INC {}", idx_rp(p)),
                            1 => format!("DEC {}", idx_rp(p)),
                            _ => unreachable!("Impossible opcode"),
                        }
                    }
                    4 => format!("INC {}", idx_r(y)),
                    5 => format!("DEC {}", idx_r(y)),
                    6 => format!("LD {},{}", idx_r(y), d8()),
                    7 => {
                        match y {
                            0 => "RLCA".to_string(),
                            1 => "RRCA".to_string(),
                            2 => "RLA".to_string(),
                            3 => "RRA".to_string(),
                            4 => "DAA".to_string(),
                            5 => "CPL".to_string(),
                            6 => "SCF".to_string(),
                            7 => "CCF".to_string(),
                            _ => unreachable!("Impossible opcode"),
                        }
                    }
                    _ => unreachable!("Impossible opcode"),
                }
            }
            1 => {
                match (z, y) {
                    (6, 6) => "HALT".to_string(),
                    _ => format!("LD {},{}", idx_r(y), idx_r(z)),
                }
            }
            // FIXME cheating here a bit with idx_alu value
            2 => format!("{} {}", idx_alu(y), idx_r(z)),
            3 => {
                match z {
                    0 => {
                        match y {
                            0...3 => format!("RET {}", idx_cc(y)),
                            // 4 => format!("LDH ({}),A", a8()),
                            4 => format!("LD ($FF00+{}),A", a8()),
                            5 => format!("ADD SP,{}", r8(0)), // FIXME
                            // 6 => format!("LDH A,({})", a8()),
                            6 => format!("LD A,($FF00+{})", a8()),
                            7 => format!("LD HL,(sp + {})", r8(0)), // FIXME
                            _ => unreachable!("Impossible opcode"),
                        }
                    }
                    1 => {
                        match q {
                            0 => format!("POP {}", idx_rp2(p)),
                            1 => {
                                match p {
                                    0 => "RET".to_string(),
                                    1 => "RETI".to_string(),
                                    2 => "JP (HL)".to_string(),
                                    3 => "LD SP,HL".to_string(),
                                    _ => unreachable!("Impossible opcode"),
                                }
                            }
                            _ => unreachable!("Impossible opcode"),
                        }
                    }
                    2 => {
                        match y {
                            0...3 => format!("JP {},{}", idx_cc(y), a16()),
                            4 => "LD ($FF00+C),A".to_string(),
                            5 => format!("LD ({}),A", a16()),
                            6 => "LD A,($FF00+C)".to_string(),
                            7 => format!("LD A,({})", a16()),
                            _ => unreachable!("Impossible opcode"),
                        }
                    }
                    3 => {
                        match y {
                            0 => format!("JP {}", a16()),
                            1 => {
                                // Prefix
                                prefix();
                                let x = (second_byte >> 6) & 0b011;
                                let y = (second_byte >> 3) & 0b111;
                                let z = (second_byte >> 0) & 0b111;

                                // WARNING: a8, d8, d16, etc. are broken here
                                match x {
                                    0 => format!("{} {}", idx_rot(y), idx_r(z)),
                                    1 => format!("BIT {},{}", y, idx_r(z)),
                                    2 => format!("RES {},{}", y, idx_r(z)),
                                    3 => format!("SET {},{}", y, idx_r(z)),
                                    _ => unreachable!("Impossible opcode"),
                                }
                            }
                            2...5 => illegal_op(first_byte),
                            6 => "DI".to_string(),
                            7 => "EI".to_string(),
                            _ => unreachable!("Impossible opcode"),
                        }
                    }
                    4 => {
                        match y {
                            0...3 => format!("CALL {},{}", idx_cc(y), a16()),
                            4...7 => illegal_op(first_byte),
                            _ => unreachable!("Impossible opcode"),
                        }
                    }
                    5 => {
                        match q {
                            0 => format!("PUSH {}", idx_rp2(p)),
                            1 => {
                                match p {
                                    0 => format!("CALL {}", a16()),
                                    1...3 => illegal_op(first_byte),
                                    _ => unreachable!("Impossible opcode"),
                                }
                            }
                            _ => unreachable!("Impossible opcode"),
                        }
                    }
                    6 => format!("{} {}", idx_alu(y), d8()),
                    7 => format!("RST {:02X}H", y * 8),
                    _ => unreachable!("Impossible opcode"),
                }
            }
            _ => unreachable!("Impossible opcode"),
        }
    };

    // This garbage is here because two closures can't borrow same var.
    instruction_size += prefix_used;
    instruction_size += used_a8;
    instruction_size += used_r8;
    instruction_size += used_d8;
    instruction_size += used_a16;
    instruction_size += used_d16;
    (mnemonic, instruction_size)
}

#[allow(dead_code)]
fn disasm_rom(rom: [u8; 0x8000], rom_size: usize) {
    let mut pc = 0;

    let crash_preventing_size = rom_size - 2; // FIXME
    while pc < crash_preventing_size {
        let b1 = rom[pc];
        let b2 = rom[pc + 1];
        let b3 = rom[pc + 2];
        let (mnemonic, size) = pp_opcode(b1, b2, b3, pc as u16);
        print!("\t{:25}; ${:04X} ", mnemonic, pc);
        match size {
            1 => print!("{:02x}", b1),
            2 => print!("{:02x}{:02x}", b1, b2),
            3 => print!("{:02x}{:02x}{:02x}", b1, b2, b3),
            _ => unreachable!(),
        }
        println!();

        pc += size as usize;
    }
}

pub fn disasm_rom_to_vec(rom: [u8; 0x8000], rom_size: usize) -> Vec<(String, u16)> {
    let mut pc = 0;
    let mut ret: Vec<(String, u16)> = vec![];

    while pc < rom_size {
        let (mnemonic, size) = pp_opcode(rom[pc], rom[pc + 1], rom[pc + 2], pc as u16);
        ret.push((format!("0x{:04X}\t{}", pc, mnemonic), pc as u16));
        pc += size as usize;
    }

    ret
}

pub fn binsearch_inst(vec: &Vec<(String, u16)>,
                      desired_pc: u16,
                      begin: usize,
                      end: usize)
                      -> Option<usize> {
    if end < begin {
        return None;
    } else if end - begin <= 10 {
        for (x, &(_, b)) in vec.iter()
                .enumerate()
                .take((end + 1))
                .skip(begin) {
            //            let (_, b) = vec[x];
            if b == desired_pc {
                return Some(x);
            }
        }
        return None;
    }

    let search = if (end + begin) % 2 == 0 {
        (end + begin) / 2
    } else {
        ((end + begin) / 2) + 1
    };

    let (_, b) = vec[search];

    if b == desired_pc {
        Some(search)
    } else if b > desired_pc {
        binsearch_inst(vec, desired_pc, begin, (search + 1) as usize)
    } else {
        // if b > desired_pc {
        binsearch_inst(vec, desired_pc, (search - 1) as usize, end)
    }
}

#[allow(dead_code)]
fn main() {
    // // Print "[prefix] opcode size mnemonic" table
    // for i in 0..255 {
    //     let (mnemonic, size) = pp_opcode(i, 0xF2, 0x02, 0x2FFF);
    //     println!("0x{:02X} {} {:?}", i, size, mnemonic);
    // }
    // for i in 0..255 {
    //     let (mnemonic, size) = pp_opcode(0xCB, i, 0x02, 0x2FFF);
    //     println!("0xCB 0x{:02X} {} {:?}", i, size, mnemonic);
    // }
    use std::fs::File;
    use std::io::Read;
    use clap::{Arg, App};

    let matches = App::new("disasm")
        .version("0.1")
        .author("spawnedartifact")
        .about("GB z80 disassembler")
        .arg(Arg::with_name("game")
                 .index(1)
                 .value_name("FILE")
                 .help("Specifies ROM to disassemble")
                 .takes_value(true))
        .get_matches();


    let file_path = matches.value_of("game").expect("Could not open rom");
    let mut rom = File::open(file_path).expect("Could not open rom file");
    let mut rom_buffer: [u8; 0x8000] = [0u8; 0x8000];

    let rom_size = rom.read(&mut rom_buffer).unwrap();

    disasm_rom(rom_buffer, rom_size);
}
