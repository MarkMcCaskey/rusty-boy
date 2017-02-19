#[allow(non_snake_case)]
pub mod CSL;
#[allow(unknown_lints, useless_attribute, needless_lifetimes, match_same_arms, cyclomatic_complexity, clone_on_copy, type_complexity, dead_code, unused_comparisons, unused_label, absurd_extreme_comparisons)]
#[cfg(feature = "asm")]
pub mod asm;
#[cfg(not(feature = "asm"))]
pub mod asm {
    //TODO: Add better error handling later
    #[allow(non_snake_case, unused_variables)]
    pub fn parse_Input(input: &str) {
        panic!("Turn on asm feature if you want to use the assembler");
    }
}
#[cfg(feature = "asm")]
mod tests;
