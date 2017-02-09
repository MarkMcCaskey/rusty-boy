#[cfg(test)]

// use super::CSL::*;
use super::asm;

#[test]
fn parse_instruction() {
    let insts = asm::parse_Input(".code NOP").unwrap();
    assert_eq!(insts[0], 0);
}

#[test]
fn parse_instructions() {
    let insts = asm::parse_Input(".code SUB B ").unwrap();
}
