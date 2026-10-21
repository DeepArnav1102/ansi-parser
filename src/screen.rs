//! Headless virtual terminal screen, 2D character grid buffer, and cursor engine.
//!
//! When testing CLI applications, TUIs, or terminal multiplexers, applications need to know
//! the actual visual state of the terminal display (characters and colors at every row/column).
//!
//! This module provides:
//! - [`Cell`]: A single character cell paired with its visual [`Style`].
//! - [`Cursor`]: Cursor coordinates (0-indexed) and visibility state.
//! - [`TerminalGrid`]: A 2D matrix of character cells with scrolling and line clearing.
//! - [`Screen`]: A complete headless virtual terminal managing main and alternate screen buffers,
//!   cursor state, titles, and processing [`Event`] items directly from the parser.

use crate::csi::{CsiAction, EraseDisplayMode, EraseLineMode};
use crate::style::Style;
use crate::{Event, Parser};

/// A single character cell in the terminal grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Cell {
    /// Character displayed in this cell.
    pub ch: char,
    /// Visual style applied to this cell.
    pub style: Style,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            ch: ' ',
            style: Style::default(),
        }
    }
}

impl Cell {
    /// Create a new cell with the specified character and style.
    pub fn new(ch: char, style: Style) -> Self {
        Self { ch, style }
    }
}

/// Cursor position and visibility state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cursor {
    /// 0-indexed row position.
    pub row: usize,
    /// 0-indexed column position.
    pub col: usize,
    /// Whether the cursor is currently visible.
    pub visible: bool,
}

impl Default for Cursor {
    fn default() -> Self {
        Self {
            row: 0,
            col: 0,
            visible: true,
        }
    }
}

/// A 2D grid matrix of [`Cell`] items representing a terminal display screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalGrid {
    rows: usize,
    cols: usize,
    cells: Vec<Cell>,
}

impl TerminalGrid {
    /// Create a new grid filled with blank spaces and default styles.
    pub fn new(rows: usize, cols: usize) -> Self {
        let rows = rows.max(1);
        let cols = cols.max(1);
        let total = rows * cols;
        Self {
            rows,
            cols,
            cells: vec![Cell::default(); total],
        }
    }

    /// Number of rows in the grid.
    pub fn rows(&self) -> usize {
        self.rows
    }

    /// Number of columns in the grid.
    pub fn cols(&self) -> usize {
        self.cols
    }

    /// Get a reference to the cell at `(row, col)`.
    pub fn get(&self, row: usize, col: usize) -> Option<&Cell> {
        if row < self.rows && col < self.cols {
            Some(&self.cells[row * self.cols + col])
        } else {
            None
        }
    }

    /// Get a mutable reference to the cell at `(row, col)`.
    pub fn get_mut(&mut self, row: usize, col: usize) -> Option<&mut Cell> {
        if row < self.rows && col < self.cols {
            Some(&mut self.cells[row * self.cols + col])
        } else {
            None
        }
    }

    /// Set the cell at `(row, col)`.
    pub fn set(&mut self, row: usize, col: usize, cell: Cell) {
        if row < self.rows && col < self.cols {
            self.cells[row * self.cols + col] = cell;
        }
    }

    /// Clear all cells in the grid to blank spaces with the given style.
    pub fn clear_all(&mut self, style: Style) {
        let blank = Cell::new(' ', style);
        for cell in &mut self.cells {
            *cell = blank;
        }
    }

    /// Clear a portion of a line according to [`EraseLineMode`].
    pub fn clear_line(&mut self, row: usize, col: usize, mode: EraseLineMode, style: Style) {
        if row >= self.rows {
            return;
        }
        let blank = Cell::new(' ', style);
        let start_col = match mode {
            EraseLineMode::ToEnd => col.min(self.cols),
            EraseLineMode::ToStart => 0,
            EraseLineMode::All => 0,
        };
        let end_col = match mode {
            EraseLineMode::ToEnd => self.cols,
            EraseLineMode::ToStart => (col + 1).min(self.cols),
            EraseLineMode::All => self.cols,
        };

        for c in start_col..end_col {
            self.set(row, c, blank);
        }
    }

