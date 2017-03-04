#[cfg(test)]
use super::constants::*;
#[allow(unused_imports)]
use cpu::*;

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
            
            assert_eq!(cpu.$output_reg, $expected_output);
            assert_eq!($flag_find_value(cpu.f), $flag_expected_value);
        }
    }
}

macro_rules! test_op_no_arg {
    ($func:ident, $method:ident, $output_reg:expr, $input:expr,
     $expected_output:expr, $flag_find_value:expr,
     $flag_expected_value:expr) => {
        
        #[test]
        fn $func() {
            let mut cpu = Cpu::new();
            
            cpu.set_register($output_reg, $input);
            cpu.$method($output_reg);
            
            assert_eq!(
                cpu.access_register($output_reg).expect("invalid register")
                    , $expected_output);
            assert_eq!($flag_find_value(cpu.f), $flag_expected_value);
        }
    }
}

macro_rules! test_op_really_no_arg {
    ($func:ident, $method:ident, $output_reg:expr, $input:expr,
     $expected_output:expr, $flag_find_value:expr,
     $flag_expected_value:expr) => {
        
        #[test]
        fn $func() {
            let mut cpu = Cpu::new();
            
            cpu.set_register($output_reg, $input);
            cpu.$method();
            
            assert_eq!(
                cpu.access_register($output_reg).expect("invalid register")
                    , $expected_output);
            assert_eq!($flag_find_value(cpu.f), $flag_expected_value);
        }
    }
}


macro_rules! test_op_no_arg16 {
    ($func:ident, $method:ident, $output_reg:expr, $input:expr,
     $expected_output:expr, $flag_find_value:expr,
     $flag_expected_value:expr) => {
        
        #[test]
        fn $func() {
            let mut cpu = Cpu::new();
            let old_hl = cpu.hl();

            cpu.set_register16($output_reg, $input);
            cpu.$method($output_reg);
            println!("{:X} + {:X} = {:X}. should be: {:X}, but expected {:X}", $input, old_hl, cpu.access_register16(CpuRegister16::HL), ($input + (old_hl as u32)) as u16, $expected_output);
            
            assert_eq!(
                cpu.access_register16($output_reg), $expected_output);
            assert_eq!($flag_find_value(cpu.f), $flag_expected_value);
        }
    }
}

macro_rules! test_op16 {
    ($func:ident, $method:ident, $input_reg:expr, $output_reg:expr, $input:expr,
     $expected_output:expr, $flag_find_value:expr,
     $flag_expected_value:expr) => {
        
        #[test]
        fn $func() {
            let mut cpu = Cpu::new();

            cpu.set_register16($input_reg, $input);
            cpu.$method($input_reg);
            
            assert_eq!(
                cpu.access_register16($output_reg)
                    , $expected_output);
            assert_eq!($flag_find_value(cpu.f), $flag_expected_value);
        }
    }
}



test_op!(add_const, add, (5, CpuRegister::Num(5)), a, 5 + 5, |flags| flags & ZL, 0,5);
//test_op!(add_reg, add, (5, CpuRegister::B), a, 5 + -5, |flags| flags, ZL, -5);
test_op!(add_mem_half_carry, add, (1, CpuRegister::HL), a, 15 + 1, |flags| flags, HL, 15);
test_op!(add_mem_half_carry1, add, (15, CpuRegister::HL), a, 15 + 1, |flags| flags, HL, 1);

//test_op!(adc_test, adc, (10, CpuRegister::E), a, 10 + 1, |flags| flags, 0, 1);
//    test_op!(adc_carry, adc, (10, CpuRegister::F), a, 10 + 1, |flags| flags, 0, cl);

        

test_op!(sub_const, sub, (5, CpuRegister::Num(5)), a, 5 - 5, |flags| flags, ZL | NLV | HL | CL, 0);
//test_op!(sub_reg, sub, (10, CpuRegister::B), a, 20, |flags| flags, NLV | HL | CL, -10);
test_op!(sub_reg2, sub, (10, CpuRegister::C), a, 0, |flags| flags, ZL | NLV | HL | CL, 10);
//test_op!(sub_reg_borrow, sub, (0, CpuRegister::D), a, -10, |flags| flags, NLV, 10);
//TODO: write half carry onlvy test:
//    test_op!(sub_reg_borrow_half, sub, (130, CpuRegister::D), a, 40, |flags| flags, nlv | hl, 32);

test_op!(and_test, and, (0xF, CpuRegister::B), a, 0xF, |flags| flags, HL, 0xF);
test_op!(and_test1, and, (0xF, CpuRegister::C), a, 0x1, |flags| flags, HL, 0x1);
test_op!(and_test2, and, (0xF, CpuRegister::E), a, 0, |flags| flags, ZL | HL, 0x70);

