use crate::OscSequence;

/// Target clipboard selection for OSC 52.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ClipboardTarget {
    /// System clipboard ('c').
    Clipboard,
    /// Primary selection buffer ('p').
    Primary,
    /// Select buffer ('s').
    Select,
    /// Specific cut buffer index (0..=7).
    CutBuffer(u8),
    /// Unrecognized selection target character.
    Other(char),
}

impl ClipboardTarget {
    /// Parse selection target code from character.
    pub fn from_char(c: char) -> Self {
        match c {
            'c' => Self::Clipboard,
            'p' => Self::Primary,
            's' => Self::Select,
            '0'..='7' => Self::CutBuffer((c as u8) - b'0'),
            other => Self::Other(other),
        }
    }
}

/// Strongly-typed action representing an interpreted OSC command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OscAction {
    /// OSC 0: Set window title and icon name.
    SetIconAndWindowTitle(String),

    /// OSC 2: Set window title.
    SetWindowTitle(String),

    /// OSC 7: Set shell current working directory (e.g. `file://hostname/path`).
    SetCurrentWorkingDirectory(String),

    /// OSC 8: Open a clickable hyperlink (`\x1b]8;params;url\x1b\`).
    SetHyperlink {
        /// Optional metadata parameters (e.g. `id=my-link`).
        params: String,
        /// The destination URL (e.g. `https://example.com`).
        url: String,
    },

    /// OSC 8: Close / clear the active hyperlink (`\x1b]8;;\x1b\`).
    ClearHyperlink,

    /// OSC 52: System clipboard read or write.
    Clipboard {
        /// Target clipboard selection.
        target: ClipboardTarget,
        /// True if this is a query (`?`), false if it contains data to write.
        query: bool,
        /// Base64-encoded clipboard content, if writing.
        base64_data: Option<String>,
    },

    /// OSC 10: Query default foreground color.
    QueryForegroundColor,

    /// OSC 10: Set default foreground color.
    SetForegroundColor(String),

    /// OSC 11: Query default background color.
    QueryBackgroundColor,

    /// OSC 11: Set default background color.
    SetBackgroundColor(String),

    /// Unrecognized or raw OSC sequence.
    Unknown {
        /// Numeric command code.
        command: u32,
        /// Raw unparsed payload bytes after the first semicolon.
        raw_args: Vec<u8>,
    },

    /// OSC payload was malformed (e.g. non-numeric command prefix).
    Malformed,
}

impl OscSequence {
    /// Interpret the OSC sequence into a strongly-typed [`OscAction`].
    pub fn action(&self) -> OscAction {
        let payload = &self.payload;
        if payload.is_empty() {
            return OscAction::Malformed;
        }

        // Find the delimiter ';' between command number and arguments
        let delim_pos = match payload.iter().position(|&b| b == b';') {
            Some(pos) => pos,
            None => return OscAction::Malformed,
        };

        let cmd_bytes = &payload[..delim_pos];
        let arg_bytes = &payload[delim_pos + 1..];

        // Parse command integer
        let cmd_str = match std::str::from_utf8(cmd_bytes) {
            Ok(s) => s.trim(),
            Err(_) => return OscAction::Malformed,
        };

        let command: u32 = match cmd_str.parse() {
            Ok(num) => num,
            Err(_) => return OscAction::Malformed,
        };

        let arg_str = String::from_utf8_lossy(arg_bytes).into_owned();

        match command {
            0 => OscAction::SetIconAndWindowTitle(arg_str),
            2 => OscAction::SetWindowTitle(arg_str),
            7 => OscAction::SetCurrentWorkingDirectory(arg_str),
            8 => parse_hyperlink(arg_bytes),
            52 => parse_clipboard(arg_bytes),
            10 => {
                if arg_bytes == b"?" {
                    OscAction::QueryForegroundColor
                } else {
                    OscAction::SetForegroundColor(arg_str)
                }
            }
            11 => {
                if arg_bytes == b"?" {
                    OscAction::QueryBackgroundColor
                } else {
                    OscAction::SetBackgroundColor(arg_str)
                }
            }
            _ => OscAction::Unknown {
                command,
                raw_args: arg_bytes.to_vec(),
            },
        }
    }

    pub fn decode_clipboard_data(&self) -> Option<Vec<u8>> {
        match self.action() {
            OscAction::Clipboard {
                query: false,
                base64_data: Some(b64),
                ..
            } => decode_base64(b64.trim().as_bytes()),
            _ => None,
        }
    }
}

fn parse_hyperlink(arg_bytes: &[u8]) -> OscAction {
    if arg_bytes.is_empty() || arg_bytes == b";" {
        return OscAction::ClearHyperlink;
    }

    if let Some(pos) = arg_bytes.iter().position(|&b| b == b';') {
        let params = String::from_utf8_lossy(&arg_bytes[..pos]).into_owned();
        let url = String::from_utf8_lossy(&arg_bytes[pos + 1..]).into_owned();

        if url.is_empty() && params.is_empty() {
            OscAction::ClearHyperlink
        } else {
            OscAction::SetHyperlink { params, url }
        }
    } else {
        OscAction::SetHyperlink {
            params: String::new(),
            url: String::from_utf8_lossy(arg_bytes).into_owned(),
        }
    }
}

fn parse_clipboard(arg_bytes: &[u8]) -> OscAction {
    let mut parts = arg_bytes.splitn(2, |&b| b == b';');
    let target_slice = parts.next().unwrap_or(&[]);
    let data_slice = parts.next().unwrap_or(&[]);

    let target_char = target_slice.first().map(|&b| b as char).unwrap_or('c');
    let target = ClipboardTarget::from_char(target_char);

    if data_slice == b"?" {
        OscAction::Clipboard {
            target,
            query: true,
            base64_data: None,
        }
    } else {
        let b64 = String::from_utf8_lossy(data_slice).into_owned();
        OscAction::Clipboard {
            target,
            query: false,
            base64_data: Some(b64),
        }
    }
}

/// Zero-dependency base64 decoder.
pub fn decode_base64(input: &[u8]) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity((input.len() * 3) / 4);
    let mut buf = 0u32;
    let mut bits = 0;

    for &b in input {
        if b.is_ascii_whitespace() {
            continue;
        }
        if b == b'=' {
            break;
        }
        let val = match b {
            b'A'..=b'Z' => b - b'A',
            b'a'..=b'z' => b - b'a' + 26,
            b'0'..=b'9' => b - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return None,
        } as u32;

        buf = (buf << 6) | val;
        bits += 6;

        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
            buf &= (1 << bits) - 1;
        }
    }

    Some(out)
}
