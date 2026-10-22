use nc_parser::{Color, StandardColor, Style, StyleTracker, to_html, to_styled_spans};

#[test]
fn tracks_styles_across_sgr_codes() {
    let input = b"\x1b[1;31mBold Red \x1b[4mUnderline \x1b[0mNormal";
    let spans = to_styled_spans(input);

    assert_eq!(spans.len(), 3);

    // Span 1: "Bold Red "
    assert_eq!(spans[0].to_string_lossy(), "Bold Red ");
    assert!(spans[0].style.bold);
    assert_eq!(
        spans[0].style.foreground,
        Some(Color::Standard(StandardColor::Red))
    );
    assert!(!spans[0].style.underline);

    // Span 2: "Underline "
    assert_eq!(spans[1].to_string_lossy(), "Underline ");
    assert!(spans[1].style.bold);
    assert!(spans[1].style.underline);

    // Span 3: "Normal"
    assert_eq!(spans[2].to_string_lossy(), "Normal");
    assert_eq!(spans[2].style, Style::default());
}

#[test]
fn merges_adjacent_spans_with_identical_styles() {
    let mut tracker = StyleTracker::new();

    // Two consecutive print chunks with no style changes between them
    let mut spans = tracker.feed_to_spans(b"Hello ");
    spans.extend(tracker.feed_to_spans(b"World"));

    // Feed to spans handles merging internally per feed
    let unified = to_styled_spans(b"Hello World");
    assert_eq!(unified.len(), 1);
    assert_eq!(unified[0].to_string_lossy(), "Hello World");
}

#[test]
fn converts_ansi_to_html_with_css() {
    let input = b"\x1b[1;31mAlert:\x1b[0m \x1b[32mSuccess <&>\x1b[0m";
    let html = to_html(input);

    assert!(html.contains("font-weight:bold;"));
    assert!(html.contains("color:#cd0000;"));
    assert!(html.contains("Alert:"));
    assert!(html.contains("color:#00cd00;"));
    // Escaped entities
    assert!(html.contains("&lt;&amp;&gt;"));
}
