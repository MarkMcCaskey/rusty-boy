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
    let insts = asm::parse_Input(".code SUB B NOP NOP NOP NOP");
    assert!(insts.is_ok());
}

#[test]
fn parse_compare_output() {
    let insts = asm::parse_Input(
        r#"
.code 
NOP 
NOP
JR NZ, 0x32
ADD A, C
RLC C"#).unwrap();

    let inst_output = [0,0,0x20,0x32,0x81,0xCB,0x01];
    for i in 0..(inst_output.len()){
        assert_eq!(insts[i],inst_output[i]);
    }
}

#[test]
fn parse_compare_output1() {
    let insts = asm::parse_Input(
        r#"
.code 
NOP 
NOP
ADD A, C
RLC C"#).unwrap();

    let inst_output = [0,0,0x81,0xCB,0x01];
    for i in 0..(inst_output.len()){
        assert_eq!(insts[i],inst_output[i]);
    }
}

#[test]
fn parse_compare_output2() {
    let insts = asm::parse_Input(
        r#"
.code 
NOP 
NOP
JR NZ, 0x32
RLC C"#).unwrap();

    let inst_output = [0,0,0x20,0x32,0xCB,0x01];
    for i in 0..(inst_output.len()){
        assert_eq!(insts[i],inst_output[i]);
    }
}

#[test]
fn sum_odd_numbers_under_100() {
    let insts = asm::parse_Input(
        r#"
.code
LD B, 0

INC B
BIT 0, B
LD A, B
ADD HL, BC
CP 99 
JP NZ, 0x101
"#);
    println!("{:?}", insts);
    assert!(insts.is_ok());
}
