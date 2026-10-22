//! Interactive terminal input sequence protocol: keyboard, mouse, and paste encoding/decoding.
//!
//! Terminal emulators communicate user actions (keystrokes, mouse clicks/drags, paste brackets,
//! and focus events) back to host applications via specialized escape sequences.
//!
//! This module provides:
//! - Strongly typed [`KeyCode`], [`Modifiers`], [`MouseEvent`], and [`InputEvent`].
//! - [`encode_key`]: Convert key presses and modifier combinations into standard xterm ANSI bytes.
//! - [`encode_mouse`]: Convert mouse events into SGR (1006) or Normal mouse protocol bytes.
//! - [`encode_paste`]: Wrap text in bracketed paste mode (`\x1b[200~` ... `\x1b[201~`).
//! - [`decode_input_sequence`]: Parse raw bytes received from terminal standard input into [`InputEvent`].

use alloc::string::String;
use alloc::vec::Vec;

/// Key code representing keyboard keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyCode {
    /// Printable character or symbol.
    Char(char),
    /// Enter / Return (`\r` or `\n`).
    Enter,
    /// Backspace (`0x7F` or `0x08`).
    Backspace,
    /// Tab (`\t` or `0x09`).
    Tab,
    /// BackTab / Shift+Tab (`\x1b[Z`).
    BackTab,
    /// Escape key (`\x1b`).
    Esc,
    /// Arrow Up (`\x1b[A`).
    Up,
    /// Arrow Down (`\x1b[B`).
    Down,
    /// Arrow Left (`\x1b[D`).
    Left,
    /// Arrow Right (`\x1b[C`).
    Right,
    /// Home (`\x1b[H` or `\x1b[1~`).
    Home,
    /// End (`\x1b[F` or `\x1b[4~`).
    End,
    /// Page Up (`\x1b[5~`).
    PageUp,
    /// Page Down (`\x1b[6~`).
    PageDown,
    /// Insert (`\x1b[2~`).
    Insert,
    /// Delete (`\x1b[3~`).
    Delete,
    /// Function key F1..=F12.
    F(u8),
    /// Null key byte (`0x00`).
    Null,
}

/// Modifier keys (Shift, Alt, Ctrl) active during an input event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Modifiers {
    /// Shift key held.
    pub shift: bool,
    /// Alt / Option key held.
    pub alt: bool,
    /// Control key held.
    pub ctrl: bool,
}

impl Modifiers {
    /// Create empty modifiers (no keys held).
    pub const NONE: Self = Self {
        shift: false,
        alt: false,
        ctrl: false,
    };

    /// Shift modifier held.
    pub const SHIFT: Self = Self {
        shift: true,
        alt: false,
        ctrl: false,
    };

    /// Alt modifier held.
    pub const ALT: Self = Self {
        shift: false,
        alt: true,
        ctrl: false,
    };

    /// Ctrl modifier held.
    pub const CTRL: Self = Self {
        shift: false,
        alt: false,
        ctrl: true,
    };

    /// Convert modifier combo to xterm numeric parameter:
    /// 1 + (shift ? 1 : 0) + (alt ? 2 : 0) + (ctrl ? 4 : 0).
    pub fn to_xterm_param(self) -> Option<u8> {
        let mut code = 1;
        if self.shift {
            code += 1;
        }
        if self.alt {
            code += 2;
        }
        if self.ctrl {
            code += 4;
        }
        if code > 1 { Some(code) } else { None }
    }

    /// Parse modifier flags from an xterm modifier parameter (2..=8).
    pub fn from_xterm_param(param: u8) -> Self {
        if param < 2 {
            return Self::NONE;
        }
        let code = param - 1;
        Self {
            shift: (code & 1) != 0,
            alt: (code & 2) != 0,
            ctrl: (code & 4) != 0,
        }
    }
}

/// Mouse button identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Middle,
    Right,
    WheelUp,
    WheelDown,
    None,
}

