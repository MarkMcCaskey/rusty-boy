//! ncurses-based TUI interactive debugger
#[allow(
    unknown_lints,
    clippy::useless_attribute,
    clippy::needless_lifetimes,
    clippy::match_same_arms,
    cyclomatic_complexity,
    clippy::clone_on_copy,
    clippy::type_complexity,
    dead_code,
    unused_comparisons,
    unused_label,
    clippy::absurd_extreme_comparisons
)]
#[cfg(feature = "debugger")]
pub mod graphics;
#[cfg(feature = "debugger")]
mod language;
#[cfg(feature = "debugger")]
mod tests;

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
    use crate::cpu::*;
    pub struct Debugger {}

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
