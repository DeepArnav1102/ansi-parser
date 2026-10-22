# nc-parser

[![CI](https://github.com/DeepArnav1102/nc-parser/actions/workflows/ci.yml/badge.svg)](https://github.com/DeepArnav1102/nc-parser/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/nc-parser.svg)](https://crates.io/crates/nc-parser)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![no_std compatible](https://img.shields.io/badge/no__std-compatible-success.svg)](https://docs.rs/nc-parser)

`nc-parser` is an incremental ANSI/ECMA-48 parser and lightweight headless
terminal toolkit. It is intended for terminal-output inspection, log viewers,
snapshot tests, ANSI-to-HTML conversion, and bounded virtual-screen rendering.

It is not a complete terminal emulator. See [Scope and limitations](#scope-and-limitations)
before using it as a compatibility layer for an interactive terminal.

## Why use it?

- Incremental parsing: ESC, CSI, OSC, and DCS sequences can cross arbitrary
  PTY, pipe, or network read boundaries.
- Structured events: callers receive raw bytes and lossless sequence fields,
  then may opt into typed CSI and OSC actions.
- Bounded control strings: CSI parameters, OSC payloads, and DCS payloads have
  fixed parser limits to avoid unbounded sequence accumulation.
- Useful layers above parsing: style spans, escaped HTML, screen snapshots,
  SVG rendering, ANSI screen diffs, and terminal-input encoding/decoding.
- Minimal production dependency surface: the library uses Rust `core` and
  `alloc`; `std` is only needed for the streaming I/O adapters.

## Installation

```toml
[dependencies]
nc-parser = "0.1"
```

For an `alloc`-based `no_std` target, disable default features:

```toml
[dependencies]
nc-parser = { version = "0.1", default-features = false, features = ["alloc"] }
```

The default `std` feature enables `AnsiReader`, `AnsiSpanReader`, and
`Parser::iter`.

## Core parser

`Parser` retains its state between `feed` calls. Sequence data is emitted only
when the sequence terminates, while printable text and C0 controls are emitted
as events.

```rust
use nc_parser::{Event, Parser};

let mut parser = Parser::new();

let first_events = parser.feed(b"hello \x1b[31");
assert!(matches!(first_events[0], Event::Print(_)));
let events = parser.feed(b"mred\x1b[0m");

assert!(matches!(events[0], Event::Csi(_)));
assert!(matches!(events[1], Event::Print(_)));
```

Use `feed_each` when events should be consumed immediately instead of collected:

```rust
use nc_parser::{Event, Parser};

let mut parser = Parser::new();
parser.feed_each(b"\x1b[1;31merror\x1b[0m", |event| {
    if let Event::Csi(sequence) = event {
        println!("CSI action: {:?}", sequence.action());
    }
});
```

`Event::Print` contains bytes, not a `String`. This preserves non-UTF-8 terminal
output. Convert with `String::from_utf8_lossy` only when lossy text is acceptable.

## Typed actions

The raw event model is lossless. Convenience methods interpret common sequences:

```rust
use nc_parser::{CsiAction, Event, Parser, SgrAction};

let mut parser = Parser::new();
let events = parser.feed(b"\x1b[1;38;5;196m");

let Event::Csi(csi) = &events[0] else { panic!("expected CSI") };
assert!(matches!(csi.action(), CsiAction::Sgr(actions)
    if actions.contains(&SgrAction::Bold)));
```

`OscSequence::action()` recognizes titles, OSC 7 working directories, OSC 8
hyperlinks, OSC 10/11 colors, and OSC 52 clipboard payloads. OSC actions are
untrusted data. Applications must enforce their own URL allowlist and user
consent policy before opening links or changing a clipboard.

## Headless screen and snapshots

`Screen` applies supported events to a fixed-size terminal grid. Use `feed` for
a complete byte slice, or call `process_event` with events from a persistent
`Parser` when terminal output arrives in chunks. Then inspect the active grid
or export a text/SVG snapshot.

```rust
use nc_parser::{Screen, to_snapshot};

let mut screen = Screen::new(24, 80);
screen.feed(b"\x1b[2J\x1b[1;1HStatus: \x1b[32mOK\x1b[0m");

assert_eq!(screen.line_to_string(0), "Status: OK");
assert_eq!(screen.active_grid().get(0, 8).unwrap().ch, 'O');
println!("{}", to_snapshot(&screen));
```

Use `ScreenDiff::compute` and `generate_ansi_diff` to construct a patch between
two same-sized screen states:

```rust
use nc_parser::{Screen, generate_ansi_diff};

let mut before = Screen::new(2, 20);
before.feed(b"starting");
let mut after = before.clone();
after.feed(b"\x1b[1;1Hready");

let patch = generate_ansi_diff(&before, &after);
before.feed(&patch);
assert_eq!(before.to_text(), after.to_text());
```

## Style spans and HTML

```rust
use nc_parser::{to_html, to_styled_spans};

let input = b"\x1b[1;31mAlert:\x1b[0m <check>";
let spans = to_styled_spans(input);
assert_eq!(spans[0].to_string_lossy(), "Alert:");

let html = to_html(input);
assert!(html.contains("font-weight:bold"));
assert!(html.contains("&lt;check&gt;"));
```

The HTML helper escapes text content. It does not execute OSC actions or create
hyperlinks from OSC 8 sequences.

## Supported protocol coverage

| Family | Parser behavior | Typed/high-level support |
|---|---|---|
| Printable text and C0 | Emits byte-preserving print and control events | Style tracking and screen handling for CR, LF, backspace, and tab |
| ESC | Emits `EscapeSequence` with intermediates and final byte | Screen handles DECSC, DECRC, and RIS |
| CSI | Emits parameter bytes, intermediates, and final byte | SGR; cursor movement/positioning; erase display/line; scroll; save/restore cursor; selected DEC private modes |
| OSC | Emits payload plus Bell/ST terminator | 0/2 titles, 7 CWD, 8 hyperlinks, 10/11 colors, 52 clipboard data |
| DCS | Emits header, payload, and terminator | Structured only; interpretation is caller-defined |
| 8-bit C1 | Disabled by default to avoid confusing UTF-8 continuation bytes | Enable with `Parser::with_8bit_c1(true)` |
| Terminal input | Encodes and decodes common input sequences | Keys, modified cursor keys, bracketed paste, focus, SGR mouse, and normal mouse |

## Resource limits and malformed input

The parser applies these bounds before storing sequence bytes:

| Data | Limit | Behavior at limit |
|---|---:|---|
| CSI parameter bytes | 256 bytes | Ignore through the terminating final byte |
| ESC/CSI intermediates | 16 bytes | Ignore malformed sequence |
| OSC payload | 4 KiB | Ignore through Bell or ST |
| DCS payload | 32 KiB | Ignore through Bell or ST |

Malformed and incomplete sequences do not produce partial sequence events. The
parser remains usable after a terminating byte or a cancellation control.

## Streaming I/O

With the default `std` feature, `AnsiReader<R: Read>` turns an I/O source into
incremental `Event` values. `AnsiSpanReader` applies `StyleTracker` while
reading. They retain parser state across read boundaries.

```rust,no_run
use nc_parser::{AnsiReader, Event};
use std::fs::File;

let mut reader = AnsiReader::new(File::open("build.log")?);
while let Some(event) = reader.next_event()? {
    if let Event::Print(bytes) = event {
        print!("{}", String::from_utf8_lossy(&bytes));
    }
}
# Ok::<(), std::io::Error>(())
```

## Scope and limitations

This crate deliberately does not currently provide full terminal-emulator
semantics. In particular, it has no scrollback buffer, resize protocol,
grapheme-cluster or East Asian-width handling, scrolling regions, character
insertion/deletion, terminal graphics protocols, or PTY management. Use a full
terminal-emulator core if those capabilities are required.

The public API is pre-1.0. Patch releases target compatible bug fixes; minor
releases may contain breaking API changes until downstream use and protocol
coverage justify a 1.0 stability promise.

## Development and verification

```bash
cargo test --all-features
cargo check --no-default-features --features alloc
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
```

The repository also contains libFuzzer targets under `fuzz/` and property tests
covering chunk boundaries, bounded parser states, serialization, screen safety,
and input decoding.

## License

Licensed under the [MIT License](LICENSE).
