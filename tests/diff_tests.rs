use nc_parser::{Screen, ScreenDiff, generate_ansi_diff, to_snapshot, to_svg};

#[test]
fn diffs_identical_screens() {
    let mut s1 = Screen::new(5, 10);
    s1.feed_str("Hello");

    let s2 = s1.clone();
    let diff = ScreenDiff::compute(&s1, &s2);

    assert!(diff.is_empty());
    assert_eq!(diff.patch_count(), 0);
    assert!(generate_ansi_diff(&s1, &s2).is_empty());
}

#[test]
fn computes_diff_and_generates_ansi_patch() {
    let mut s1 = Screen::new(3, 10);
    s1.feed_str("Hello");

    let mut s2 = s1.clone();
    s2.feed(b"\x1b[1;1H\x1b[31mJ\x1b[0m"); // Replace 'H' with red 'J' at (1, 1)

    let diff = ScreenDiff::compute(&s1, &s2);
    assert_eq!(diff.patch_count(), 1);

    // Generate ANSI patch
    let patch = generate_ansi_diff(&s1, &s2);
    assert!(!patch.is_empty());

    // Applying patch to s1 should transform it into s2!
    s1.feed(&patch);
    assert_eq!(s1.to_text(), s2.to_text());
    assert_eq!(s1.cursor(), s2.cursor());
}

#[test]
fn renders_clean_snapshot() {
    let mut screen = Screen::new(3, 10);
    screen.feed_str("Title\r\nItem 1");

    let snapshot = to_snapshot(&screen);
    assert!(snapshot.contains("+------------+"));
    assert!(snapshot.contains("| Title      |"));
    assert!(snapshot.contains("| Item 1     |"));
}

#[test]
fn generates_valid_svg_image() {
    let mut screen = Screen::new(4, 15);
    screen.feed(b"\x1b[32mSuccess!\x1b[0m");

    let svg = to_svg(&screen);
    assert!(svg.starts_with("<svg"));
    assert!(svg.ends_with("</svg>\n"));
    assert!(svg.contains(">S</text>"));
    assert!(svg.contains(">!</text>"));
    assert!(svg.contains("#00cd00")); // Green text
}
