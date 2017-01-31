
#[derive(Debug, PartialEq)]
pub enum ShowableThing {
    Address { addr: u16 },
    Breakpoints,
}

#[derive(Debug, PartialEq)]
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