    /// Clear a portion of the display according to [`EraseDisplayMode`].
    pub fn clear_display(&mut self, row: usize, col: usize, mode: EraseDisplayMode, style: Style) {
        match mode {
            EraseDisplayMode::All | EraseDisplayMode::Scrollback => {
                self.clear_all(style);
            }
            EraseDisplayMode::ToEnd => {
                // Clear remainder of current line
                self.clear_line(row, col, EraseLineMode::ToEnd, style);
                // Clear all subsequent lines
                let blank = Cell::new(' ', style);
                for r in (row + 1)..self.rows {
                    for c in 0..self.cols {
                        self.set(r, c, blank);
                    }
                }
            }
            EraseDisplayMode::ToStart => {
                // Clear all prior lines
                let blank = Cell::new(' ', style);
                for r in 0..row {
                    for c in 0..self.cols {
                        self.set(r, c, blank);
                    }
                }
                // Clear up to cursor on current line
                self.clear_line(row, col, EraseLineMode::ToStart, style);
            }
        }
    }

    /// Scroll grid contents up by `count` lines, inserting blank lines at the bottom.
    pub fn scroll_up(&mut self, count: usize, style: Style) {
        if count == 0 {
            return;
        }
        let blank = Cell::new(' ', style);
        if count >= self.rows {
            self.clear_all(style);
            return;
        }

        let shift = count * self.cols;
        self.cells.copy_within(shift.., 0);

        let clear_start = (self.rows - count) * self.cols;
        for cell in &mut self.cells[clear_start..] {
            *cell = blank;
        }
    }

    /// Scroll grid contents down by `count` lines, inserting blank lines at the top.
    pub fn scroll_down(&mut self, count: usize, style: Style) {
        if count == 0 {
            return;
        }
        let blank = Cell::new(' ', style);
        if count >= self.rows {
            self.clear_all(style);
            return;
        }

        let shift = count * self.cols;
        let len = self.cells.len();
        self.cells.copy_within(..len - shift, shift);

        for cell in &mut self.cells[..shift] {
            *cell = blank;
        }
    }

    /// Extract text content of a single row as a string (with trailing spaces trimmed).
    pub fn line_to_string(&self, row: usize) -> String {
        if row >= self.rows {
            return String::new();
        }
        let start = row * self.cols;
        let line_cells = &self.cells[start..start + self.cols];

        let mut s: String = line_cells.iter().map(|c| c.ch).collect();
        // Trim trailing spaces for clean assertions
        let trimmed_len = s.trim_end().len();
        s.truncate(trimmed_len);
        s
    }

    /// Convert entire grid into a multi-line string.
    pub fn to_text(&self) -> String {
        let mut lines = Vec::with_capacity(self.rows);
        for r in 0..self.rows {
            lines.push(self.line_to_string(r));
        }
        // Remove trailing empty lines
        while lines.last().map(|l| l.is_empty()).unwrap_or(false) {
            lines.pop();
        }
        lines.join("\n")
    }
}

/// A virtual headless terminal screen with cursor management, styled text rendering,
/// and support for DEC private alternate screen buffers.
#[derive(Debug, Clone)]
pub struct Screen {
    rows: usize,
    cols: usize,
    main_grid: TerminalGrid,
    alt_grid: Option<TerminalGrid>,
    cursor: Cursor,
    saved_cursor: Option<Cursor>,
    current_style: Style,
    title: String,
    is_alt_screen: bool,
}

impl Screen {
    /// Create a new virtual terminal screen with dimensions `rows` x `cols`.
    pub fn new(rows: usize, cols: usize) -> Self {
        let rows = rows.max(1);
        let cols = cols.max(1);
        Self {
            rows,
            cols,
            main_grid: TerminalGrid::new(rows, cols),
            alt_grid: None,
            cursor: Cursor::default(),
            saved_cursor: None,
            current_style: Style::default(),
            title: String::new(),
            is_alt_screen: false,
        }
    }