test_op!(or_test, or, (0xF, CpuRegister::C), a, 0xF, |flags| flags, 0, 0xF);
test_op!(or_test1, or, (0xF, CpuRegister::D), a, 0xF, |flags| flags, 0, 0);
test_op!(or_test2, or, (0xF, CpuRegister::B), a, 0xFF, |flags| flags, 0, 0xF0);
test_op!(or_test3, or, (0, CpuRegister::D), a, 0, |flags| flags, ZL, 0);

test_op!(xor_test, xor, (0xF, CpuRegister::C), a, 0, |flags| flags, ZL, 0xF);
test_op!(xor_test1, xor, (0xF, CpuRegister::D), a, 0xF, |flags| flags, 0, 0);
test_op!(xor_test2, xor, (0xF, CpuRegister::B), a, 0xFF, |flags| flags, 0, 0xF0);
test_op!(xor_test3, xor, (0, CpuRegister::D), a, 0, |flags| flags, ZL, 0);

    
test_op!(cp_test, cp, (0xF, CpuRegister::C), a, 0xF, |flags| flags,  ZL | NLV | HL , 0xF);
test_op!(cp_test1, cp, (0xF, CpuRegister::D), a, 0xF, |flags| flags, NLV | HL , 0);
// TODO: verify hl and cl flags here make sense:
test_op!(cp_test2, cp, (0xF, CpuRegister::B), a, 0xF, |flags| flags, NLV | HL , 0xF0);
test_op!(cp_test3, cp, (0, CpuRegister::D), a, 0, |flags| flags, ZL | NLV | HL, 0);
test_op!(cp_test4, cp, (0xF0, CpuRegister::B), a, 0xF0, |flags| flags, NLV | CL , 0xF);

test_op16!(addhl_test, add_hl, CpuRegister16::BC, CpuRegister16::HL, 0xFFFF, (0xFFFF + 0x14D) as u16, |flags| flags & 0x70, HL | CL);
test_op16!(addhl_test1, add_hl, CpuRegister16::DE, CpuRegister16::HL, 0,  (0 + 0x14D), |flags| flags & 0x70, 0);
test_op16!(addhl_test2, add_hl, CpuRegister16::DE, CpuRegister16::HL, 0xFFFF,  (0xFFFF + 0x14D) as u16, |flags| flags & 0x70, HL | CL);


//TODO: figure out why this broke
/*#[test]
fn addsp_test() {
    let mut cpu = Cpu::new();

    cpu.add_sp(CpuRegister16::Num(-0xE));
    assert_eq!(cpu.access_register16(CpuRegister16::SP), 0xFFF0);
    cpu.add_sp(CpuRegister16::Num(0xF));
    assert_eq!(cpu.access_register16(CpuRegister16::SP), 0xFFFF);
    cpu.add_sp(CpuRegister16::Num(-0xFF));
    assert_eq!(cpu.access_register16(CpuRegister16::SP), 0xFF00);
} */

test_op16!(inc16_0, inc16, CpuRegister16::BC, CpuRegister16::BC, 0, 1, |_| 0, 0);
test_op16!(inc16_1, inc16, CpuRegister16::DE, CpuRegister16::DE, 0xFFFF, 0, |_| 0, 0);
test_op16!(inc16_2, inc16, CpuRegister16::BC, CpuRegister16::BC, 127, 128, |_| 0, 0);

test_op16!(dec16_0, dec16, CpuRegister16::BC, CpuRegister16::BC, 1, 0, |_| 0, 0);
test_op16!(dec16_1, dec16, CpuRegister16::DE, CpuRegister16::DE, 0, 0xFFFF, |_| 0, 0);
test_op16!(dec16_2, dec16, CpuRegister16::BC, CpuRegister16::BC, 128, 127, |_| 0, 0);
test_op16!(dec16_3, dec16, CpuRegister16::BC, CpuRegister16::BC, 0xFF00, 0xFEFF, |_| 0, 0);

//128 = 0x80
// 0xFF7F



//TODO: overflow tests.  Figure out what the behavior should be and test for it
    
test_op_no_arg!(inc_test,  inc, CpuRegister::A, 0xE, 0xF, |flags| flags & 0xE0, 0);
test_op_no_arg!(inc_test1,  inc, CpuRegister::B, 0x0, 0x1, |flags| flags & 0xE0, 0);
//test_op_no_arg!(inc_test2,  inc, CpuRegister::C, -1, 0, |flags| flags & 0xE0, ZL | HL);
    
test_op_no_arg!(dec_test,  dec, CpuRegister::A, 0x10, 0xF, |flags| flags & 0xE0, NLV);
//test_op_no_arg!(dec_test1,  dec, CpuRegister::B, 0, -1, |flags| flags & 0xE0, NLV );
test_op_no_arg!(dec_test2,  dec, CpuRegister::C, 1, 0, |flags| flags & 0xE0, ZL | HL | NLV);

