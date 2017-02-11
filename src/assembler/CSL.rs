use std::collections::HashMap;
use cpu::*;
use cpu::constants::*;

#[derive(Debug, PartialEq, Clone)]
pub enum Value {
    Literal8(u8),
    Literal16(u16),
    Label(String),
    Register8(CpuRegister),
    Register16(CpuRegister16),
}

#[derive(Debug, PartialEq, Clone)]
pub enum InstructionType {
    Zero,
    Unary(Value),
    Binary(Value, Value),
}

#[derive(Debug, PartialEq, Clone)]
pub struct Instruction {
    pub insttype: InstructionType,
    pub prefix: u8,
}

/// Carries relevant state/data about the program being assembled
#[derive(Debug)]
pub struct Environment {
    /// Labels that are resolved into addresses
    pub labels: HashMap<String, u16>,
}

impl Environment {
    pub fn new() -> Environment {
        Environment { labels: HashMap::new() }
    }
}

/// Takes a `Value` and returns it in a usable form (number)
pub fn resolve_value(v: Value, env: &mut Environment) -> Option<u16> {
    match v {
        Value::Literal8(n) => Some(n as u16),
        Value::Literal16(n) => Some(n),
        Value::Label(str) => {
            if let Some(&v) = env.labels.get(&str) {
                Some(v)
            } else {
                None
            }
        }
        _ => None,
    }
}

pub fn unary_dispatch(inst: Instruction, v: u8) -> (u8, u8) {
    (inst.prefix, v)
}

pub fn binary_dispatch(inst: Instruction, v1: u8, v2: u8) -> (u8, u8, u8) {
    unimplemented!();
}

pub fn cpuReg_dispatch(reg: CpuRegister) -> u8 {
    match reg {
        CpuRegister::B => 0,
        CpuRegister::C => 1,
        CpuRegister::D => 2,
        CpuRegister::E => 3,
        CpuRegister::H => 4,
        CpuRegister::L => 5,
        CpuRegister::HL => 6,
        CpuRegister::A => 7,
        _ => unreachable!(),
    }
}

pub fn cpuReg_dispatch16(reg: CpuRegister16) -> u8 {
    match reg {
        CpuRegister16::BC => 0,
        CpuRegister16::DE => 1,
        CpuRegister16::HL => 2,
        CpuRegister16::SP => 3,
        _ => unreachable!(),
    }
}


/// Returns in Big endian
pub fn lookup_prefix(n: u8, reg: CpuRegister) -> (u8, u8) {
    let z = cpuReg_dispatch(reg);
    let value = (n << 4) | z;

    (0xCB, value)
}

/// Dispatches number values from Strings for the PUSH and POP
/// instructions (uses AF, so not applicable in other situations)
pub fn push_pop_disp16(regname: &str) -> u8 {
    match regname {
        "BC" => 0,
        "DE" => 1,
        "HL" => 2,
        "AF" => 3,
        _ => unreachable!(),
    }
}

pub fn bytes_to_unary_instruction(a: u8, b: u8) -> Instruction {
    Instruction {
        insttype: InstructionType::Unary(Value::Literal8(b)),
        prefix: a,
    }
}

pub fn extract_8bit_literal(n: Value) -> Option<u8> {
    if let Value::Literal8(v) = n {
        Some(v)
    } else {
        None
    }
}
