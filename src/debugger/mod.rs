use nom::{IResult, digit, hex_digit};
mod language;
mod dbglanguage;
mod graphics;
mod tests;
use std;
use self::graphics::*;
use std::thread::sleep;
use std::time::Duration;


//#[derive(Debug, PartialEq)]
//#pub enum DebugEvent {
//#BreakPoint { pc: u16 },
//#Watch8 { reg: CpuRegister },
//#Watch16 { reg: CpuRegister16 },
//
//#pub fn parse_input(s: &str) -> Option<DebugEvent> {}
//
//#named!(number,
//#alt!(many1!(digit),
//#preceded!(alt!(tag!("0x"), tag!("0X")), many1!(hex_digit))));
//
//#named!(set_breakpoint, preceded!(tag!("set breakpoint"), number));
//#named!(unset_breakpoint,
//#preceded!(tag!("unset breakpoint"), alt!(number, tag!("all"))));
//
//#named!(command, do_parse!(tag!(":") >> alt!(set, unset)));
//
//#named!(set,
//#do_parse!(tag!("set") >>
//#many0!(space) >>
//#tag!("breakpoint") >>
//#n: number >>
//#(BreakPoint {n})));
//
//##[test]
//#fn set_test() {
//#let v = set(&b"set breakpoint 0x123");
//#assert_eq!(v, Done("", BreakPoint {0x123}));
//#}
//

// NOTE: non-blocking read as timeout(delay) or wtimeout(window,delay)
pub fn run_debugger(file_name: &str) -> ! {
    let mut dbg = Debugger::new(file_name);
    dbg.refresh_screen();
    loop {
        dbg.handle_input();
        dbg.refresh_screen();

        // run until breakpoint or end
        if dbg.should_run() {
            // Keep executing until breakpoint or other "PAUSE" condition
            while dbg.should_run() {
                dbg.run();
                dbg.refresh_screen();
            }
        } else {
            std::thread::sleep(Duration::from_millis(16));
        }
    }
}
