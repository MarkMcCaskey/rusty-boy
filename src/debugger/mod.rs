//! ncurses-based TUI interactive debugger
#[cfg(feature = "debugger")]
mod language;
#[allow(unknown_lints, useless_attribute, needless_lifetimes, match_same_arms, cyclomatic_complexity, clone_on_copy, type_complexity, dead_code, unused_comparisons, unused_label, absurd_extreme_comparisons)]

#[cfg(feature = "debugger")]
pub mod graphics;
#[cfg(feature = "debugger")] mod tests;


/*
 **************************************************************************
 * Dummy mods below                                                       *
 **************************************************************************
*/

#[cfg(not(feature = "debugger"))]
mod dbglanguage {
    //TODO: improve this if you can
    #[allow(non_snake_case, unused_variables, dead_code)]
    pub fn parse_Input(input: &str) -> ! {
        panic!("Compile with --features=debugger to use the debugging language");
    }
}

#[cfg(not(feature = "debugger"))]
pub mod graphics {
    use cpu::*;
    pub struct Debugger {
        
    }

    #[allow(unused_variables, dead_code)]
    impl Debugger {
        pub fn new(gb: &Cpu) -> Debugger {
            panic!("Compile with --features=debugger to use the debugger")
        }

        pub fn step(&mut self, cpu: &mut Cpu) {
            panic!("Compile with --features=debugger to use the debugger")
        }
        pub fn die(&mut self) {}
    }
}
