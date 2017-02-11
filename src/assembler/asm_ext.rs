// #![crate_type="dylib"]
// #![feature(plugin_registrar, rustc_private)]
//
// extern crate syntax;
// extern crate rustc;
// extern crate rustc_plugin;
//
// use syntax::parse::token;
// use syntax::tokenstream::TokenTree;
// use syntax::ext::base::{ExtCtxt, MacResult, DummyResult, MacEager};
// use syntax::ext::build::AstBuilder;  // A trait for expr_usize.
// use syntax::ext::quote::rt::Span;
// use rustc_plugin::Registry;
//
// fn expand_asm(cx: &mut ExtCtxt, sp: Span, args: &[TokenTree]) -> Box<MacResult + 'static> {
// use rusty_boy::assembler::asm;
// use rusty_boy::assembler::language;
//
// asm::parseInput();
//
// }
//
//
// #[plugin_registrar]
// pub fn plugin_registrar(reg: &mut Registry) {
// reg.register_macro("z80asm", expand_asm);
// }
//