/// Mouse action types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseAction {
    Press,
    Release,
    Drag,
    Move,
}

/// Supported mouse escape sequence reporting protocols.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseProtocol {
    /// Modern SGR 1006 protocol: `\x1b[<{button};{col};{row}M` / `m`.
    Sgr,
    /// Legacy X10 / Normal protocol: `\x1b[M{b+32}{x+32}{y+32}`.
    Normal,
}

/// A mouse event with 1-indexed coordinates and modifier state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MouseEvent {
    /// 1-indexed column position.
    pub col: usize,
    /// 1-indexed row position.
    pub row: usize,
    /// Mouse button involved.
    pub button: MouseButton,
    /// Action (press, release, drag, move).
    pub action: MouseAction,
    /// Active modifier keys during the event.
    pub modifiers: Modifiers,
}

/// High-level event representing user terminal input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputEvent {
    /// Keystroke event.
    Key { key: KeyCode, modifiers: Modifiers },
    /// Mouse event.
    Mouse(MouseEvent),
    /// Bracketed paste payload.
    Paste(String),
    /// Terminal gained focus (`\x1b[I`).
    FocusGained,
    /// Terminal lost focus (`\x1b[O`).
    FocusLost,
}

/// Encode a keyboard key and modifier combination into standard terminal escape bytes.
pub fn encode_key(key: KeyCode, modifiers: Modifiers) -> Vec<u8> {
    if modifiers.ctrl
        && !modifiers.alt
        && !modifiers.shift
        && let KeyCode::Char(c) = key
        && c.is_ascii_alphabetic()
    {
        let code = (c.to_ascii_uppercase() as u8) - b'A' + 1;
        return vec![code];
    }

    let mod_param = modifiers.to_xterm_param();

    match key {
        KeyCode::Char(c) => {
            let mut buf = Vec::new();
            if modifiers.alt {
                buf.push(0x1B);
            }
            let mut char_bytes = [0u8; 4];
            let encoded = c.encode_utf8(&mut char_bytes);
            buf.extend_from_slice(encoded.as_bytes());
            buf
        }
        KeyCode::Enter => {
            if modifiers.alt {
                vec![0x1B, b'\r']
            } else {
                vec![b'\r']
            }
        }
        KeyCode::Backspace => {
            if modifiers.alt {
                vec![0x1B, 0x7F]
            } else {
                vec![0x7F]
            }
        }
        KeyCode::Tab => {
            if modifiers.alt {
                vec![0x1B, b'\t']
            } else {
                vec![b'\t']
            }
        }
        KeyCode::BackTab => vec![0x1B, b'[', b'Z'],
        KeyCode::Esc => vec![0x1B],
        KeyCode::Null => vec![0x00],

        KeyCode::Up => encode_cursor_key(b'A', mod_param),
        KeyCode::Down => encode_cursor_key(b'B', mod_param),
        KeyCode::Right => encode_cursor_key(b'C', mod_param),
        KeyCode::Left => encode_cursor_key(b'D', mod_param),
        KeyCode::Home => encode_cursor_key(b'H', mod_param),
        KeyCode::End => encode_cursor_key(b'F', mod_param),

        KeyCode::Insert => encode_tilde_key(2, mod_param),
        KeyCode::Delete => encode_tilde_key(3, mod_param),
        KeyCode::PageUp => encode_tilde_key(5, mod_param),
        KeyCode::PageDown => encode_tilde_key(6, mod_param),

        KeyCode::F(n) => encode_f_key(n, mod_param),
    }
}

fn encode_cursor_key(final_byte: u8, mod_param: Option<u8>) -> Vec<u8> {
    if let Some(m) = mod_param {
        format!("\x1b[1;{m}{}", final_byte as char).into_bytes()
    } else {
        vec![0x1B, b'[', final_byte]
    }
}

