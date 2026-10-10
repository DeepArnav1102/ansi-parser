# ansi-parser

An incremental ANSI and terminal escape-sequence parser written in Rust.

## Goals

- Parse terminal byte streams incrementally.
- Recognize ordinary output and terminal control sequences.
- Expose structured events through a documented API.
- Handle fragmented and malformed input predictably.
- Remain independent of terminal rendering and GUI frameworks.

## Status

Early development. The public API and supported sequence set
may change before the first stable release.

## License

MIT
