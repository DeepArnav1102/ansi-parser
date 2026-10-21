#![no_main]

use libfuzzer_sys::fuzz_target;

// The input decoder must never panic and must always make forward progress:
// every successful decode consumes at least one byte.
fuzz_target!(|data: &[u8]| {
    let mut offset = 0usize;

    while offset < data.len() {
        match ansi_parser::decode_input_sequence(&data[offset..]) {
            Some((_event, consumed)) => {
                assert!(consumed > 0, "decoder consumed zero bytes");
                offset += consumed;
            }
            None => break,
        }
    }
});
