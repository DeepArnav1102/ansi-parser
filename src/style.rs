use crate::csi::{Color, SgrAction, StandardColor};
use crate::{CsiAction, Event, Parser};
use alloc::string::{String, ToString};
use alloc::vec::Vec;

/// Active text rendition styling state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Style {
    /// Active foreground color, if set.
    pub foreground: Option<Color>,
    /// Active background color, if set.
    pub background: Option<Color>,
    /// Active underline color, if set (via `58;...`).
    pub underline_color: Option<Color>,
    /// Bold or increased intensity.
    pub bold: bool,
    /// Dim or faint intensity.
    pub dim: bool,
    /// Italicized text.
    pub italic: bool,
    /// Single underline.
    pub underline: bool,
    /// Double underline.
    pub double_underline: bool,
    /// Slow blink.
    pub slow_blink: bool,
    /// Rapid blink.
    pub rapid_blink: bool,
    /// Inverted (swapped) foreground and background.
    pub inverse: bool,
    /// Concealed / hidden text.
    pub hidden: bool,
    /// Strikethrough / crossed out.
    pub strikethrough: bool,
    /// Overlined text.
    pub overline: bool,
}

impl Style {
    /// Create a new default style (plain text, no colors or attributes).
    pub fn new() -> Self {
        Self::default()
    }

    /// Reset all styles and colors to normal.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Apply a single [`SgrAction`] to mutate this style state.
    pub fn apply_sgr(&mut self, action: SgrAction) {
        match action {
            SgrAction::Reset => self.reset(),
            SgrAction::Bold => self.bold = true,
            SgrAction::Dim => self.dim = true,
            SgrAction::Italic => self.italic = true,
            SgrAction::Underline => self.underline = true,
            SgrAction::DoubleUnderline => self.double_underline = true,
            SgrAction::SlowBlink => self.slow_blink = true,
            SgrAction::RapidBlink => self.rapid_blink = true,
            SgrAction::Inverse => self.inverse = true,
            SgrAction::Hidden => self.hidden = true,
            SgrAction::Strikethrough => self.strikethrough = true,
            SgrAction::Overline => self.overline = true,

            SgrAction::NormalIntensity => {
                self.bold = false;
                self.dim = false;
            }
            SgrAction::NoItalic => self.italic = false,
            SgrAction::NoUnderline => {
                self.underline = false;
                self.double_underline = false;
            }
            SgrAction::NoBlink => {
                self.slow_blink = false;
                self.rapid_blink = false;
            }
            SgrAction::NoInverse => self.inverse = false,
            SgrAction::NoHidden => self.hidden = false,
            SgrAction::NoStrikethrough => self.strikethrough = false,
            SgrAction::NoOverline => self.overline = false,

            SgrAction::SetForegroundColor(Color::Default) => self.foreground = None,
            SgrAction::SetForegroundColor(c) => self.foreground = Some(c),

            SgrAction::SetBackgroundColor(Color::Default) => self.background = None,
            SgrAction::SetBackgroundColor(c) => self.background = Some(c),

            SgrAction::SetUnderlineColor(Color::Default) | SgrAction::DefaultUnderlineColor => {
                self.underline_color = None
            }
            SgrAction::SetUnderlineColor(c) => self.underline_color = Some(c),

            SgrAction::Unknown(_) => {}
        }
    }

    /// Apply a slice of [`SgrAction`] commands in order.
    pub fn apply_sgr_slice(&mut self, actions: &[SgrAction]) {
        for &action in actions {
            self.apply_sgr(action);
        }
    }

    /// Generate an inline CSS style string corresponding to this visual style.
    pub fn to_css_style(&self) -> String {
        let mut css = Vec::new();

        let (fg, bg) = if self.inverse {
            (self.background, self.foreground)
        } else {
            (self.foreground, self.background)
        };

        if let Some(c) = fg {
            css.push(format!("color:{};", color_to_hex(c, false)));
        }
        if let Some(c) = bg {
            css.push(format!("background-color:{};", color_to_hex(c, true)));
        }
        if self.bold {
            css.push("font-weight:bold;".to_string());
        }
        if self.dim {
            css.push("opacity:0.65;".to_string());
        }
        if self.italic {
            css.push("font-style:italic;".to_string());
        }
        if self.underline || self.double_underline {
            let line = if self.double_underline {
                "text-decoration:underline double;"
            } else {
                "text-decoration:underline;"
            };
            css.push(line.to_string());
        }
        if self.strikethrough {
            css.push("text-decoration:line-through;".to_string());
        }
        if self.overline {
            css.push("text-decoration:overline;".to_string());
        }
        if self.hidden {
            css.push("visibility:hidden;".to_string());
        }

        css.join("")
    }
}

/// A contiguous span of text associated with an active visual [`Style`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StyledSpan {
    /// Raw text bytes in this span.
    pub text: Vec<u8>,
    /// Visual style active when this text was emitted.
    pub style: Style,
}

impl StyledSpan {
    /// Convert text bytes to a UTF-8 string lossily.
    pub fn to_string_lossy(&self) -> String {
        String::from_utf8_lossy(&self.text).into_owned()
    }

