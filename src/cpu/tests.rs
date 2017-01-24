#[cfg(test)]
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


test_op!(add_const, add, (5, CpuRegister::Num(5)), a, 5 + 5, |flags| flags & zl, 0,5);
test_op!(add_reg, add, (5, CpuRegister::B), a, 5 + -5, |flags| flags, zl, -5);
test_op!(add_mem_half_carry, add, (1, CpuRegister::HL), a, 15 + 1, |flags| flags, hl, 15);
test_op!(add_mem_half_carry1, add, (15, CpuRegister::HL), a, 15 + 1, |flags| flags, hl, 1);

test_op!(adc_test, adc, (10, CpuRegister::E), a, 10 + 1, |flags| flags, 0, 1);
//    test_op!(adc_carry, adc, (10, CpuRegister::F), a, 10 + 1, |flags| flags, 0, cl);

        

test_op!(sub_const, sub, (5, CpuRegister::Num(5)), a, 5 - 5, |flags| flags, zl | nl | hl | cl, 0);
test_op!(sub_reg, sub, (10, CpuRegister::B), a, 20, |flags| flags, nl | hl | cl, -10);
test_op!(sub_reg2, sub, (10, CpuRegister::C), a, 0, |flags| flags, zl | nl | hl | cl, 10);
test_op!(sub_reg_borrow, sub, (0, CpuRegister::D), a, -10, |flags| flags, nl, 10);
//TODO: write half carry only test:
//    test_op!(sub_reg_borrow_half, sub, (130, CpuRegister::D), a, 40, |flags| flags, nl | hl, 32);

test_op!(and_test, and, (0xF, CpuRegister::B), a, 0xF, |flags| flags, hl, 0xF);
test_op!(and_test1, and, (0xF, CpuRegister::C), a, 0x1, |flags| flags, hl, 0x1);
test_op!(and_test2, and, (0xF, CpuRegister::E), a, 0, |flags| flags, zl | hl, 0x70);

test_op!(or_test, or, (0xF, CpuRegister::C), a, 0xF, |flags| flags, 0, 0xF);
test_op!(or_test1, or, (0xF, CpuRegister::D), a, 0xF, |flags| flags, 0, 0);
test_op!(or_test2, or, (0xF, CpuRegister::B), a, 0xFF, |flags| flags, 0, 0xF0);
test_op!(or_test3, or, (0, CpuRegister::D), a, 0, |flags| flags, zl, 0);

test_op!(xor_test, xor, (0xF, CpuRegister::C), a, 0, |flags| flags, zl, 0xF);
test_op!(xor_test1, xor, (0xF, CpuRegister::D), a, 0xF, |flags| flags, 0, 0);
test_op!(xor_test2, xor, (0xF, CpuRegister::B), a, 0xFF, |flags| flags, 0, 0xF0);
test_op!(xor_test3, xor, (0, CpuRegister::D), a, 0, |flags| flags, zl, 0);

    
test_op!(cp_test, cp, (0xF, CpuRegister::C), a, 0xF, |flags| flags, zl | nl  | hl | cl, 0xF);
test_op!(cp_test1, cp, (0xF, CpuRegister::D), a, 0xF, |flags| flags, nl | hl | cl, 0);
// TODO: verify hl and cl flags here make sense:
test_op!(cp_test2, cp, (0xF, CpuRegister::B), a, 0xF, |flags| flags, nl | hl | cl , 0xF0);
test_op!(cp_test3, cp, (0, CpuRegister::D), a, 0, |flags| flags, zl | nl | hl | cl, 0);

//    test_op!(addhl_test, add_hl, (0xFFFF, CpuRegister16::AB), a, 0x10000, |flags| flags, zl | nl  | hl | cl, 1);
  //  test_op!(addhl_test1, add_hl, (1256, CpuRegister16::CD), a, 1256, |flags| flags, nl | hl | cl, 0);


    
test_op_no_arg!(inc_test,  inc, CpuRegister::A, 0xE, 0xF, |flags| flags & 0xE0, 0);
test_op_no_arg!(inc_test1,  inc, CpuRegister::B, 0x0, 0x1, |flags| flags & 0xE0, 0);
test_op_no_arg!(inc_test2,  inc, CpuRegister::C, -1, 0, |flags| flags & 0xE0, zl | hl);
 //   test_op_no_arg!(inc_t`oest1, inc, (0xF, CpuRegister::D), a, 0xF, |flags| flags, nl | hl | cl, 0);
   // test_op_no_arg!(inc_test2, inc, (0xF, CpuRegister::B), a, 0xF, |flags| flags, nl | hl | cl , 0xF0);

    

    //dec



   /* //TODO: flag tests on BCD
    test_op!(bcd_test1, daa, (0x15, ), a, 0x15, |_| 0, 0, 0x15);
    test_op!(bcd_test1, daa, (0x70, ), a, 0x70, |_| 0, 0, 0x70);
    test_op!(bcd_test1, daa, (0x79, ), a, 0x79, |_| 0, 0, 0x79);
    test_op!(bcd_test1, daa, (0x3F, ), a, 0x3F, |_| 0, 0, 0x3F);
    */


