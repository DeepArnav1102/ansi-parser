use ansi_parser::{DcsSequence, Event, OscTerminator, Parser};

#[test]
fn parses_dcs_terminated_by_st() {
    let mut parser = Parser::new();
    // ESC P 1 $ q "p ESC \ (DECRQSS request)
    let input = b"\x1bP1$q\"p\x1b\\";
    let events = parser.feed(input);

    assert_eq!(
        events,
        vec![Event::Dcs(DcsSequence {
            parameter_bytes: b"1".to_vec(),
            intermediates: b"$".to_vec(),
            final_byte: b'q',
            payload: b"\"p".to_vec(),
            terminator: OscTerminator::St,
        })]
    );
}

#[test]
fn parses_dcs_terminated_by_bell() {
    let mut parser = Parser::new();
    let input = b"\x1bP0;1|payload\x07";
    let events = parser.feed(input);

    assert_eq!(
        events,
        vec![Event::Dcs(DcsSequence {
            parameter_bytes: b"0;1".to_vec(),
            intermediates: vec![],
            final_byte: b'|',
            payload: b"payload".to_vec(),
            terminator: OscTerminator::Bell,
        })]
    );
}

#[test]
fn parses_dcs_split_across_chunks() {
    let mut parser = Parser::new();

    assert!(parser.feed(b"\x1bP").is_empty());
    assert!(parser.feed(b"1$").is_empty());
    assert!(parser.feed(b"q").is_empty());
    assert!(parser.feed(b"data").is_empty());
    assert!(parser.feed(b"\x1b").is_empty());

    let events = parser.feed(b"\\");
    assert_eq!(
        events,
        vec![Event::Dcs(DcsSequence {
            parameter_bytes: b"1".to_vec(),
            intermediates: b"$".to_vec(),
            final_byte: b'q',
            payload: b"data".to_vec(),
            terminator: OscTerminator::St,
        })]
    );
}

#[test]
fn dcs_oversized_payload_is_ignored_and_recovers() {
    let mut parser = Parser::new();

    let mut input = b"\x1bP1q".to_vec();
    input.extend(std::iter::repeat_n(b'A', 35000));
    input.extend_from_slice(b"\x1b\\Recovered");

    let events = parser.feed(&input);

    assert!(
        !events.iter().any(|event| matches!(event, Event::Dcs(_))),
        "Oversized DCS must not be emitted"
    );

    assert!(
        events.iter().any(|event| matches!(
            event,
            Event::Print(bytes) if bytes == b"Recovered"
        )),
        "Parser must recover and emit trailing text"
    );
}

#[test]
fn test_8bit_c1_mode_toggle() {
    let mut parser = Parser::new(); // default: 8-bit C1 disabled (UTF-8 safe)
    // 0x90 is DCS in 8-bit, but in UTF-8 it's a continuation byte
    let events = parser.feed(b"\x901qdata\x07");
    // Should be treated as plain print bytes because 8-bit C1 is disabled by default
    assert!(events.iter().any(|event| matches!(event, Event::Print(_))));

    let mut c1_parser = Parser::new().with_8bit_c1(true);
    let events = c1_parser.feed(b"\x901qdata\x07");
    assert!(events.iter().any(|event| matches!(event, Event::Dcs(_))));
}
