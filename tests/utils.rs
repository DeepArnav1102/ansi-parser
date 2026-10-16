use ansi_parser::{CsiSequence, Event, Parser, strip_ansi, strip_ansi_to_string};

#[test]
fn test_strip_ansi_bytes() {
    let raw = b"\x1b[31;1mError:\x1b[0m File \x1b[4mnot found\x1b[0m!\n";
    let stripped = strip_ansi(raw);
    assert_eq!(stripped, b"Error: File not found!\n");
}

#[test]
fn test_strip_ansi_string() {
    let input = "\x1b[38;2;255;128;0mOrange\x1b[0m text and \x1b[?25hcursor";
    let clean = strip_ansi_to_string(input);
    assert_eq!(clean, "Orange text and cursor");
}

#[test]
fn test_feed_each_zero_allocation() {
    let mut parser = Parser::new();
    let mut print_count = 0;
    let mut csi_count = 0;

    parser.feed_each(b"\x1b[31mHello\x1b[0m World", |event| match event {
        Event::Print(_) => print_count += 1,
        Event::Csi(_) => csi_count += 1,
        _ => {}
    });

    assert_eq!(csi_count, 2);
    assert_eq!(print_count, 2);
}

#[test]
fn test_lossless_serialization_roundtrip() {
    let mut parser = Parser::new();
    let input = b"\x1b[38;2;255;0;0mRed\x1b[0m";
    let events = parser.feed(input);

    let mut serialized = Vec::new();
    for event in &events {
        serialized.extend(event.to_bytes());
    }

    assert_eq!(serialized, input);

    // Also check Display formatting
    let csi = CsiSequence {
        parameter_bytes: b"31".to_vec(),
        intermediates: Vec::new(),
        final_byte: b'm',
    };
    assert_eq!(format!("{csi}"), "\x1b[31m");
}
