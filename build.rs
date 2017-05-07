#[cfg(feature="asm")]
extern crate lalrpop;

#[cfg(feature="asm")]
fn main() {
    lalrpop::process_root().unwrap();
}

#[cfg(not(feature="asm"))]
fn main() {}
