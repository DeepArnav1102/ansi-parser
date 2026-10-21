#![no_main]

use libfuzzer_sys::fuzz_target;

// `strip_ansi` must never emit an escape byte, never panic, and be
// idempotent.
fuzz_target!(|data: &[u8]| {
    let once = ansi_parser::strip_ansi(data);
    assert!(
        !once.contains(&0x1B),
        "strip_ansi output still contains ESC"
    );

    let twice = ansi_parser::strip_ansi(&once);
    assert_eq!(once, twice, "strip_ansi is not idempotent");
});
