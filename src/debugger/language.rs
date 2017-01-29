#[derive(Debug)]
pub enum DebuggerAction {
    WatchPoint { addr: u16 },
    UnwatchPoint { addr: u16 },
    SetBreakPoint { addr: u16 },
    UnsetBreakPoint { addr: u16 },
    Step,
    Run,
    Reset,
    Echo { str: String },
}

pub enum Expression {
    num { n: i32 },
}