    /// Number of rows in the viewport.
    pub fn rows(&self) -> usize {
        self.rows
    }

    /// Number of columns in the viewport.
    pub fn cols(&self) -> usize {
        self.cols
    }

    /// Current cursor position and visibility state.
    pub fn cursor(&self) -> &Cursor {
        &self.cursor
    }

    /// Window title set via OSC sequences.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Returns `true` if the screen is currently displaying the alternate screen buffer.
    pub fn is_alt_screen(&self) -> bool {
        self.is_alt_screen
    }

    /// Reference to the currently active [`TerminalGrid`].
    pub fn active_grid(&self) -> &TerminalGrid {
        if self.is_alt_screen {
            self.alt_grid.as_ref().unwrap_or(&self.main_grid)
        } else {
            &self.main_grid
        }
    }

    /// Mutable reference to the currently active [`TerminalGrid`].
    pub fn active_grid_mut(&mut self) -> &mut TerminalGrid {
        if self.is_alt_screen {
            if self.alt_grid.is_none() {
                self.alt_grid = Some(TerminalGrid::new(self.rows, self.cols));
            }
            self.alt_grid.as_mut().unwrap()
        } else {
            &mut self.main_grid
        }
    }

    /// Feed a byte chunk into the screen, parsing sequences and updating the grid state.
    pub fn feed(&mut self, input: &[u8]) {
        let mut parser = Parser::new();
        let events = parser.feed(input);
        for event in events {
            self.process_event(&event);
        }
    }

    /// Feed a string slice into the screen.
    pub fn feed_str(&mut self, input: &str) {
        self.feed(input.as_bytes());
    }

