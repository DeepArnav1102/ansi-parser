#![no_main]

use ansi_parser::Event;
use libfuzzer_sys::fuzz_target;

// Feed arbitrary bytes in deterministic pseudo-random chunk sizes.
//
// The final byte of the input selects the chunk length (1..=257) so the
// fuzzer explores split points inside escape introducers, OSC payloads, and
// ST terminators. Chunked parsing must yield the same events as the
// one-shot parse after merging adjacent text runs.
fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }

    let chunk_size = (data[data.len() - 1] as usize % 257) + 1;
    let body = &data[..data.len() - 1];

    let mut one_shot = ansi_parser::Parser::new();
    let expected = one_shot.feed(data);

    let mut chunked = ansi_parser::Parser::new();
    let mut actual = Vec::new();
    for chunk in body.chunks(chunk_size) {
        actual.extend(chunked.feed(chunk));
    }

    assert_events_equivalent(&expected, &actual);
});

/// Compare event lists ignoring where text runs happened to be flushed.
fn assert_events_equivalent(expected: &[Event], actual: &[Event]) {
    let mut expected = expected.to_vec();
    let mut actual = actual.to_vec();

    normalize(&mut expected);
    normalize(&mut actual);

    assert_eq!(
        expected, actual,
        "chunked parsing diverged from one-shot parsing"
    );
}

fn normalize(events: &mut Vec<Event>) {
    let mut merged: Vec<Event> = Vec::with_capacity(events.len());

    for event in events.drain(..) {
        match event {
            Event::Print(bytes) => {
                if let Some(Event::Print(previous)) = merged.last_mut() {
                    previous.extend(bytes);
                } else if !bytes.is_empty() {
                    merged.push(Event::Print(bytes));
                }
            }
            other => merged.push(other),
        }
    }

    *events = merged;
}
