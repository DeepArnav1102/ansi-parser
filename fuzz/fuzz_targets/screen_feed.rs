#![no_main]

use libfuzzer_sys::fuzz_target;

// Process arbitrary event streams through the headless screen. Guards
// against panics in cursor motion, scrolling, and erasing, and asserts the
// cursor always remains within the grid bounds.
fuzz_target!(|data: &[u8]| {
    let mut parser = nc_parser::Parser::new();
    let events = parser.feed(data);

    let (rows, cols) = (6usize, 24usize);
    let mut screen = nc_parser::Screen::new(rows, cols);

    for event in &events {
        screen.process_event(event);
    }

    let cursor = screen.cursor();
    assert!(cursor.row < rows, "cursor row out of bounds: {}", cursor.row);
    assert!(cursor.col < cols, "cursor col out of bounds: {}", cursor.col);
});