    /// Process a single [`Event`] updating the cursor and active grid.
    pub fn process_event(&mut self, event: &Event) {
        match event {
            Event::Print(bytes) => {
                let text = String::from_utf8_lossy(bytes);
                for ch in text.chars() {
                    self.write_char(ch);
                }
            }
            Event::Control(b) => match *b {
                b'\r' => self.cursor.col = 0,
                b'\n' => self.line_feed(),
                0x08 => self.cursor.col = self.cursor.col.saturating_sub(1),
                b'\t' => {
                    let next_tab = (self.cursor.col / 8 + 1) * 8;
                    self.cursor.col = next_tab.min(self.cols - 1);
                }
                _ => {}
            },
            Event::Csi(csi) => match csi.action() {
                CsiAction::Sgr(actions) => {
                    self.current_style.apply_sgr_slice(&actions);
                }
                CsiAction::CursorUp(n) => {
                    self.cursor.row = self.cursor.row.saturating_sub(n as usize);
                }
                CsiAction::CursorDown(n) => {
                    self.cursor.row = (self.cursor.row + n as usize).min(self.rows - 1);
                }
                CsiAction::CursorForward(n) => {
                    self.cursor.col = (self.cursor.col + n as usize).min(self.cols - 1);
                }
                CsiAction::CursorBack(n) => {
                    self.cursor.col = self.cursor.col.saturating_sub(n as usize);
                }
                CsiAction::CursorNextLine(n) => {
                    self.cursor.col = 0;
                    self.cursor.row = (self.cursor.row + n as usize).min(self.rows - 1);
                }
                CsiAction::CursorPreviousLine(n) => {
                    self.cursor.col = 0;
                    self.cursor.row = self.cursor.row.saturating_sub(n as usize);
                }
                CsiAction::CursorHorizontalAbsolute(col) => {
                    self.cursor.col = (col.saturating_sub(1) as usize).min(self.cols - 1);
                }
                CsiAction::CursorPosition { row, col } => {
                    self.cursor.row = (row.saturating_sub(1) as usize).min(self.rows - 1);
                    self.cursor.col = (col.saturating_sub(1) as usize).min(self.cols - 1);
                }
                CsiAction::EraseInDisplay(mode) => {
                    let r = self.cursor.row;
                    let c = self.cursor.col;
                    let style = self.current_style;
                    self.active_grid_mut().clear_display(r, c, mode, style);
                }
                CsiAction::EraseInLine(mode) => {
                    let r = self.cursor.row;
                    let c = self.cursor.col;
                    let style = self.current_style;
                    self.active_grid_mut().clear_line(r, c, mode, style);
                }
                CsiAction::ScrollUp(n) => {
                    let style = self.current_style;
                    self.active_grid_mut().scroll_up(n as usize, style);
                }
                CsiAction::ScrollDown(n) => {
                    let style = self.current_style;
                    self.active_grid_mut().scroll_down(n as usize, style);
                }
                CsiAction::SaveCursorPosition => {
                    self.saved_cursor = Some(self.cursor);
                }
                CsiAction::RestoreCursorPosition => {
                    if let Some(saved) = self.saved_cursor {
                        self.cursor = saved;
                    }
                }
                CsiAction::SetDecPrivateMode(mode) => match mode {
                    25 => self.cursor.visible = true,
                    1049 => {
                        self.saved_cursor = Some(self.cursor);
                        self.is_alt_screen = true;
                        let mut grid = TerminalGrid::new(self.rows, self.cols);
                        grid.clear_all(self.current_style);
                        self.alt_grid = Some(grid);
                        self.cursor.row = 0;
                        self.cursor.col = 0;
                    }
                    47 => {
                        self.is_alt_screen = true;
                        if self.alt_grid.is_none() {
                            self.alt_grid = Some(TerminalGrid::new(self.rows, self.cols));
                        }
                    }
                    _ => {}
                },
                CsiAction::ResetDecPrivateMode(mode) => match mode {
                    25 => self.cursor.visible = false,
                    1049 => {
                        self.is_alt_screen = false;
                        self.alt_grid = None;
                        if let Some(saved) = self.saved_cursor {
                            self.cursor = saved;
                        }
                    }
                    47 => {
                        self.is_alt_screen = false;
                    }
                    _ => {}
                },
                _ => {}
            },
            Event::Escape(esc) => match (esc.intermediates.as_slice(), esc.final_byte) {
                // ESC 7: DECSC (Save cursor)
                (&[], b'7') => self.saved_cursor = Some(self.cursor),
                // ESC 8: DECRC (Restore cursor)
                (&[], b'8') => {
                    if let Some(saved) = self.saved_cursor {
                        self.cursor = saved;
                    }
                }
                // ESC c: RIS (Reset to Initial State)
                (&[], b'c') => {
                    self.main_grid.clear_all(Style::default());
                    self.alt_grid = None;
                    self.cursor = Cursor::default();
                    self.saved_cursor = None;
                    self.current_style = Style::default();
                    self.is_alt_screen = false;
                }
                _ => {}
            },
            Event::Osc(osc) => match osc.action() {
                crate::OscAction::SetWindowTitle(title)
                | crate::OscAction::SetIconAndWindowTitle(title) => {
                    self.title = title;
                }
                _ => {}
            },
            _ => {}
        }
    }

    fn write_char(&mut self, ch: char) {
        let r = self.cursor.row;
        let c = self.cursor.col;
        let cell = Cell::new(ch, self.current_style);

        self.active_grid_mut().set(r, c, cell);

        self.cursor.col += 1;
        if self.cursor.col >= self.cols {
            self.cursor.col = 0;
            self.line_feed();
        }
    }

    fn line_feed(&mut self) {
        if self.cursor.row + 1 >= self.rows {
            let style = self.current_style;
            self.active_grid_mut().scroll_up(1, style);
            self.cursor.row = self.rows - 1;
        } else {
            self.cursor.row += 1;
        }
    }

    /// Extract text of a specific row.
    pub fn line_to_string(&self, row: usize) -> String {
        self.active_grid().line_to_string(row)
    }

    /// Convert the active screen buffer into full trimmed text.
    pub fn to_text(&self) -> String {
        self.active_grid().to_text()
    }
}
