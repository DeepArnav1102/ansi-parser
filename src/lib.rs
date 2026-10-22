#![no_std]

#[cfg(feature = "alloc")]
#[macro_use]
extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

pub mod csi;
pub mod diff;
pub mod input;
pub mod osc;
mod parser;
pub mod screen;
#[cfg(feature = "std")]
pub mod stream;
pub mod style;
pub mod util;

pub use csi::{Color, CsiAction, EraseDisplayMode, EraseLineMode, SgrAction, StandardColor};
pub use diff::{generate_ansi_diff, to_snapshot, to_svg, CellPatch, ScreenDiff};
pub use input::{
    decode_input_sequence, encode_key, encode_mouse, encode_paste, InputEvent, KeyCode, Modifiers,
    MouseAction, MouseButton, MouseEvent, MouseProtocol,
};
pub use osc::{ClipboardTarget, OscAction};
pub use parser::Parser;
pub use screen::{Cell, Cursor, Screen, TerminalGrid};
#[cfg(feature = "std")]
pub use stream::{AnsiReader, AnsiSpanReader, ParserIter};
pub use style::{to_html, to_styled_spans, Style, StyleTracker, StyledSpan};
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
