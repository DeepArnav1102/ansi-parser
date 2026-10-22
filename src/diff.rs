//! Terminal screen diffing, minimal ANSI patch generation, and SVG visual snapshotting.
//!
//! When driving real terminal devices or running visual regression tests on TUIs:
//! - Applications want to compute the minimal delta between two screen frames.
//! - Terminal emulators and SSH streams require minimal escape code bandwidth to update displays.
//! - Documentation and automated tests benefit from vector SVG screenshots and text snapshots.
//!
//! This module provides:
//! - [`CellPatch`]: A single cell modification at `(row, col)`.
//! - [`ScreenDiff`]: The computed difference between two [`Screen`] states.
//! - [`generate_ansi_diff`]: Compiles a minimal ANSI byte sequence to transform screen A into screen B.
//! - [`to_snapshot`]: Render a bordered, numbered text snapshot for test assertions.
//! - [`to_svg`]: Render a full terminal screen into a standalone vector SVG image.

use crate::screen::{Cell, Cursor, Screen};
use crate::style::Style;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

/// A single cell modification at grid coordinates `(row, col)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellPatch {
    /// 0-indexed row position.
    pub row: usize,
    /// 0-indexed column position.
    pub col: usize,
    /// New cell content and style.
    pub cell: Cell,
}

/// The set of differences between two [`Screen`] states.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScreenDiff {
    /// List of modified cells.
    pub patches: Vec<CellPatch>,
    /// Whether the cursor position or visibility changed.
    pub cursor_changed: bool,
    /// The target cursor state.
    pub target_cursor: Cursor,
}

impl ScreenDiff {
    /// Compute the diff between `from` and `to`.
    pub fn compute(from: &Screen, to: &Screen) -> Self {
        let mut patches = Vec::new();
        let from_grid = from.active_grid();
        let to_grid = to.active_grid();

        let rows = from_grid.rows().min(to_grid.rows());
        let cols = from_grid.cols().min(to_grid.cols());

        for r in 0..rows {
            for c in 0..cols {
                let from_cell = from_grid.get(r, c).copied().unwrap_or_default();
                let to_cell = to_grid.get(r, c).copied().unwrap_or_default();
                if from_cell != to_cell {
                    patches.push(CellPatch {
                        row: r,
                        col: c,
                        cell: to_cell,
                    });
                }
            }
        }

        let cursor_changed = from.cursor() != to.cursor();

        Self {
            patches,
            cursor_changed,
            target_cursor: *to.cursor(),
        }
    }

    /// Returns `true` if there are no cell or cursor differences.
    pub fn is_empty(&self) -> bool {
        self.patches.is_empty() && !self.cursor_changed
    }

    /// Number of cell modifications.
    pub fn patch_count(&self) -> usize {
        self.patches.len()
    }
}

/// Generate a bandwidth-optimized ANSI byte sequence to update a terminal from `from` to `to`.
///
/// Groups contiguous changed cells on the same row to eliminate redundant cursor jumps,
/// and only emits SGR style changes when attributes or colors change.
pub fn generate_ansi_diff(from: &Screen, to: &Screen) -> Vec<u8> {
    let diff = ScreenDiff::compute(from, to);
    if diff.is_empty() {
        return Vec::new();
    }

    let mut out = Vec::new();
    let mut current_style = Style::default();
    let mut current_cursor: Option<(usize, usize)> = None;

    for patch in &diff.patches {
        let (r, c) = (patch.row, patch.col);

        // Check if we need to jump the cursor
        let needs_jump = match current_cursor {
            Some((cr, cc)) => cr != r || cc != c,
            None => true,
        };

        if needs_jump {
            // CUP is 1-indexed: \x1b[{row};{col}H
            let jump = format!("\x1b[{};{}H", r + 1, c + 1);
            out.extend_from_slice(jump.as_bytes());
        }

        // Check if style changed
        if patch.cell.style != current_style {
            // Reset and apply new style if needed
            out.extend_from_slice(b"\x1b[0m");
            current_style = patch.cell.style;
            let css_actions = current_style_to_sgr(&current_style);
            out.extend_from_slice(&css_actions);
        }

        // Write character
        let mut char_buf = [0u8; 4];
        let encoded = patch.cell.ch.encode_utf8(&mut char_buf);
        out.extend_from_slice(encoded.as_bytes());

        current_cursor = Some((r, c + 1));
    }

    // Reset style after patches
    if current_style != Style::default() {
        out.extend_from_slice(b"\x1b[0m");
    }

    // Move cursor to final destination
    let target = diff.target_cursor;
    let final_jump = format!("\x1b[{};{}H", target.row + 1, target.col + 1);
    out.extend_from_slice(final_jump.as_bytes());

    // Update cursor visibility if changed
    if diff.cursor_changed {
        if target.visible {
            out.extend_from_slice(b"\x1b[?25h");
        } else {
            out.extend_from_slice(b"\x1b[?25l");
        }
    }

    out
}

