use nc_parser::{
    Color, CsiAction, EraseDisplayMode, EraseLineMode, Event, Parser, SgrAction, StandardColor,
};

#[test]
fn decodes_sgr_colors_and_styles() {
    let mut parser = Parser::new();
    let events = parser.feed(b"\x1b[1;31;42;4mText\x1b[0m");

    let csi_start = match &events[0] {
        Event::Csi(c) => c,
        _ => panic!("Expected CSI"),
    };

    let actions = csi_start.parse_sgr();
    assert_eq!(
        actions,
        vec![
            SgrAction::Bold,
            SgrAction::SetForegroundColor(Color::Standard(StandardColor::Red)),
            SgrAction::SetBackgroundColor(Color::Standard(StandardColor::Green)),
            SgrAction::Underline,
        ]
    );

    let csi_reset = match &events[2] {
        Event::Csi(c) => c,
        _ => panic!("Expected CSI reset"),
    };
    assert_eq!(csi_reset.parse_sgr(), vec![SgrAction::Reset]);
}

#[test]
fn decodes_256_color_and_truecolor() {
    let mut parser = Parser::new();

    // 256 color: 38;5;196 (bright red)
    let events = parser.feed(b"\x1b[38;5;196m");
    if let Event::Csi(c) = &events[0] {
        assert_eq!(
            c.parse_sgr(),
            vec![SgrAction::SetForegroundColor(Color::Indexed(196))]
        );
    } else {
        panic!("Expected CSI");
    }

    // 24-bit TrueColor RGB: 48;2;255;128;64
    let events = parser.feed(b"\x1b[48;2;255;128;64m");
    if let Event::Csi(c) = &events[0] {
        assert_eq!(
            c.parse_sgr(),
            vec![SgrAction::SetBackgroundColor(Color::Rgb(255, 128, 64))]
        );
    } else {
        panic!("Expected CSI");
    }

    // ISO colon format: 38:2::10:20:30
    let events = parser.feed(b"\x1b[38:2::10:20:30m");
    if let Event::Csi(c) = &events[0] {
        assert_eq!(
            c.parse_sgr(),
            vec![SgrAction::SetForegroundColor(Color::Rgb(10, 20, 30))]
        );
    } else {
        panic!("Expected CSI");
    }
}

#[test]
fn decodes_cursor_and_screen_actions() {
    let mut parser = Parser::new();

    // Move cursor up 5
    let events = parser.feed(b"\x1b[5A");
    if let Event::Csi(c) = &events[0] {
        assert_eq!(c.action(), CsiAction::CursorUp(5));
    }

    // Default cursor up (omitted param defaults to 1)
    let events = parser.feed(b"\x1b[A");
    if let Event::Csi(c) = &events[0] {
        assert_eq!(c.action(), CsiAction::CursorUp(1));
    }

    // Cursor position: row 12, col 34
    let events = parser.feed(b"\x1b[12;34H");
    if let Event::Csi(c) = &events[0] {
        assert_eq!(c.action(), CsiAction::CursorPosition { row: 12, col: 34 });
    }

    // Erase in display: entire screen (2J)
    let events = parser.feed(b"\x1b[2J");
    if let Event::Csi(c) = &events[0] {
        assert_eq!(c.action(), CsiAction::EraseInDisplay(EraseDisplayMode::All));
    }

    // Erase in line: to end of line (0K)
    let events = parser.feed(b"\x1b[0K");
    if let Event::Csi(c) = &events[0] {
        assert_eq!(c.action(), CsiAction::EraseInLine(EraseLineMode::ToEnd));
    }
}

#[test]
fn decodes_dec_private_modes() {
    let mut parser = Parser::new();

    // Hide cursor: ?25l
    let events = parser.feed(b"\x1b[?25l");
    if let Event::Csi(c) = &events[0] {
        assert_eq!(c.action(), CsiAction::ResetDecPrivateMode(25));
    }

    // Show cursor: ?25h
    let events = parser.feed(b"\x1b[?25h");
    if let Event::Csi(c) = &events[0] {
        assert_eq!(c.action(), CsiAction::SetDecPrivateMode(25));
    }

    // Alternate screen: ?1049h
    let events = parser.feed(b"\x1b[?1049h");
    if let Event::Csi(c) = &events[0] {
        assert_eq!(c.action(), CsiAction::SetDecPrivateMode(1049));
    }
}
