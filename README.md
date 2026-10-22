# ansi-parser

[![CI](https://github.com/arnav-deep/ansi-parser/actions/workflows/ci.yml/badge.svg)](https://github.com/arnav-deep/ansi-parser/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/ansi-parser.svg)](https://crates.io/crates/ansi-parser)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![no_std compatible](https://img.shields.io/badge/no__std-compatible-success.svg)](https://docs.rs/ansi-parser)

A blazing fast, zero-dependency, streaming ANSI escape sequence parser and virtual terminal toolkit in pure Rust.

Designed for high-throughput terminal multiplexers, TUIs, headless CLI testing, terminal recording/playback, and embedded (`#![no_std]`) environments.

---

## ✨ Features at a Glance

- **Zero Runtime Dependencies**: Strict commitment to 100% zero third-party dependencies (strictly Rust `core` and `alloc`).
- **`#![no_std]` & Embedded Ready**: First-class support for bare-metal targets (e.g., `thumbv7em`, WASM) via feature flags (`default = ["std"]`).
- **Streaming & Chunk-Invariant**: Correctly parses control sequences split arbitrarily across TCP packets, Unix PTY chunks, or pipe boundaries.
- **Rich CSI Actions**: Full ECMA-48 CSI decoding (SGR colors 16/256/TrueColor 24-bit RGB with colon and semicolon formats, cursor jumps, line/screen erasures, DEC private modes).
- **OSC Dispatch & Zero-Dep Base64**: Parses OSC window titles, OSC 7 working directories, OSC 8 hyperlinks, OSC 10/11 color queries, and OSC 52 clipboard sequences (with built-in base64 decoding).
- **ECMA-48 DCS Engine**: Full Device Control String parsing (`ESC P`) with DoS payload limits (32 KiB cap) and safe opt-in 8-bit C1 control support.
- **Visual Style Tracking & HTML Export**: Tracks active text styling bitmasks and colors across stream boundaries, coalescing adjacent text spans and rendering clean CSS/HTML.
- **Virtual Terminal Screen Grid**: Complete 2D headless virtual screen buffer (`Screen`, `TerminalGrid`) supporting scrolling, cursor coordinates, and DECSET 1049 alternate screen buffer switching.
- **Screen Diffing & Bandwidth-Optimal Patching**: Calculates frame-by-frame screen deltas (`ScreenDiff`) and generates bandwidth-minimized ANSI escape patches. Exports standalone vector SVG screenshots and text snapshots.
- **Bi-directional Input Protocol**: Encodes and decodes keyboard keys, modifiers, SGR 1006 mouse events, focus events, and bracketed paste mode.
- **Battle-Tested & Fuzzed**: Fuzzed against Google's `libFuzzer` and verified with a 15-property `proptest` invariant suite across millions of executions.

---

## 📦 Installation

Add `ansi-parser` to your `Cargo.toml`:

```toml
[dependencies]
ansi-parser = "0.1.0"
```

For embedded or `#![no_std]` environments without `std`:

```toml
[dependencies]
ansi-parser = { version = "0.1.0", default-features = false, features = ["alloc"] }
```

---

## 🚀 Quick Start Examples

### 1. Streaming Parser with Zero Allocations (`feed_each`)

```rust
use ansi_parser::{Parser, Event};

let mut parser = Parser::new();
let input = b"Hello, \x1b[31;1mBold Red\x1b[0m World!\n";

parser.feed_each(input, |event| {
    match event {
        Event::Print(text) => print!("{}", String::from_utf8_lossy(&text)),
        Event::Csi(csi) => println!("CSI Action: {:?}", csi.action()),
        Event::Control(b) => println!("Control byte: 0x{:02X}", b),
        _ => {}
    }
});
```

### 2. Stripping ANSI Sequences

```rust
use ansi_parser::{strip_ansi, strip_ansi_to_string};

let clean_str = strip_ansi_to_string("\x1b[32mSuccess:\x1b[0m Job finished!");
assert_eq!(clean_str, "Success: Job finished!");
```

### 3. Headless Virtual Terminal Screen & Grid

```rust
use ansi_parser::Screen;

let mut screen = Screen::new(24, 80);
screen.feed(b"\x1b[2J\x1b[1;1HHeader Line\x1b[3;5H[ Status: OK ]");

// Inspect cell at row 2 (0-indexed), col 4
let cell = screen.grid().get(2, 4).unwrap();
assert_eq!(cell.ch, '[');

// Export screen state as bordered text or standalone SVG
let snapshot = ansi_parser::to_snapshot(&screen);
let svg = ansi_parser::to_svg(&screen);
```

### 4. Screen Diffing & Minimal ANSI Patch Generation

```rust
use ansi_parser::{Screen, generate_ansi_diff};

let mut frame1 = Screen::new(24, 80);
frame1.feed(b"Initial frame text");

let mut frame2 = frame1.clone();
frame2.feed(b"\x1b[1;1HUpdated");

// Generates the minimal byte sequence needed to transition from frame1 to frame2
let patch_bytes = generate_ansi_diff(&frame1, &frame2);

let mut replayed = frame1.clone();
replayed.feed(&patch_bytes);
assert_eq!(replayed.to_text(), frame2.to_text());
```

### 5. Bi-directional Terminal Input Protocol

```rust
use ansi_parser::{encode_key, decode_input_sequence, KeyCode, Modifiers, InputEvent};

// Encode Shift+Up Arrow key
let bytes = encode_key(KeyCode::Up, Modifiers::SHIFT);
assert_eq!(bytes, b"\x1b[1;2A");

// Decode received stdin bytes
let (event, consumed) = decode_input_sequence(&bytes).unwrap();
assert_eq!(consumed, 6);
assert_eq!(event, InputEvent::Key { key: KeyCode::Up, modifiers: Modifiers::SHIFT });
```

---

## ⚡ Performance

Benchmarks measured on x86_64:

| Benchmark | Payload Size | Throughput |
|---|---|---|
| `feed_each` (Plain text) | 67 KB | **~60 MB/s** |
| `feed_each` (High-density ANSI SGR) | 48 KB | **~15 MB/s** |
| `strip_ansi` (Mixed ANSI) | 48 KB | **~25 MB/s** |
| `screen_diff_compute` | 80x24 (1920 cells) | **~90 µs / frame** |
| `generate_ansi_diff` | 80x24 (1920 cells) | **~85 µs / patch** |

Run benchmarks yourself:
```bash
cargo bench --bench parser_bench
```

---

## 🛡 Security & Verification

- **DoS-Resistant Buffers**: Hard ceilings on parameter and payload allocations (256 CSI params, 4 KB OSC payload, 32 KB DCS payload). Oversized sequences enter safe ignore states with zero memory allocation.
- **libFuzzer Targets**: 5 continuous fuzz targets under `fuzz/` covering chunk fragmentation, screen grids, input decoding, and ANSI stripping.
- **Proptest Invariant Suite**: 15 property-based tests verifying chunk invariance, round-trip serialization, and bounded execution over arbitrary byte sequences.

---

## 📜 License

Licensed under the [MIT License](LICENSE).
