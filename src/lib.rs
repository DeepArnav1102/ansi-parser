pub mod csi;
mod parser;
pub mod util;

pub use csi::{Color, CsiAction, EraseDisplayMode, EraseLineMode, SgrAction, StandardColor};
pub use parser::Parser;
pub use util::{strip_ansi, strip_ansi_to_string};

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum Event {
    Print(Vec<u8>),
    Control(u8),
    Escape(EscapeSequence),
    Csi(CsiSequence),
    Osc(OscSequence),
    Dcs(DcsSequence),
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

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct DcsSequence {
    pub parameter_bytes: Vec<u8>,
    pub intermediates: Vec<u8>,
    pub final_byte: u8,
    pub payload: Vec<u8>,
    pub terminator: OscTerminator,
}
