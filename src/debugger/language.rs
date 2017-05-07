use serde::*;
use nom;
use nom::{le_u16, hex_digit, hex_u32, space};


#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum ShowableThing {
    Address { addr: u16 },
    Breakpoints,
}



#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum DebuggerAction {
    WatchPoint { addr: u16 },
    UnwatchPoint { addr: u16 },
    SetBreakPoint { addr: u16 },
    UnsetBreakPoint { addr: u16 },
    Show { show: ShowableThing },
    Step,
    Run,
    Reset,
    Echo { str: String },
    RunToAddress { addr: u16 },
}

pub fn parse_debug_language(input: &str) -> Option<DebuggerAction> {
    match dbg_parser(input.as_bytes()) {
        nom::IResult::Done(_, out) => Some(out),
        _ => None,
    }
}

named!(dbg_parser<&[u8], DebuggerAction>, alt!(
    runtoaddress_parser | run_parser | step_parser | reset_parser | show_parser | watchpoint_parser
        | unwatchpoint_parser | setbreakpoint_parser | unsetbreakpoint_parser));

named!(run_parser<&[u8], DebuggerAction>, do_parse!(tag!("run") >> (DebuggerAction::Run)));
named!(step_parser<&[u8], DebuggerAction>, do_parse!(tag!("step") >> (DebuggerAction::Step)));
named!(reset_parser<&[u8], DebuggerAction>, do_parse!(tag!("reset") >> (DebuggerAction::Reset)));
named!(runtoaddress_parser<&[u8], DebuggerAction>, do_parse!(tag!("run to") >>
                                                          n: number_parser >>
                                                          (DebuggerAction::RunToAddress {addr: n})));
named!(show_parser<&[u8], DebuggerAction>, do_parse!(tag!("show") >>
                                                     many1!(space) >>
                                                     s: showablething_parser >>
                                                     (DebuggerAction::Show{show : s})));

named!(watchpoint_parser<&[u8], DebuggerAction>, do_parse!(
    tag!("watch") >>
        many1!(space) >>
        n: number_parser >>
        (DebuggerAction::WatchPoint{addr: n})));

named!(unwatchpoint_parser<&[u8], DebuggerAction>, do_parse!(
    tag!("unwatch") >>
        many1!(space) >>
        n: number_parser >>
        (DebuggerAction::UnwatchPoint{addr: n})));

named!(setbreakpoint_parser<&[u8], DebuggerAction>, do_parse!(
    tag!("set") >>
        many1!(space) >>
        tag!("breakpoint") >>
        many1!(space) >>
        n: number_parser >>
        (DebuggerAction::SetBreakPoint{addr: n})));

named!(unsetbreakpoint_parser<&[u8], DebuggerAction>, do_parse!(
    tag!("unset") >>
        many1!(space) >>
        tag!("breakpoint") >>
        many1!(space) >>
        n: number_parser >>
        (DebuggerAction::UnsetBreakPoint{addr: n})));

named!(showablething_parser<&[u8], ShowableThing>,
       alt!(do_parse!(tag!("breakpoints") >>
                      (ShowableThing::Breakpoints))
            | do_parse!(n: number_parser >>
                        (ShowableThing::Address{addr: n}))));

named!(number_parser<&[u8], u16>, alt!(le_u16 | hex_parser));


named!(hex_parser<&[u8], u16>, do_parse!(
    res: hex_u32 >>
        (res as u16)));
