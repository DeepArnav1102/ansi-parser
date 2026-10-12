use ansi_parser::{Event, Parser};

#[test]
fn incomplete_csi_does_not_panic() {
    let mut parser = Parser::new();

    let events = parser.feed(b"\x1b[31");

    assert!(events.is_empty());
}

#[test]
fn incomplete_osc_does_not_emit_an_event() {
    let mut parser = Parser::new();

    let events = parser.feed(b"\x1b]0;unfinished title");

    assert!(!events.iter().any(|event| matches!(event, Event::Osc(_))));
}

#[test]
fn interrupted_osc_preserves_embedded_escape() {
    let mut parser = Parser::new();

    let events = parser.feed(b"\x1b]0;ab\x1bXcd\x07");

    assert!(events.iter().any(|event| matches!(
        event,
        Event::Osc(sequence)
            if sequence.payload == b"0;ab\x1bXcd"
    )));
}

#[test]
fn parser_recovers_after_osc_terminator() {
    let mut parser = Parser::new();

    let events = parser.feed(b"\x1b]0;title\x07hello");

    assert!(events.iter().any(|event| matches!(
        event,
        Event::Osc(sequence) if sequence.payload == b"0;title"
    )));

    assert!(events.iter().any(|event| matches!(
        event,
        Event::Print(bytes) if bytes == b"hello"
    )));
}
