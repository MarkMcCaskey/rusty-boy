use cpu::*;
use cpu::constants::*;

#[derive(Debug, PartialEq, Clone)]
pub enum Value {
    Literal8(u8),
    Literal16(u16),
    Identifer(&'static str),
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

pub fn unary_dispatch(inst: Instruction, v: Value) -> (u8, u8) {
    // let z = (inst.prefix >> 6) & 3;
    // match inst {
    //
    unimplemented!();

}

pub fn binary_dispatch(inst: Instruction, v1: Value, v2: Value) -> (u8, u8, u8) {
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

/// Returns in Big Endian
pub fn lookup_prefix(n: u8, reg: CpuRegister) -> (u8, u8) {
    let z = cpuReg_dispatch(reg);
    let value = (n << 4) | z;

    (0xCB, value)
}
