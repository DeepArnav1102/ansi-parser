//! Property-based tests for the parser and its consumers.
//!
//! These complement the scenario-focused integration tests with randomized
//! invariants:
//!
//! - Chunk-boundary invariance: any split of the input yields the same events.
//! - Panic freedom: arbitrary bytes never panic the parser, screen, or decoder.
//! - Bounded memory: payload/parameter limits are always enforced.
//! - Round-trip: parsed sequences re-serialize and re-parse identically.
//! - Recovery: valid sequences still parse after arbitrary garbage.
//! - `strip_ansi` output never contains escape bytes and is idempotent.

use ansi_parser::{
    decode_input_sequence, strip_ansi, Cell, CsiSequence, Cursor, Event, OscTerminator, Parser,
    Screen, ScreenDiff,
};
use proptest::collection::vec;
use proptest::prelude::*;

/// Merge adjacent `Print` events so event lists compare equal regardless of
/// how the input was chunked.
fn normalize(events: Vec<Event>) -> Vec<Event> {
    let mut normalized: Vec<Event> = Vec::new();

    for event in events {
        match event {
            Event::Print(bytes) => {
                if let Some(Event::Print(previous)) = normalized.last_mut() {
                    previous.extend(bytes);
                } else if !bytes.is_empty() {
                    normalized.push(Event::Print(bytes));
                }
            }
            other => normalized.push(other),
        }
    }

    normalized
}

fn parse_all(parser: &mut Parser, input: &[u8]) -> Vec<Event> {
    let mut events = Vec::new();
    parser.feed_each(input, |event| events.push(event));
    events
}

