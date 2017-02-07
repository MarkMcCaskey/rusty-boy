#[cfg(test)]

use super::CSL::*;
use super::asm;

#[test]
fn parse_number() {
    let insts = asm::parse_Input(".code NOP").unwrap();
    assert_eq!(insts[0], 0);
}
