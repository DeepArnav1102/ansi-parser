use nc_parser::{Event, Parser};

#[test]
fn oversized_osc_is_ignored_and_parser_recovers() {
    let mut parser = Parser::new();

    let mut input = b"\x1b]0;".to_vec();
    input.extend(std::iter::repeat_n(b'A', 5000));
    input.extend_from_slice(b"\x07OK");

    let events = parser.feed(&input);

    assert!(
        !events.iter().any(|event| matches!(event, Event::Osc(_))),
        "Oversized OSC should not be emitted"
    );

    assert!(
        events.iter().any(|event| matches!(
            event,
            Event::Print(bytes) if bytes == b"OK"
        )),
        "Parser should recover and emit text after OSC termination"
    );
}