fn encode_tilde_key(num: u8, mod_param: Option<u8>) -> Vec<u8> {
    if let Some(m) = mod_param {
        format!("\x1b[{num};{m}~").into_bytes()
    } else {
        format!("\x1b[{num}~").into_bytes()
    }
}

fn encode_f_key(n: u8, mod_param: Option<u8>) -> Vec<u8> {
    match n {
        1..=4 => {
            let final_byte = (b'P' + (n - 1)) as char;
            if let Some(m) = mod_param {
                format!("\x1b[1;{m}{final_byte}").into_bytes()
            } else {
                format!("\x1bO{final_byte}").into_bytes()
            }
        }
        5 => encode_tilde_key(15, mod_param),
        6 => encode_tilde_key(17, mod_param),
        7 => encode_tilde_key(18, mod_param),
        8 => encode_tilde_key(19, mod_param),
        9 => encode_tilde_key(20, mod_param),
        10 => encode_tilde_key(21, mod_param),
        11 => encode_tilde_key(23, mod_param),
        12 => encode_tilde_key(24, mod_param),
        _ => Vec::new(),
    }
}

/// Encode a mouse event into bytes using the specified protocol.
pub fn encode_mouse(event: MouseEvent, protocol: MouseProtocol) -> Vec<u8> {
    let mut code: u8 = match event.button {
        MouseButton::Left => 0,
        MouseButton::Middle => 1,
        MouseButton::Right => 2,
        MouseButton::None => 3,
        MouseButton::WheelUp => 64,
        MouseButton::WheelDown => 65,
    };

    if event.modifiers.shift {
        code |= 4;
    }
    if event.modifiers.alt {
        code |= 8;
    }
    if event.modifiers.ctrl {
        code |= 16;
    }
    if event.action == MouseAction::Drag {
        code |= 32;
    }

    match protocol {
        MouseProtocol::Sgr => {
            let suffix = if event.action == MouseAction::Release {
                'm'
            } else {
                'M'
            };
            format!("\x1b[<{code};{};{}{suffix}", event.col, event.row).into_bytes()
        }
        MouseProtocol::Normal => {
            let b = code.min(223) + 32;
            let x = ((event.col.min(223)) as u8) + 32;
            let y = ((event.row.min(223)) as u8) + 32;
            vec![0x1B, b'[', b'M', b, x, y]
        }
    }
}

/// Wrap a string payload in bracketed paste mode escape sequences.
pub fn encode_paste(text: &str) -> Vec<u8> {
    let mut buf = Vec::with_capacity(text.len() + 12);
    buf.extend_from_slice(b"\x1b[200~");
    buf.extend_from_slice(text.as_bytes());
    buf.extend_from_slice(b"\x1b[201~");
    buf
}

