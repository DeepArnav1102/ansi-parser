use ansi_parser::{AnsiReader, AnsiSpanReader, Color, Event, Parser, StandardColor};
use std::io::Cursor;

#[test]
fn reads_events_incrementally_from_reader() {
    let raw = b"Line 1\n\x1b[32mOK\x1b[0m\n".to_vec();
    let cursor = Cursor::new(raw);
    let mut reader = AnsiReader::new(cursor);

    let mut events = Vec::new();
    while let Some(event) = reader.next_event().unwrap() {
        events.push(event);
    }

    assert_eq!(events.len(), 6);
    assert_eq!(events[0], Event::Print(b"Line 1".to_vec()));
    assert_eq!(events[1], Event::Control(b'\n'));
    assert!(matches!(events[2], Event::Csi(_)));
    assert_eq!(events[3], Event::Print(b"OK".to_vec()));
    assert!(matches!(events[4], Event::Csi(_)));
    assert_eq!(events[5], Event::Control(b'\n'));
}

#[test]
fn reads_styled_spans_from_reader() {
    let raw = b"\x1b[31mError\x1b[0m \x1b[32mPassed\x1b[0m".to_vec();
    let cursor = Cursor::new(raw);
    let span_reader = AnsiSpanReader::new(cursor);

    let spans: Vec<_> = span_reader.map(|res| res.unwrap()).collect();

    assert_eq!(spans.len(), 3);
    assert_eq!(spans[0].to_string_lossy(), "Error");
    assert_eq!(
        spans[0].style.foreground,
        Some(Color::Standard(StandardColor::Red))
    );

    assert_eq!(spans[1].to_string_lossy(), " ");
    assert_eq!(spans[1].style.foreground, None);

    assert_eq!(spans[2].to_string_lossy(), "Passed");
    assert_eq!(
        spans[2].style.foreground,
        Some(Color::Standard(StandardColor::Green))
    );
}

#[test]
fn parses_using_parser_iter() {
    let mut parser = Parser::new();
    let input = b"\x1b[31mRed\x1b[0m";

    let mut count = 0;
    for event in parser.iter(input) {
        match event {
            Event::Csi(_) => count += 1,
            Event::Print(bytes) => {
                assert_eq!(bytes, b"Red");
                count += 1;
            }
            _ => {}
        }
    }

    assert_eq!(count, 3);
}
