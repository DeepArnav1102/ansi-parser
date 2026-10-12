use ansi_parser::{Event, OscSequence, OscTerminator, Parser};

#[test]
fn parses_osc_terminated_by_bell() {
    let mut parser = Parser::new();

    let events = parser.feed(b"\x1b]0;My Terminal\x07");

    assert_eq!(
        events,
        vec![Event::Osc(OscSequence {
            payload: b"0;My Terminal".to_vec(),
            terminator: OscTerminator::Bell,
        })]
    );
}

#[test]
fn parses_osc_terminated_by_st() {
    let mut parser = Parser::new();

    let events = parser.feed(b"\x1b]0;My Terminal\x1b\\");

    assert_eq!(
        events,
        vec![Event::Osc(OscSequence {
            payload: b"0;My Terminal".to_vec(),
            terminator: OscTerminator::St,
        })]
    );
}

#[test]
fn parses_st_split_across_chunks() {
    let mut parser = Parser::new();

    assert!(parser.feed(b"\x1b]0;Title\x1b").is_empty());

    let events = parser.feed(b"\\");

    assert_eq!(
        events,
        vec![Event::Osc(OscSequence {
            payload: b"0;Title".to_vec(),
            terminator: OscTerminator::St,
        })]
    );
}

#[test]
fn osc_does_not_emit_until_terminated() {
    let mut parser = Parser::new();

    assert!(parser.feed(b"\x1b]0;Incomplete").is_empty());

    let events = parser.feed(b"\x07");

    assert_eq!(
        events,
        vec![Event::Osc(OscSequence {
            payload: b"0;Incomplete".to_vec(),
            terminator: OscTerminator::Bell,
        })]
    );
}
