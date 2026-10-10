#[derive(PartialEq, Eq, Debug, Clone)]
pub enum Event {
    Print(Vec<u8>),
    Control(u8),
    Escape(EscapeSequence),
    Csi(CsiSequence),
    Osc(OscSequence),
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct EscapeSequence {
    pub intermediates: Vec<u8>,
    pub final_byte: u8,
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct CsiSequence {
    pub parameter_bytes: Vec<u8>,
    pub intermediates: Vec<u8>,
    pub final_byte: u8,
}

#[derive(PartialEq, Eq, Copy, Debug, Clone)]
pub enum OscTerminator {
    Bell,
    St,
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct OscSequence {
    pub payload: Vec<u8>,
    pub terminator: OscTerminator,
}
