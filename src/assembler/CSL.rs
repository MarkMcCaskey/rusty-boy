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
    unimplemented!();
}

pub fn binary_dispatch(inst: Instruction, v1: Value, v2: Value) -> (u8, u8, u8) {
    unimplemented!();
}
