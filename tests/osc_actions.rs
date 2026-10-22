use nc_parser::{ClipboardTarget, Event, OscAction, Parser};

#[test]
fn decodes_window_titles() {
    let mut parser = Parser::new();

    // OSC 0: Icon & Title
    let events = parser.feed(b"\x1b]0;My App Window\x07");
    if let Event::Osc(osc) = &events[0] {
        assert_eq!(
            osc.action(),
            OscAction::SetIconAndWindowTitle("My App Window".to_string())
        );
    } else {
        panic!("Expected OSC");
    }

    // OSC 2: Window Title (ST terminated)
    let events = parser.feed(b"\x1b]2;Terminal Tab 1\x1b\\");
    if let Event::Osc(osc) = &events[0] {
        assert_eq!(
            osc.action(),
            OscAction::SetWindowTitle("Terminal Tab 1".to_string())
        );
    } else {
        panic!("Expected OSC");
    }
}

#[test]
fn decodes_cwd_and_hyperlinks() {
    let mut parser = Parser::new();

    // OSC 7: Current Working Directory
    let events = parser.feed(b"\x1b]7;file://localhost/home/user/project\x1b\\");
    if let Event::Osc(osc) = &events[0] {
        assert_eq!(
            osc.action(),
            OscAction::SetCurrentWorkingDirectory("file://localhost/home/user/project".to_string())
        );
    } else {
        panic!("Expected OSC");
    }

    // OSC 8: Clickable Hyperlink with metadata
    let events = parser.feed(b"\x1b]8;id=commit-1;https://github.com/rust-lang/rust\x1b\\");
    if let Event::Osc(osc) = &events[0] {
        assert_eq!(
            osc.action(),
            OscAction::SetHyperlink {
                params: "id=commit-1".to_string(),
                url: "https://github.com/rust-lang/rust".to_string(),
            }
        );
    } else {
        panic!("Expected OSC");
    }

    // OSC 8: Close Hyperlink
    let events = parser.feed(b"\x1b]8;;\x1b\\");
    if let Event::Osc(osc) = &events[0] {
        assert_eq!(osc.action(), OscAction::ClearHyperlink);
    } else {
        panic!("Expected OSC");
    }
}

#[test]
fn decodes_clipboard_and_base64() {
    let mut parser = Parser::new();

    // OSC 52: Clipboard write "Hello" (base64: SGVsbG8=)
    let events = parser.feed(b"\x1b]52;c;SGVsbG8=\x07");
    if let Event::Osc(osc) = &events[0] {
        assert_eq!(
            osc.action(),
            OscAction::Clipboard {
                target: ClipboardTarget::Clipboard,
                query: false,
                base64_data: Some("SGVsbG8=".to_string()),
            }
        );

        assert_eq!(osc.decode_clipboard_data(), Some(b"Hello".to_vec()));
    } else {
        panic!("Expected OSC");
    }

    // OSC 52: Primary selection query '?'
    let events = parser.feed(b"\x1b]52;p;?\x07");
    if let Event::Osc(osc) = &events[0] {
        assert_eq!(
            osc.action(),
            OscAction::Clipboard {
                target: ClipboardTarget::Primary,
                query: true,
                base64_data: None,
            }
        );
    } else {
        panic!("Expected OSC");
    }
}

#[test]
fn decodes_color_queries_and_sets() {
    let mut parser = Parser::new();

    // OSC 10: Query FG
    let events = parser.feed(b"\x1b]10;?\x07");
    if let Event::Osc(osc) = &events[0] {
        assert_eq!(osc.action(), OscAction::QueryForegroundColor);
    }

    // OSC 11: Set BG
    let events = parser.feed(b"\x1b]11;rgb:0000/0000/0000\x07");
    if let Event::Osc(osc) = &events[0] {
        assert_eq!(
            osc.action(),
            OscAction::SetBackgroundColor("rgb:0000/0000/0000".to_string())
        );
    }
}