/// Bytes guaranteed not to collide with OSC/DCS terminator bytes
/// (0x07 BEL, 0x1B ESC, 0x5C backslash). Uses `prop_map` so no generated
/// value is ever rejected (avoiding rejection storms on large collections).
fn safe_payload_byte() -> impl Strategy<Value = u8> {
    any::<u8>().prop_map(|b| match b {
        0x07 | 0x1B | b'\\' => b'A',
        other => other,
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    /// The parser never panics on arbitrary bytes, no matter how the input
    /// is chunked.
    #[test]
    fn parser_never_panics_on_arbitrary_bytes(input in vec(any::<u8>(), 0..1024)) {
        let mut parser = Parser::new();
        let _ = parse_all(&mut parser, &input);
    }

    /// Feeding the same bytes with any chunk size produces exactly the same
    /// event stream (after merging adjacent text runs).
    #[test]
    fn events_are_chunk_size_invariant(
        input in vec(any::<u8>(), 0..1024),
        chunk_size in 1usize..=97,
    ) {
        let mut parser = Parser::new();
        let expected = parse_all(&mut parser, &input);

        let mut chunked = Parser::new();
        let mut actual = Vec::new();
        for chunk in input.chunks(chunk_size.max(1)) {
            actual.extend(parse_all(&mut chunked, chunk));
        }

        prop_assert_eq!(normalize(expected), normalize(actual));
    }

    /// The pull-based iterator produces the same events as batch feeding.
    #[test]
    fn iterator_matches_batch_feed(input in vec(any::<u8>(), 0..512)) {
        let mut parser = Parser::new();
        let expected = parse_all(&mut parser, &input);

        let mut parser = Parser::new();
        let actual: Vec<Event> = parser.iter(&input).collect();

        prop_assert_eq!(normalize(expected), normalize(actual));
    }

    /// OSC payloads are capped at 4096 bytes and the parser still recovers
    /// to ground state afterwards.
    #[test]
    fn osc_payload_never_exceeds_limit(filler in vec(safe_payload_byte(), 4097..4600)) {
        let mut input = b"\x1b]2;".to_vec();
        input.extend_from_slice(&filler);
        input.extend_from_slice(b"\x07tail");

        let mut parser = Parser::new();
        let events = parse_all(&mut parser, &input);

        for event in &events {
            if let Event::Osc(osc) = event {
                prop_assert!(osc.payload.len() <= 4096, "payload {}", osc.payload.len());
            }
        }

        let trailing: Vec<u8> = events
            .iter()
            .filter_map(|event| match event {
                Event::Print(bytes) => Some(bytes.clone()),
                _ => None,
            })
            .flatten()
            .collect();
        prop_assert_eq!(&trailing, b"tail");
    }

    /// CSI parameter and intermediate byte buffers never exceed their caps.
    #[test]
    fn csi_buffers_never_exceed_limits(
        params in vec(0x30u8..=0x3F, 0..600),
        finals in 0u8..8,
    ) {
        let mut input = b"\x1b[".to_vec();
        input.extend_from_slice(&params);
        input.push(0x40 + finals);

        let mut parser = Parser::new();
        let events = parse_all(&mut parser, &input);

        if params.len() <= 256 {
            // Under the cap the full sequence is emitted with every byte kept.
            prop_assert_eq!(events.len(), 1);
            match &events[0] {
                Event::Csi(csi) => prop_assert_eq!(&csi.parameter_bytes, &params),
                other => panic!("expected CSI event, got {other:?}"),
            }
        } else {
            // Over the cap the parser enters ignore mode and emits nothing.
            prop_assert!(
                !events.iter().any(|e| matches!(e, Event::Csi(_))),
                "oversized CSI should not be emitted"
            );
        }
    }

    /// DCS payloads are capped at 32768 bytes.
    #[test]
    fn dcs_payload_never_exceeds_limit(filler in vec(safe_payload_byte(), 32769..32800)) {
        let mut input = b"\x1bPq".to_vec();
        input.extend_from_slice(&filler);
        input.extend_from_slice(b"\x1b\\");

        let mut parser = Parser::new();
        let events = parse_all(&mut parser, &input);

        for event in &events {
            if let Event::Dcs(dcs) = event {
                prop_assert!(dcs.payload.len() <= 32768, "payload {}", dcs.payload.len());
            }
        }
    }

    /// A well-formed sequence re-serializes to bytes that parse back into
    /// the identical sequence. Parameters and intermediates are generated
    /// separately because the parser only accepts parameter bytes before
    /// the first intermediate byte.
    #[test]
    fn csi_round_trips_through_to_bytes(
        parameter_bytes in vec(0x30u8..=0x3F, 0..8),
        intermediates in vec(0x20u8..=0x2F, 0..4),
        final_byte in 0x40u8..=0x7E,
    ) {
        let csi = CsiSequence {
            parameter_bytes,
            intermediates,
            final_byte,
        };

        let mut parser = Parser::new();
        let events = parse_all(&mut parser, &csi.to_bytes());

        prop_assert_eq!(events, vec![Event::Csi(csi)]);
    }

    /// OSC sequences terminate with whichever terminator they were given and
    /// survive a re-encode/re-parse cycle.
    #[test]
    fn osc_round_trips(
        payload in vec(safe_payload_byte(), 0..64),
        use_bell in any::<bool>(),
    ) {
        let terminator = if use_bell {
            OscTerminator::Bell
        } else {
            OscTerminator::St
        };

        let raw = if use_bell {
            let mut bytes = b"\x1b]".to_vec();
            bytes.extend_from_slice(&payload);
            bytes.push(0x07);
            bytes
        } else {
            let mut bytes = b"\x1b]".to_vec();
            bytes.extend_from_slice(&payload);
            bytes.extend_from_slice(b"\x1b\\");
            bytes
        };

        let mut parser = Parser::new();
        let events = parse_all(&mut parser, &raw);

        prop_assert_eq!(events.len(), 1);
        match &events[0] {
            Event::Osc(osc) => {
                prop_assert_eq!(&osc.payload, &payload);
                prop_assert_eq!(osc.terminator, terminator);
            }
            other => panic!("expected OSC event, got {other:?}"),
        }
    }

    /// `strip_ansi` never emits escape bytes, and stripping its output again
    /// is a no-op.
    #[test]
    fn strip_ansi_removes_all_escapes_and_is_idempotent(input in vec(any::<u8>(), 0..512)) {
        let once = strip_ansi(&input);
        prop_assert!(!once.contains(&0x1B), "output still contains ESC");

        let twice = strip_ansi(&once);
        prop_assert_eq!(once, twice);
    }

    /// Printable garbage cannot corrupt parser state: a valid SGR sequence
    /// immediately after it is still recognized.
    #[test]
    fn valid_sequence_survives_preceding_garbage(
        garbage in vec(0x20u8..=0x7E, 0..256),
        params in vec(0x30u8..=0x3F, 0..6),
    ) {
        let mut input = garbage.clone();
        input.extend_from_slice(b"\x1b[");
        input.extend_from_slice(&params);
        input.push(b'm');

        let mut parser = Parser::new();
        let events = parse_all(&mut parser, &input);

        let expected = CsiSequence {
            parameter_bytes: params,
            intermediates: Vec::new(),
            final_byte: b'm',
        };

        prop_assert_eq!(events.last(), Some(&Event::Csi(expected)));
    }

    /// The virtual screen never panics and always keeps the cursor within
    /// the grid bounds, for any event stream.
    #[test]
    fn screen_stays_in_bounds_on_arbitrary_events(input in vec(any::<u8>(), 0..1024)) {
        let mut parser = Parser::new();
        let events = parse_all(&mut parser, &input);

        let mut screen = Screen::new(24, 80);
        for event in &events {
            screen.process_event(event);
        }

        let cursor: &Cursor = screen.cursor();
        prop_assert!(cursor.row < 24);
        prop_assert!(cursor.col < 80);
    }

    /// Diff computation over arbitrary screen contents produces patches that
    /// reference only in-bounds cells.
    #[test]
    fn diff_patches_are_in_bounds(
        a in vec(any::<u8>(), 0..256),
        b in vec(any::<u8>(), 0..256),
    ) {
        let mut screen_a = Screen::new(6, 20);
        let mut screen_b = Screen::new(6, 20);

        let mut parser = Parser::new();
        for event in parse_all(&mut parser, &a) {
            screen_a.process_event(&event);
        }
        for event in parse_all(&mut parser, &b) {
            screen_b.process_event(&event);
        }

        let diff = ScreenDiff::compute(&screen_a, &screen_b);
        for patch in &diff.patches {
            prop_assert!(patch.row < 6 && patch.col < 20);
        }
    }

    /// The input decoder never panics on arbitrary bytes.
    #[test]
    fn input_decoder_never_panics(input in vec(any::<u8>(), 0..256)) {
        let mut offset = 0usize;
        while offset < input.len() {
            match decode_input_sequence(&input[offset..]) {
                Some((_event, consumed)) => {
                    prop_assert!(consumed > 0, "decoder consumed zero bytes");
                    offset += consumed;
                }
                None => break,
            }
        }
    }
}

/// The chunk-invariance property also holds for adversarial splits at every
/// single byte position for a known tricky input (splitting escape
/// introducers, OSC payloads, and ST terminators).
#[test]
fn byte_split_at_every_position_is_invariant() {
    let cases: &[&[u8]] = &[
        b"Hello\x1b[31mRed\x1b[0m!",
        b"\x1b]0;title\x07tail",
        b"\x1b]52;c;aGVsbG8=\x1b\\",
        b"\x1bPq;1;2\x1b\\after",
        b"\x1b[2;10H\x1b7\x1b8",
        b"a\x1b",
        b"\x1b",
        b"\x1b[",
        b"\x1b]52;c",
    ];

    for &input in cases {
        let mut parser = Parser::new();
        let expected = parse_all(&mut parser, input);

        for split in 0..=input.len() {
            let mut parser = Parser::new();
            let mut actual = parse_all(&mut parser, &input[..split]);
            actual.extend(parse_all(&mut parser, &input[split..]));

            assert_eq!(
                normalize(expected.clone()),
                normalize(actual),
                "mismatch when splitting {input:?} at byte {split}"
            );
        }
    }
}

// Randomized cells and styles applied to a grid keep `get`/`set` consistent.
proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    #[test]
    fn grid_set_get_round_trip(
        rows in 1usize..12,
        cols in 1usize..24,
        writes in vec(((0usize..11), (0usize..23), any::<char>()), 0..100),
    ) {
        let mut screen = Screen::new(rows, cols);
        let mut expected: Vec<((usize, usize), char)> = Vec::new();

        for &(row, col, ch) in &writes {
            screen
                .active_grid_mut()
                .set(row, col, Cell::new(ch, Default::default()));

            // Track only the final write per in-bounds cell, matching the
            // last-writer-wins semantics of `set`.
            if row < rows && col < cols {
                match expected.iter_mut().find(|((r, c), _)| *r == row && *c == col) {
                    Some((_, last)) => *last = ch,
                    None => expected.push(((row, col), ch)),
                }
            }
        }

        for &((row, col), ch) in &expected {
            prop_assert_eq!(screen.active_grid().get(row, col).unwrap().ch, ch);
        }
    }
}
