use ansi_parser::{Event, Parser};

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

fn parse_in_chunks(input: &[u8], chunk_size: usize) -> Vec<Event> {
    assert!(chunk_size > 0);

    let mut parser = Parser::new();
    let mut events = Vec::new();

    for chunk in input.chunks(chunk_size) {
        events.extend(parser.feed(chunk));
    }

    normalize(events)
}

#[test]
fn every_two_chunk_split_matches_single_chunk() {
    let input: &[u8] = b"Hello\x1b[31mRed\x1b[0m!\n\x1b[2;10HDone\x1b7";

    let expected = parse_in_chunks(input, input.len());

    for split in 0..=input.len() {
        let mut parser = Parser::new();
        let mut events = Vec::new();

        events.extend(parser.feed(&input[..split]));
        events.extend(parser.feed(&input[split..]));

        assert_eq!(
            normalize(events),
            expected,
            "Different result when splitting input at byte {split}"
        );
    }
}

#[test]
fn one_byte_chunks_match_single_chunk() {
    let input: &[u8] = b"Hello\x1b[31mRed\x1b[0m!\n\x1b[2;10HDone\x1b7";

    assert_eq!(
        parse_in_chunks(input, 1),
        parse_in_chunks(input, input.len()),
    );
}

#[test]
fn all_fixed_chunk_sizes_match() {
    let input: &[u8] = b"Start\x1b[38;2;255;128;0mOrange\x1b[0m\x1b[2JEnd";

    let expected = parse_in_chunks(input, input.len());

    for size in 1..=input.len() {
        assert_eq!(
            parse_in_chunks(input, size),
            expected,
            "Different result with chunk size {size}"
        );
    }
}
