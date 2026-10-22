use nc_parser::{Color, Screen, StandardColor};

#[test]
fn writes_text_and_moves_cursor() {
    let mut screen = Screen::new(5, 10);
    screen.feed_str("Hello");

    assert_eq!(screen.cursor().row, 0);
    assert_eq!(screen.cursor().col, 5);
    assert_eq!(screen.line_to_string(0), "Hello");
}

#[test]
fn wraps_and_scrolls_grid() {
    let mut screen = Screen::new(2, 5);
    screen.feed_str("12345678901");

    assert_eq!(screen.cursor().row, 1);
    assert_eq!(screen.cursor().col, 1);
    // After scrolling, first row is "67890", second row is "1"
    assert_eq!(screen.line_to_string(0), "67890");
    assert_eq!(screen.line_to_string(1), "1");
}

#[test]
fn handles_cursor_positioning_and_clearing() {
    let mut screen = Screen::new(5, 20);
    screen.feed_str("Line 1\r\nLine 2\r\nLine 3");

    // Position cursor at row 2, col 1 (1-indexed 2;1)
    screen.feed(b"\x1b[2;1H");
    assert_eq!(screen.cursor().row, 1);
    assert_eq!(screen.cursor().col, 0);

    // Erase line to end (EL 0)
    screen.feed(b"\x1b[0K");
    assert_eq!(screen.line_to_string(1), "");
    assert_eq!(screen.line_to_string(0), "Line 1");
    assert_eq!(screen.line_to_string(2), "Line 3");

    // Erase entire display (ED 2)
    screen.feed(b"\x1b[2J");
    assert_eq!(screen.to_text(), "");
}

#[test]
fn handles_styled_cells() {
    let mut screen = Screen::new(3, 20);
    screen.feed(b"\x1b[31;1mRed\x1b[0mNormal");

    let grid = screen.active_grid();
    let cell_r = grid.get(0, 0).unwrap();
    assert_eq!(cell_r.ch, 'R');
    assert!(cell_r.style.bold);
    assert_eq!(
        cell_r.style.foreground,
        Some(Color::Standard(StandardColor::Red))
    );

    let cell_n = grid.get(0, 3).unwrap();
    assert_eq!(cell_n.ch, 'N');
    assert!(!cell_n.style.bold);
    assert_eq!(cell_n.style.foreground, None);
}

#[test]
fn toggles_alternate_screen_buffer() {
    let mut screen = Screen::new(3, 20);
    screen.feed_str("Main Screen");
    assert_eq!(screen.line_to_string(0), "Main Screen");
    assert!(!screen.is_alt_screen());

    // Switch to Alternate Screen (DECSET 1049)
    screen.feed(b"\x1b[?1049h");
    assert!(screen.is_alt_screen());
    assert_eq!(screen.line_to_string(0), "");

    screen.feed_str("Alt Screen");
    assert_eq!(screen.line_to_string(0), "Alt Screen");

    // Switch back to Main Screen (DECRST 1049)
    screen.feed(b"\x1b[?1049l");
    assert!(!screen.is_alt_screen());
    assert_eq!(screen.line_to_string(0), "Main Screen");
}
