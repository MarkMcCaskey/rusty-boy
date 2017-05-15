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
    let insts = asm::parse_Input(r#"
.code 
NOP 
NOP
JR NZ, 0x32
ADD A, C
RLC C"#)
            .unwrap();

    let inst_output = [0, 0, 0x20, 0x32, 0x81, 0xCB, 0x01];
    for i in 0..(inst_output.len()) {
        assert_eq!(insts[i], inst_output[i]);
    }
}

#[test]
fn parse_compare_output1() {
    let insts = asm::parse_Input(r#"
.code 
NOP 
NOP
ADD A, C
RLC C"#)
            .unwrap();

    let inst_output = [0, 0, 0x81, 0xCB, 0x01];
    for i in 0..(inst_output.len()) {
        assert_eq!(insts[i], inst_output[i]);
    }
}

#[test]
fn parse_compare_output2() {
    let insts = asm::parse_Input(r#"
.code 
NOP 
NOP
JR NZ, 0x32
RLC C"#)
            .unwrap();

    let inst_output = [0, 0, 0x20, 0x32, 0xCB, 0x01];
    for i in 0..(inst_output.len()) {
        assert_eq!(insts[i], inst_output[i]);
    }
}

#[test]
fn sum_odd_numbers_under_100() {
    let insts = asm::parse_Input(r#"
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

#[test]
fn opcode_tests() {
    let insts = asm::parse_Input(r#"
.code
ADD A, B
SUB D
RET NZ
PUSH HL
RST 30H
EI
DEC HL
ADC A, 89
RST 08H
CPL
INC A
JR C, 0
INC D
LD A, B
LD C, E
RRC D
SET 1, C
SET 4, H
BIT 2, L
SLA E
SWAP (HL)
"#)
            .unwrap();
    let out_bytes = [0x80, 0x92, 0xC0, 0xE5, 0xF7, 0xFB, 0x2B, 0xCE, 89, 0xCF, 0x2F, 0x3C, 0x38,
                     0, 0x14, 0x78, 0x4B, 0xCB, 0x0A, 0xCB, 0xC9, 0xCB, 0xE4, 0xCB, 0x55, 0xCB,
                     0x23, 0xCB, 0x36];

    for i in 0..(out_bytes.len()) {
        println!("{}: {:X}, {:X}", i, insts[i], out_bytes[i]);
        assert_eq!(insts[i], out_bytes[i]);
    }
}