fn current_style_to_sgr(style: &Style) -> Vec<u8> {
    let mut codes: Vec<String> = Vec::new();

    if style.bold {
        codes.push("1".to_string());
    }
    if style.dim {
        codes.push("2".to_string());
    }
    if style.italic {
        codes.push("3".to_string());
    }
    if style.underline {
        codes.push("4".to_string());
    }
    if style.inverse {
        codes.push("7".to_string());
    }

    if let Some(fg) = style.foreground {
        match fg {
            crate::Color::Standard(std) => codes.push(format!("{}", 30 + (std as u8))),
            crate::Color::Bright(std) => codes.push(format!("{}", 90 + (std as u8))),
            crate::Color::Indexed(n) => codes.push(format!("38;5;{n}")),
            crate::Color::Rgb(r, g, b) => codes.push(format!("38;2;{r};{g};{b}")),
            crate::Color::Default => codes.push("39".to_string()),
        }
    }

    if let Some(bg) = style.background {
        match bg {
            crate::Color::Standard(std) => codes.push(format!("{}", 40 + (std as u8))),
            crate::Color::Bright(std) => codes.push(format!("{}", 100 + (std as u8))),
            crate::Color::Indexed(n) => codes.push(format!("48;5;{n}")),
            crate::Color::Rgb(r, g, b) => codes.push(format!("48;2;{r};{g};{b}")),
            crate::Color::Default => codes.push("49".to_string()),
        }
    }

    if codes.is_empty() {
        Vec::new()
    } else {
        format!("\x1b[{}m", codes.join(";")).into_bytes()
    }
}

/// Render a clean, bordered, line-numbered snapshot of the terminal screen for test assertions.
pub fn to_snapshot(screen: &Screen) -> String {
    let rows = screen.rows();
    let cols = screen.cols();
    let mut out = String::new();

    let border = format!("+{}+\n", "-".repeat(cols + 2));
    out.push_str(&border);

    for r in 0..rows {
        let line = screen.line_to_string(r);
        let padding = cols.saturating_sub(line.chars().count());
        out.push_str(&format!("| {line}{} |\n", " ".repeat(padding)));
    }

    out.push_str(&border);
    out
}

/// Render the complete virtual terminal screen into an SVG image.
pub fn to_svg(screen: &Screen) -> String {
    let rows = screen.rows();
    let cols = screen.cols();
    let cell_width = 9;
    let cell_height = 18;
    let font_size = 14;

    let width = cols * cell_width + 20;
    let height = rows * cell_height + 20;

    let mut svg = String::new();
    svg.push_str(&format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">
  <rect width="100%" height="100%" fill="#1e1e1e" rx="6"/>
  <style>
    .t {{ font-family: 'Courier New', monospace; font-size: {font_size}px; }}
  </style>
"##
    ));

    let grid = screen.active_grid();

    for r in 0..rows {
        for c in 0..cols {
            if let Some(cell) = grid.get(r, c) {
                let x = c * cell_width + 10;
                let y = r * cell_height + 10;

                // Render background if custom
                if let Some(bg) = cell.style.background {
                    let hex = color_to_hex(bg);
                    svg.push_str(&format!(
                        r#"  <rect x="{x}" y="{y}" width="{cell_width}" height="{cell_height}" fill="{hex}"/>
"#
                    ));
                }

                // Render character
                if cell.ch != ' ' {
                    let fg_hex = cell
                        .style
                        .foreground
                        .map(color_to_hex)
                        .unwrap_or_else(|| "#d4d4d4".to_string());
                    let text_y = y + cell_height - 4;
                    let escaped = match cell.ch {
                        '&' => "&amp;".to_string(),
                        '<' => "&lt;".to_string(),
                        '>' => "&gt;".to_string(),
                        '"' => "&quot;".to_string(),
                        other => other.to_string(),
                    };

                    let weight = if cell.style.bold {
                        " font-weight=\"bold\""
                    } else {
                        ""
                    };
                    let font_style = if cell.style.italic {
                        " font-style=\"italic\""
                    } else {
                        ""
                    };

                    svg.push_str(&format!(
                        r#"  <text x="{x}" y="{text_y}" fill="{fg_hex}" class="t"{weight}{font_style}>{escaped}</text>
"#
                    ));
                }
            }
        }
    }

    // Render cursor if visible
    if screen.cursor().visible {
        let cx = screen.cursor().col * cell_width + 10;
        let cy = screen.cursor().row * cell_height + 10;
        svg.push_str(&format!(
            r##"  <rect x="{cx}" y="{cy}" width="{cell_width}" height="{cell_height}" fill="#ffffff" opacity="0.4"/>
"##
        ));
    }

    svg.push_str("</svg>\n");
    svg
}

fn color_to_hex(color: crate::Color) -> String {
    match color {
        crate::Color::Standard(std) => match std {
            crate::StandardColor::Black => "#000000",
            crate::StandardColor::Red => "#cd0000",
            crate::StandardColor::Green => "#00cd00",
            crate::StandardColor::Yellow => "#cdcd00",
            crate::StandardColor::Blue => "#0000ee",
            crate::StandardColor::Magenta => "#cd00cd",
            crate::StandardColor::Cyan => "#00cdcd",
            crate::StandardColor::White => "#e5e5e5",
        }
        .to_string(),
        crate::Color::Bright(std) => match std {
            crate::StandardColor::Black => "#7f7f7f",
            crate::StandardColor::Red => "#ff0000",
            crate::StandardColor::Green => "#00ff00",
            crate::StandardColor::Yellow => "#ffff00",
            crate::StandardColor::Blue => "#5c5cff",
            crate::StandardColor::Magenta => "#ff00ff",
            crate::StandardColor::Cyan => "#00ffff",
            crate::StandardColor::White => "#ffffff",
        }
        .to_string(),
        crate::Color::Rgb(r, g, b) => format!("#{r:02x}{g:02x}{b:02x}"),
        crate::Color::Indexed(idx) => format!("#{:02x}{:02x}{:02x}", idx, idx, idx),
        crate::Color::Default => "#ffffff".to_string(),
    }
}
