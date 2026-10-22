use nc_parser::{Event, Parser};

#[test]
fn parses_plain_text() {
    let mut parser = Parser::new();

    let events = parser.feed(b"Hello");

    assert_eq!(events, vec![Event::Print(b"Hello".to_vec())]);
}

#[test]
fn parses_csi_across_multiple_chunks() {
    let mut parser = Parser::new();

    // ESC
    assert!(parser.feed(b"\x1b").is_empty());

    // [
    assert!(parser.feed(b"[").is_empty());

    // Parameter bytes
    assert!(parser.feed(b"31").is_empty());

    // Final byte
    let events = parser.feed(b"m");

    assert_eq!(
        events,
        vec![Event::Csi(nc_parser::CsiSequence {
            parameter_bytes: b"31".to_vec(),
            intermediates: vec![],
            final_byte: b'm',
        })]
    );
}

#[test]
fn parses_text_before_csi() {
    let mut parser = Parser::new();

    let events = parser.feed(b"Hello\x1b[31m");

    assert_eq!(
        events,
        vec![
            Event::Print(b"Hello".to_vec()),
            Event::Csi(nc_parser::CsiSequence {
                parameter_bytes: b"31".to_vec(),
                intermediates: vec![],
                final_byte: b'm',
            }),
        ]
    );
}

#[test]
fn parses_multiple_sequences() {
    let mut parser = Parser::new();

    let events = parser.feed(b"\x1b[31mRed\x1b[0m");

    assert_eq!(
        events,
        vec![
            Event::Csi(nc_parser::CsiSequence {
                parameter_bytes: b"31".to_vec(),
                intermediates: vec![],
                final_byte: b'm',
            }),
            Event::Print(b"Red".to_vec()),
            Event::Csi(nc_parser::CsiSequence {
                parameter_bytes: b"0".to_vec(),
                intermediates: vec![],
                final_byte: b'm',
            }),
        ]
    );
}

#[test]
fn handles_line_feed() {
    let mut parser = Parser::new();

    let events = parser.feed(b"Hello\nWorld");

    assert_eq!(
        events,
        vec![
            Event::Print(b"Hello".to_vec()),
            Event::Control(b'\n'),
            Event::Print(b"World".to_vec()),
        ]
    );
}