    /// Convert this span to an HTML `<span>` element with inline CSS styling.
    pub fn to_html(&self) -> String {
        let text_escaped = html_escape(&self.to_string_lossy());
        let css = self.style.to_css_style();

        if css.is_empty() {
            text_escaped
        } else {
            format!("<span style=\"{css}\">{text_escaped}</span>")
        }
    }
}

/// A stateful tracker that processes [`Event`] streams and generates [`StyledSpan`] items.
#[derive(Debug, Clone, Default)]
pub struct StyleTracker {
    current_style: Style,
}

impl StyleTracker {
    /// Create a new style tracker starting with the default style.
    pub fn new() -> Self {
        Self::default()
    }

    /// Reference to the currently active visual style.
    pub fn current_style(&self) -> &Style {
        &self.current_style
    }

    /// Mutable reference to the currently active visual style.
    pub fn current_style_mut(&mut self) -> &mut Style {
        &mut self.current_style
    }

    pub fn process_event(&mut self, event: &Event) -> Option<StyledSpan> {
        match event {
            Event::Print(bytes) => Some(StyledSpan {
                text: bytes.clone(),
                style: self.current_style,
            }),
            Event::Control(b) if matches!(b, b'\n' | b'\r' | b'\t') => Some(StyledSpan {
                text: vec![*b],
                style: self.current_style,
            }),
            Event::Csi(csi) => {
                if let CsiAction::Sgr(actions) = csi.action() {
                    self.current_style.apply_sgr_slice(&actions);
                }
                None
            }
            _ => None,
        }
    }

    /// Feed a byte chunk through an internal parser, tracking styles and emitting spans.
    pub fn feed_to_spans(&mut self, input: &[u8]) -> Vec<StyledSpan> {
        let mut spans: Vec<StyledSpan> = Vec::new();
        let mut parser = Parser::new();

        parser.feed_each(input, |event| {
            if let Some(span) = self.process_event(&event) {
                // Merge adjacent spans with identical styles
                if let Some(last) = spans.last_mut()
                    && last.style == span.style
                {
                    last.text.extend(span.text);
                    return;
                }
                spans.push(span);
            }
        });

        spans
    }
}

/// Convert a byte slice into a sequence of [`StyledSpan`] items with consolidated styles.
pub fn to_styled_spans(input: &[u8]) -> Vec<StyledSpan> {
    let mut tracker = StyleTracker::new();
    tracker.feed_to_spans(input)
}

/// Convert an ANSI formatted string or byte slice into an HTML string with inline CSS.
pub fn to_html(input: &[u8]) -> String {
    let spans = to_styled_spans(input);
    let mut html = String::new();

    for span in spans {
        html.push_str(&span.to_html());
    }

    html
}

fn html_escape(input: &str) -> String {
    let mut escaped = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            _ => escaped.push(c),
        }
    }
    escaped
}

fn color_to_hex(color: Color, is_bg: bool) -> String {
    match color {
        Color::Standard(std) => standard_to_hex(std, false, is_bg),
        Color::Bright(std) => standard_to_hex(std, true, is_bg),
        Color::Indexed(idx) => indexed_to_hex(idx),
        Color::Rgb(r, g, b) => format!("#{r:02x}{g:02x}{b:02x}"),
        Color::Default => if is_bg { "#000000" } else { "#ffffff" }.to_string(),
    }
}

fn standard_to_hex(std: StandardColor, bright: bool, _is_bg: bool) -> String {
    let hex = match (std, bright) {
        (StandardColor::Black, false) => "#000000",
        (StandardColor::Black, true) => "#7f7f7f",
        (StandardColor::Red, false) => "#cd0000",
        (StandardColor::Red, true) => "#ff0000",
        (StandardColor::Green, false) => "#00cd00",
        (StandardColor::Green, true) => "#00ff00",
        (StandardColor::Yellow, false) => "#cdcd00",
        (StandardColor::Yellow, true) => "#ffff00",
        (StandardColor::Blue, false) => "#0000ee",
        (StandardColor::Blue, true) => "#5c5cff",
        (StandardColor::Magenta, false) => "#cd00cd",
        (StandardColor::Magenta, true) => "#ff00ff",
        (StandardColor::Cyan, false) => "#00cdcd",
        (StandardColor::Cyan, true) => "#00ffff",
        (StandardColor::White, false) => "#e5e5e5",
        (StandardColor::White, true) => "#ffffff",
    };
    hex.to_string()
}

fn indexed_to_hex(idx: u8) -> String {
    if idx < 8 {
        standard_to_hex(
            StandardColor::from_index(idx).unwrap_or(StandardColor::Black),
            false,
            false,
        )
    } else if idx < 16 {
        standard_to_hex(
            StandardColor::from_index(idx - 8).unwrap_or(StandardColor::Black),
            true,
            false,
        )
    } else if idx < 232 {
        let id = idx - 16;
        let r = (id / 36) % 6;
        let g = (id / 6) % 6;
        let b = id % 6;
        let conv = |c: u8| if c == 0 { 0 } else { 55 + c * 40 };
        format!("#{:02x}{:02x}{:02x}", conv(r), conv(g), conv(b))
    } else {
        let gray = 8 + (idx - 232) * 10;
        format!("#{gray:02x}{gray:02x}{gray:02x}")
    }
}
