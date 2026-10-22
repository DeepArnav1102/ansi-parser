use crate::CsiSequence;
use alloc::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StandardColor {
    Black = 0,
    Red = 1,
    Green = 2,
    Yellow = 3,
    Blue = 4,
    Magenta = 5,
    Cyan = 6,
    White = 7,
}

impl StandardColor {
    pub fn from_index(index: u8) -> Option<Self> {
        match index {
            0 => Some(Self::Black),
            1 => Some(Self::Red),
            2 => Some(Self::Green),
            3 => Some(Self::Yellow),
            4 => Some(Self::Blue),
            5 => Some(Self::Magenta),
            6 => Some(Self::Cyan),
            7 => Some(Self::White),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Color {
    Standard(StandardColor),
    Bright(StandardColor),
    Indexed(u8),
    Rgb(u8, u8, u8),
    Default,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SgrAction {
    Reset,
    Bold,
    Dim,
    Italic,
    Underline,
    DoubleUnderline,
    SlowBlink,
    RapidBlink,
    Inverse,
    Hidden,
    Strikethrough,
    NormalIntensity,
    NoItalic,
    NoUnderline,
    NoBlink,
    NoInverse,
    NoHidden,
    NoStrikethrough,
    Overline,
    NoOverline,
    SetForegroundColor(Color),
    SetBackgroundColor(Color),
    SetUnderlineColor(Color),
    DefaultUnderlineColor,
    Unknown(u16),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EraseDisplayMode {
    ToEnd,
    ToStart,
    All,
    Scrollback,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EraseLineMode {
    ToEnd,
    ToStart,
    All,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CsiAction {
    Sgr(Vec<SgrAction>),
    CursorUp(u16),
    CursorDown(u16),
    CursorForward(u16),
    CursorBack(u16),
    CursorNextLine(u16),
    CursorPreviousLine(u16),
    CursorHorizontalAbsolute(u16),
    CursorPosition {
        row: u16,
        col: u16,
    },
    EraseInDisplay(EraseDisplayMode),
    EraseInLine(EraseLineMode),
    ScrollUp(u16),
    ScrollDown(u16),
    SaveCursorPosition,
    RestoreCursorPosition,
    SetDecPrivateMode(u16),
    ResetDecPrivateMode(u16),
    Unknown {
        final_byte: u8,
        is_private: bool,
        params: Vec<u16>,
        intermediates: Vec<u8>,
    },
}

impl CsiSequence {
    pub fn parse_numeric_params(&self) -> (bool, Vec<Option<u16>>) {
        let bytes = &self.parameter_bytes;
        if bytes.is_empty() {
            return (false, Vec::new());
        }

        let (is_private, slice) = if bytes[0] == b'?' {
            (true, &bytes[1..])
        } else {
            (false, &bytes[..])
        };

        if slice.is_empty() {
            return (is_private, Vec::new());
        }

        let mut params = Vec::new();
        let mut current: Option<u16> = None;

        for &b in slice {
            match b {
                b'0'..=b'9' => {
                    let digit = (b - b'0') as u16;
                    let val = current.unwrap_or(0);
                    current = Some(val.saturating_mul(10).saturating_add(digit));
                }
                b';' | b':' => {
                    params.push(current);
                    current = None;
                }
                _ => {}
            }
        }
        params.push(current);

        (is_private, params)
    }

    pub fn parse_sgr(&self) -> Vec<SgrAction> {
        if self.final_byte != b'm' {
            return Vec::new();
        }

        let (_, params) = self.parse_numeric_params();
        if params.is_empty() {
            return vec![SgrAction::Reset];
        }

        let mut actions = Vec::new();
        let mut i = 0;

        while i < params.len() {
            let code = params[i].unwrap_or(0);
            i += 1;

            match code {
                0 => actions.push(SgrAction::Reset),
                1 => actions.push(SgrAction::Bold),
                2 => actions.push(SgrAction::Dim),
                3 => actions.push(SgrAction::Italic),
                4 => actions.push(SgrAction::Underline),
                5 => actions.push(SgrAction::SlowBlink),
                6 => actions.push(SgrAction::RapidBlink),
                7 => actions.push(SgrAction::Inverse),
                8 => actions.push(SgrAction::Hidden),
                9 => actions.push(SgrAction::Strikethrough),
                21 => actions.push(SgrAction::DoubleUnderline),
                22 => actions.push(SgrAction::NormalIntensity),
                23 => actions.push(SgrAction::NoItalic),
                24 => actions.push(SgrAction::NoUnderline),
                25 => actions.push(SgrAction::NoBlink),
                27 => actions.push(SgrAction::NoInverse),
                28 => actions.push(SgrAction::NoHidden),
                29 => actions.push(SgrAction::NoStrikethrough),
                53 => actions.push(SgrAction::Overline),
                55 => actions.push(SgrAction::NoOverline),

                30..=37 => {
                    if let Some(c) = StandardColor::from_index((code - 30) as u8) {
                        actions.push(SgrAction::SetForegroundColor(Color::Standard(c)));
                    }
                }
                38 => {
                    if let Some((color, consumed)) = parse_extended_color(&params[i..]) {
                        actions.push(SgrAction::SetForegroundColor(color));
                        i += consumed;
                    }
                }
                39 => actions.push(SgrAction::SetForegroundColor(Color::Default)),

                40..=47 => {
                    if let Some(c) = StandardColor::from_index((code - 40) as u8) {
                        actions.push(SgrAction::SetBackgroundColor(Color::Standard(c)));
                    }
                }
                48 => {
                    if let Some((color, consumed)) = parse_extended_color(&params[i..]) {
                        actions.push(SgrAction::SetBackgroundColor(color));
                        i += consumed;
                    }
                }
                49 => actions.push(SgrAction::SetBackgroundColor(Color::Default)),

                58 => {
                    if let Some((color, consumed)) = parse_extended_color(&params[i..]) {
                        actions.push(SgrAction::SetUnderlineColor(color));
                        i += consumed;
                    }
                }
                59 => actions.push(SgrAction::DefaultUnderlineColor),

                90..=97 => {
                    if let Some(c) = StandardColor::from_index((code - 90) as u8) {
                        actions.push(SgrAction::SetForegroundColor(Color::Bright(c)));
                    }
                }
                100..=107 => {
                    if let Some(c) = StandardColor::from_index((code - 100) as u8) {
                        actions.push(SgrAction::SetBackgroundColor(Color::Bright(c)));
                    }
                }

                other => actions.push(SgrAction::Unknown(other)),
            }
        }

        actions
    }

    pub fn action(&self) -> CsiAction {
        let (is_private, params) = self.parse_numeric_params();

        if is_private {
            let mode = params.first().copied().flatten().unwrap_or(0);
            return match self.final_byte {
                b'h' => CsiAction::SetDecPrivateMode(mode),
                b'l' => CsiAction::ResetDecPrivateMode(mode),
                _ => CsiAction::Unknown {
                    final_byte: self.final_byte,
                    is_private: true,
                    params: params.into_iter().map(|p| p.unwrap_or(0)).collect(),
                    intermediates: self.intermediates.clone(),
                },
            };
        }

        let first = params.first().copied().flatten().unwrap_or(1);
        let second = params.get(1).copied().flatten().unwrap_or(1);

        match self.final_byte {
            b'm' => CsiAction::Sgr(self.parse_sgr()),
            b'A' => CsiAction::CursorUp(first),
            b'B' => CsiAction::CursorDown(first),
            b'C' => CsiAction::CursorForward(first),
            b'D' => CsiAction::CursorBack(first),
            b'E' => CsiAction::CursorNextLine(first),
            b'F' => CsiAction::CursorPreviousLine(first),
            b'G' => CsiAction::CursorHorizontalAbsolute(first),
            b'H' | b'f' => CsiAction::CursorPosition {
                row: first,
                col: second,
            },
            b'J' => {
                let mode_code = params.first().copied().flatten().unwrap_or(0);
                let mode = match mode_code {
                    1 => EraseDisplayMode::ToStart,
                    2 => EraseDisplayMode::All,
                    3 => EraseDisplayMode::Scrollback,
                    _ => EraseDisplayMode::ToEnd,
                };
                CsiAction::EraseInDisplay(mode)
            }
            b'K' => {
                let mode_code = params.first().copied().flatten().unwrap_or(0);
                let mode = match mode_code {
                    1 => EraseLineMode::ToStart,
                    2 => EraseLineMode::All,
                    _ => EraseLineMode::ToEnd,
                };
                CsiAction::EraseInLine(mode)
            }
            b'S' => CsiAction::ScrollUp(first),
            b'T' => CsiAction::ScrollDown(first),
            b's' => CsiAction::SaveCursorPosition,
            b'u' => CsiAction::RestoreCursorPosition,
            _ => CsiAction::Unknown {
                final_byte: self.final_byte,
                is_private: false,
                params: params.into_iter().map(|p| p.unwrap_or(0)).collect(),
                intermediates: self.intermediates.clone(),
            },
        }
    }
}

fn parse_extended_color(sub_params: &[Option<u16>]) -> Option<(Color, usize)> {
    if sub_params.is_empty() {
        return None;
    }

    match sub_params[0].unwrap_or(0) {
        5 => {
            if sub_params.len() >= 2 {
                let index = (sub_params[1].unwrap_or(0) & 0xFF) as u8;
                Some((Color::Indexed(index), 2))
            } else {
                None
            }
        }
        2 => {
            if sub_params.len() >= 5 && sub_params.get(1) == Some(&None) {
                let r = (sub_params[2].unwrap_or(0) & 0xFF) as u8;
                let g = (sub_params[3].unwrap_or(0) & 0xFF) as u8;
                let b = (sub_params[4].unwrap_or(0) & 0xFF) as u8;
                Some((Color::Rgb(r, g, b), 5))
            } else if sub_params.len() >= 4 {
                let r = (sub_params[1].unwrap_or(0) & 0xFF) as u8;
                let g = (sub_params[2].unwrap_or(0) & 0xFF) as u8;
                let b = (sub_params[3].unwrap_or(0) & 0xFF) as u8;
                Some((Color::Rgb(r, g, b), 4))
            } else {
                None
            }
        }
        _ => None,
    }
}