test_op_no_arg!(swap_test,   swap, CpuRegister::A, 0xFA, 0xAF, |flags| flags, 0);
test_op_no_arg!(swap_test1,  swap, CpuRegister::B, 0x12, 0x21, |flags| flags, 0);
test_op_no_arg!(swap_test2,  swap, CpuRegister::C, 0, 0, |flags| flags, ZL);


#[test]
fn hl_tests() {
    let mut cpu = Cpu::new();

    cpu.set_register16(CpuRegister16::HL, 0);
    assert_eq!(cpu.access_register16(CpuRegister16::HL), 0);

    cpu.set_register16(CpuRegister16::HL, 100);
    assert_eq!(cpu.access_register16(CpuRegister16::HL), 100);

    assert_eq!(cpu.access_register16(CpuRegister16::HL), cpu.hl());
    
    cpu.set_register16(CpuRegister16::HL, 0xFF);
    assert_eq!(cpu.access_register16(CpuRegister16::HL), 0xFF);

    cpu.set_register16(CpuRegister16::HL, 0x100);
    assert_eq!(cpu.access_register16(CpuRegister16::HL), 0x100);

    cpu.set_register16(CpuRegister16::HL, 0xFFFE);
    assert_eq!(cpu.access_register16(CpuRegister16::HL), 0xFFFE);
}

test_op_really_no_arg!(bcd_test1, daa, CpuRegister::A, 0x15, 0x15, |flags| flags & (ZL | HL), 0);
test_op_really_no_arg!(bcd_test2, daa, CpuRegister::A, 0x70, 0x70, |flags| flags & (ZL | HL), 0);
test_op_really_no_arg!(bcd_test3, daa, CpuRegister::A, 0x79, 0x79, |flags| flags & (ZL | HL), 0);
test_op_really_no_arg!(bcd_test4, daa, CpuRegister::A, 0x3F, 0x45, |flags| flags & (ZL | HL), 0);
test_op_really_no_arg!(bcd_test5, daa, CpuRegister::A, 0, 0, |flags| flags & (ZL | HL ), ZL);

test_op_really_no_arg!(cpl_test1, cpl, CpuRegister::A, 0x15, 0xEA, |flags| flags & (NLV | HL), (NLV | HL));
test_op_really_no_arg!(cpl_test2, cpl, CpuRegister::A, 0x70, 0x8F, |flags| flags & (NLV | HL), (NLV | HL));
test_op_really_no_arg!(cpl_test3, cpl, CpuRegister::A, 0xFF, 0x00, |flags| flags & (NLV | HL), (NLV | HL));
test_op_really_no_arg!(cpl_test4, cpl, CpuRegister::A, 0x00, 0xFF, |flags| flags & (NLV | HL), (NLV | HL));


#[test]
fn ccf_tests() {
    let mut cpu = Cpu::new();
    cpu.ccf();

    assert_eq!(cpu.f, 0x80);
    cpu.ccf();
    assert_eq!(cpu.f, 0x90);
    cpu.ccf();
    cpu.ccf();
    assert_eq!(cpu.f, 0x90);

    cpu.scf();
    assert_eq!(cpu.f, 0x90);
    cpu.ccf();
    assert_eq!(cpu.f, 0x80);
    cpu.scf();
    assert_eq!(cpu.f, 0x90);
}

#[test]
fn test_halt() {
    let mut cpu = Cpu::new();

    assert_eq!(cpu.state, CpuState::Normal);
    cpu.halt();
    assert_eq!(cpu.state, CpuState::Halt);

    cpu.enable_interrupts();
    cpu.set_vblank_interrupt_enabled();
    cpu.set_vblank();
    cpu.set_vblank_interrupt_bit();
    cpu.set_vblank_interrupt_stat();

    cpu.dispatch_opcode();

    assert_eq!(cpu.state, CpuState::Normal);
}

#[test]
fn test_stop() {
    let mut cpu = Cpu::new();

    assert_eq!(cpu.state, CpuState::Normal);
    cpu.stop();
    assert_eq!(cpu.state, CpuState::Stop);
    cpu.press_a();
    assert_eq!(cpu.state, CpuState::Normal);
    cpu.press_b();
    cpu.press_a();
    assert_eq!(cpu.state, CpuState::Normal);

    cpu.stop();
    cpu.press_start();
    cpu.press_select();
    assert_eq!(cpu.state, CpuState::Normal);
}

//TODO: Test execution of DI and EI

#[allow(dead_code)]
#[test]
fn test_interrupt_disabling() {
    //let mut cpu = Cpu::new();

   // cpu.di();
   // cpu.stop();
   // cpu.press_a();
   // assert_eq!(cpu.state, CpuState::Stop);
}


#[cfg(feature = "asm")]
mod assembly_tests {
    #[test]
    fn test_running_program() {
        use assembler::asm;
        
        let program = asm::parse_Input(r#"
.code
"#);
    }
}

#[test]
fn test_jumps() {

}