/// Parse a raw terminal input byte stream, returning the first decoded [`InputEvent`]
/// along with the number of bytes consumed.
pub fn decode_input_sequence(bytes: &[u8]) -> Option<(InputEvent, usize)> {
    if bytes.is_empty() {
        return None;
    }

    // Check for escape sequences
    if bytes[0] == 0x1B {
        if bytes.len() == 1 {
            return Some((
                InputEvent::Key {
                    key: KeyCode::Esc,
                    modifiers: Modifiers::NONE,
                },
                1,
            ));
        }

        // Focus reporting: \x1b[I (in) / \x1b[O (out)
        if bytes.starts_with(b"\x1b[I") {
            return Some((InputEvent::FocusGained, 3));
        }
        if bytes.starts_with(b"\x1b[O") {
            return Some((InputEvent::FocusLost, 3));
        }

        // Bracketed paste: \x1b[200~ ... \x1b[201~
        if bytes.starts_with(b"\x1b[200~") {
            const END: &[u8] = b"\x1b[201~";
            let pos = bytes.windows(END.len()).position(|w| w == END)?;
            let payload = &bytes[6..pos];
            let text = String::from_utf8_lossy(payload).into_owned();
            return Some((InputEvent::Paste(text), pos + END.len()));
        }

        // SGR Mouse: \x1b[< {code} ; {col} ; {row} (M|m)
        if bytes.starts_with(b"\x1b[<")
            && let Some(end_idx) = bytes[3..]
                .iter()
                .position(|&b| b == b'M' || b == b'm')
                .map(|i| i + 3)
        {
            let is_release = bytes[end_idx] == b'm';
            let slice = &bytes[3..end_idx];
            if let Ok(s) = core::str::from_utf8(slice) {
                let parts: Vec<&str> = s.split(';').collect();
                if parts.len() == 3
                    && let (Ok(code), Ok(col), Ok(row)) = (
                        parts[0].parse::<u8>(),
                        parts[1].parse::<usize>(),
                        parts[2].parse::<usize>(),
                    )
                {
                    let shift = (code & 4) != 0;
                    let alt = (code & 8) != 0;
                    let ctrl = (code & 16) != 0;
                    let is_drag = (code & 32) != 0;

                    let base_code = code & !0b0011_1100;
                    let button = match base_code {
                        0 => MouseButton::Left,
                        1 => MouseButton::Middle,
                        2 => MouseButton::Right,
                        3 => MouseButton::None,
                        64 => MouseButton::WheelUp,
                        65 => MouseButton::WheelDown,
                        _ => MouseButton::None,
                    };

                    let action = if is_release {
                        MouseAction::Release
                    } else if is_drag {
                        MouseAction::Drag
                    } else {
                        MouseAction::Press
                    };

                    return Some((
                        InputEvent::Mouse(MouseEvent {
                            col,
                            row,
                            button,
                            action,
                            modifiers: Modifiers { shift, alt, ctrl },
                        }),
                        end_idx + 1,
                    ));
                }
            }
        }

        // BackTab: \x1b[Z
        if bytes.starts_with(b"\x1b[Z") {
            return Some((
                InputEvent::Key {
                    key: KeyCode::BackTab,
                    modifiers: Modifiers::SHIFT,
                },
                3,
            ));
        }

        // Arrow and home/end keys: \x1b[A, \x1b[B, \x1b[C, \x1b[D, \x1b[H, \x1b[F
        if bytes.len() >= 3 && bytes[1] == b'[' {
            let ch = bytes[2];
            let key = match ch {
                b'A' => Some(KeyCode::Up),
                b'B' => Some(KeyCode::Down),
                b'C' => Some(KeyCode::Right),
                b'D' => Some(KeyCode::Left),
                b'H' => Some(KeyCode::Home),
                b'F' => Some(KeyCode::End),
                _ => None,
            };
            if let Some(k) = key {
                return Some((
                    InputEvent::Key {
                        key: k,
                        modifiers: Modifiers::NONE,
                    },
                    3,
                ));
            }
        }

        // Modified cursor keys: \x1b[1;{m}{A-D,H,F}
        if bytes.len() >= 6 && bytes.starts_with(b"\x1b[1;") {
            let m = bytes[4] - b'0';
            let ch = bytes[5];
            let key = match ch {
                b'A' => Some(KeyCode::Up),
                b'B' => Some(KeyCode::Down),
                b'C' => Some(KeyCode::Right),
                b'D' => Some(KeyCode::Left),
                b'H' => Some(KeyCode::Home),
                b'F' => Some(KeyCode::End),
                _ => None,
            };
            if let Some(k) = key {
                return Some((
                    InputEvent::Key {
                        key: k,
                        modifiers: Modifiers::from_xterm_param(m),
                    },
                    6,
                ));
            }
        }

        // Tilde keys: \x1b[{num}~ or \x1b[{num};{m}~
        if bytes.len() >= 4
            && bytes[1] == b'['
            && let Some(pos) = bytes[2..].iter().position(|&b| b == b'~').map(|p| p + 2)
        {
            let slice = &bytes[2..pos];
            if let Ok(s) = core::str::from_utf8(slice) {
                let mut parts = s.split(';');
                if let Some(first) = parts.next()
                    && let Ok(num) = first.parse::<u8>()
                {
                    let modifiers = if let Some(m_str) = parts.next() {
                        m_str
                            .parse::<u8>()
                            .map(Modifiers::from_xterm_param)
                            .unwrap_or(Modifiers::NONE)
                    } else {
                        Modifiers::NONE
                    };

                    let key = match num {
                        2 => Some(KeyCode::Insert),
                        3 => Some(KeyCode::Delete),
                        5 => Some(KeyCode::PageUp),
                        6 => Some(KeyCode::PageDown),
                        11 => Some(KeyCode::F(1)),
                        12 => Some(KeyCode::F(2)),
                        13 => Some(KeyCode::F(3)),
                        14 => Some(KeyCode::F(4)),
                        15 => Some(KeyCode::F(5)),
                        17 => Some(KeyCode::F(6)),
                        18 => Some(KeyCode::F(7)),
                        19 => Some(KeyCode::F(8)),
                        20 => Some(KeyCode::F(9)),
                        21 => Some(KeyCode::F(10)),
                        23 => Some(KeyCode::F(11)),
                        24 => Some(KeyCode::F(12)),
                        _ => None,
                    };

                    if let Some(k) = key {
                        return Some((InputEvent::Key { key: k, modifiers }, pos + 1));
                    }
                }
            }
        }

        // Function keys F1..=F4 SS3 format: \x1bOP..\x1bOS
        if bytes.len() >= 3 && bytes.starts_with(b"\x1bO") {
            let key = match bytes[2] {
                b'P' => Some(KeyCode::F(1)),
                b'Q' => Some(KeyCode::F(2)),
                b'R' => Some(KeyCode::F(3)),
                b'S' => Some(KeyCode::F(4)),
                _ => None,
            };
            if let Some(k) = key {
                return Some((
                    InputEvent::Key {
                        key: k,
                        modifiers: Modifiers::NONE,
                    },
                    3,
                ));
            }
        }

        // Alt + key: \x1b{ch}
        if bytes.len() >= 2 {
            let next = bytes[1];
            if next.is_ascii() && !next.is_ascii_control() {
                return Some((
                    InputEvent::Key {
                        key: KeyCode::Char(next as char),
                        modifiers: Modifiers::ALT,
                    },
                    2,
                ));
            }
        }
    }

    // Single-byte controls and characters
    let first = bytes[0];
    match first {
        0x00 => Some((
            InputEvent::Key {
                key: KeyCode::Null,
                modifiers: Modifiers::CTRL,
            },
            1,
        )),
        0x08 | 0x7F => Some((
            InputEvent::Key {
                key: KeyCode::Backspace,
                modifiers: Modifiers::NONE,
            },
            1,
        )),
        0x09 => Some((
            InputEvent::Key {
                key: KeyCode::Tab,
                modifiers: Modifiers::NONE,
            },
            1,
        )),
        0x0D | 0x0A => Some((
            InputEvent::Key {
                key: KeyCode::Enter,
                modifiers: Modifiers::NONE,
            },
            1,
        )),
        // Ctrl+A ..= Ctrl+Z (0x01..=0x1A)
        0x01..=0x1A => {
            let c = (b'a' + (first - 1)) as char;
            Some((
                InputEvent::Key {
                    key: KeyCode::Char(c),
                    modifiers: Modifiers::CTRL,
                },
                1,
            ))
        }
        _ => {
            // Parse UTF-8 character
            let s = core::str::from_utf8(bytes).ok()?;
            let ch = s.chars().next()?;
            let len = ch.len_utf8();
            Some((
                InputEvent::Key {
                    key: KeyCode::Char(ch),
                    modifiers: Modifiers::NONE,
                },
                len,
            ))
        }
    }
}
