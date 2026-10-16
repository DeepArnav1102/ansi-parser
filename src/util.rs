use crate::{CsiSequence, DcsSequence, EscapeSequence, Event, OscSequence, OscTerminator, Parser};
use std::fmt;

pub fn strip_ansi(input: &[u8]) -> Vec<u8> {
    let mut parser = Parser::new();
    let mut clean = Vec::with_capacity(input.len());

    parser.feed_each(input, |event| match event {
        Event::Print(bytes) => clean.extend(bytes),
        Event::Control(b) if matches!(b, b'\n' | b'\r' | b'\t') => clean.push(b),
        _ => {}
    });

    clean
}

pub fn strip_ansi_to_string(input: &str) -> String {
    let bytes = strip_ansi(input.as_bytes());
    String::from_utf8_lossy(&bytes).into_owned()
}

impl EscapeSequence {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(2 + self.intermediates.len());
        buf.push(0x1B);
        buf.extend_from_slice(&self.intermediates);
        buf.push(self.final_byte);
        buf
    }
}

impl fmt::Display for EscapeSequence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "\x1b{}{}",
            String::from_utf8_lossy(&self.intermediates),
            self.final_byte as char
        )
    }
}

impl CsiSequence {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(3 + self.parameter_bytes.len() + self.intermediates.len());
        buf.push(0x1B);
        buf.push(b'[');
        buf.extend_from_slice(&self.parameter_bytes);
        buf.extend_from_slice(&self.intermediates);
        buf.push(self.final_byte);
        buf
    }
}

impl fmt::Display for CsiSequence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "\x1b[{}{}{}",
            String::from_utf8_lossy(&self.parameter_bytes),
            String::from_utf8_lossy(&self.intermediates),
            self.final_byte as char
        )
    }
}

impl OscSequence {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(4 + self.payload.len());
        buf.push(0x1B);
        buf.push(b']');
        buf.extend_from_slice(&self.payload);
        match self.terminator {
            OscTerminator::Bell => buf.push(0x07),
            OscTerminator::St => {
                buf.push(0x1B);
                buf.push(b'\\');
            }
        }
        buf
    }
}

impl fmt::Display for OscSequence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let term = match self.terminator {
            OscTerminator::Bell => "\x07",
            OscTerminator::St => "\x1b\\",
        };
        write!(f, "\x1b]{}{term}", String::from_utf8_lossy(&self.payload))
    }
}

impl DcsSequence {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(
            4 + self.parameter_bytes.len() + self.intermediates.len() + self.payload.len(),
        );
        buf.push(0x1B);
        buf.push(b'P');
        buf.extend_from_slice(&self.parameter_bytes);
        buf.extend_from_slice(&self.intermediates);
        buf.push(self.final_byte);
        buf.extend_from_slice(&self.payload);
        match self.terminator {
            OscTerminator::Bell => buf.push(0x07),
            OscTerminator::St => {
                buf.push(0x1B);
                buf.push(b'\\');
            }
        }
        buf
    }
}

impl fmt::Display for DcsSequence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let term = match self.terminator {
            OscTerminator::Bell => "\x07",
            OscTerminator::St => "\x1b\\",
        };
        write!(
            f,
            "\x1bP{}{}{}{}{term}",
            String::from_utf8_lossy(&self.parameter_bytes),
            String::from_utf8_lossy(&self.intermediates),
            self.final_byte as char,
            String::from_utf8_lossy(&self.payload)
        )
    }
}

impl Event {
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            Event::Print(bytes) => bytes.clone(),
            Event::Control(b) => vec![*b],
            Event::Escape(esc) => esc.to_bytes(),
            Event::Csi(csi) => csi.to_bytes(),
            Event::Osc(osc) => osc.to_bytes(),
            Event::Dcs(dcs) => dcs.to_bytes(),
        }
    }
}

impl fmt::Display for Event {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Event::Print(bytes) => write!(f, "{}", String::from_utf8_lossy(bytes)),
            Event::Control(b) => write!(f, "{}", *b as char),
            Event::Escape(esc) => write!(f, "{esc}"),
            Event::Csi(csi) => write!(f, "{csi}"),
            Event::Osc(osc) => write!(f, "{osc}"),
            Event::Dcs(dcs) => write!(f, "{dcs}"),
        }
    }
}
