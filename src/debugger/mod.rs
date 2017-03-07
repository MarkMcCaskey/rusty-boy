//! ncurses-based TUI interactive debugger
mod language;
#[allow(unknown_lints, useless_attribute, needless_lifetimes, match_same_arms, cyclomatic_complexity, clone_on_copy, type_complexity, dead_code, unused_comparisons, unused_label, absurd_extreme_comparisons)]
#[cfg(feature = "debugger")]
mod dbglanguage;
pub mod graphics;
#[cfg(feature = "debugger")] mod tests;

#[cfg(not(feature = "debugger"))]
mod dbglanguage {
    //TODO: improve this if you can
    #[allow(non_snake_case, unused_variables)]
    pub fn parse_Input(input: &str) -> ! {
        panic!("Compile with --features=debugger to use the debugging language");
    }
}
