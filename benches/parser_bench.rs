//! Zero-dependency micro-benchmarks for ansi-parser.
//!
//! Measures parser throughput, ANSI stripping, screen virtualization,
//! and delta diff generation across various workloads.

use ansi_parser::{
    generate_ansi_diff, strip_ansi, Event, Parser, Screen,
};
use std::time::Instant;

fn bench<F>(name: &str, bytes_processed: usize, mut f: F)
where
    F: FnMut(),
{
    // Warmup
    for _ in 0..5 {
        f();
    }

    let iterations = 200;
    let start = Instant::now();
    for _ in 0..iterations {
        f();
    }
    let elapsed = start.elapsed();
    let avg_dur = elapsed / iterations;
    let secs = avg_dur.as_secs_f64();
    let mb_per_sec = if secs > 0.0 && bytes_processed > 0 {
        (bytes_processed as f64 / (1024.0 * 1024.0)) / secs
    } else {
        0.0
    };

    println!(
        "bench {:<40} ... {:>10.2?} ({:>8.2} MB/s)",
        name, avg_dur, mb_per_sec
    );
}

fn main() {
    println!("Running ansi-parser micro-benchmarks...\n");

    // 1. Plain text benchmark (67 KB)
    let plain_text = "The quick brown fox jumps over the lazy dog.\n".repeat(1500);
    let plain_bytes = plain_text.as_bytes();

    bench("feed_each (plain text, 67 KB)", plain_bytes.len(), || {
        let mut parser = Parser::new();
        let mut count = 0;
        parser.feed_each(plain_bytes, |event| {
            if let Event::Print(t) = event {
                count += t.len();
            }
        });
        std::hint::black_box(count);
    });

    // 2. High-density ANSI SGR sequences (colors and styles)
    let mut ansi_text = String::new();
    for i in 0..1000 {
        ansi_text.push_str(&format!(
            "\x1b[38;2;{};{};{}m\x1b[48;5;{}m\x1b[1mSample line {}\x1b[0m\n",
            i % 256,
            (i * 3) % 256,
            (i * 7) % 256,
            i % 256,
            i
        ));
    }
    let ansi_bytes = ansi_text.as_bytes();

    bench("feed_each (high-density ANSI, 48 KB)", ansi_bytes.len(), || {
        let mut parser = Parser::new();
        let mut count = 0;
        parser.feed_each(ansi_bytes, |event| {
            if let Event::Csi(_) = event {
                count += 1;
            }
        });
        std::hint::black_box(count);
    });

    // 3. strip_ansi benchmark
    bench("strip_ansi (mixed ANSI, 48 KB)", ansi_bytes.len(), || {
        let stripped = strip_ansi(ansi_bytes);
        std::hint::black_box(stripped);
    });

    // 4. Screen virtualization feed benchmark
    bench("screen_feed (80x24 terminal stream)", ansi_bytes.len(), || {
        let mut screen = Screen::new(24, 80);
        screen.feed(ansi_bytes);
        std::hint::black_box(screen);
    });

    // 5. Screen diff & minimal ANSI patch benchmark
    let mut screen_a = Screen::new(24, 80);
    screen_a.feed(b"Hello World!\nInitial frame\nRow 3\nRow 4\n");
    let mut screen_b = screen_a.clone();
    screen_b.feed(b"\x1b[2;1HUpdated Frame Line\x1b[3;5H[OK]\x1b[10;1HNew bottom text");

    bench("screen_diff_compute (80x24)", 24 * 80, || {
        let diff = ansi_parser::ScreenDiff::compute(&screen_a, &screen_b);
        std::hint::black_box(diff);
    });

    bench("generate_ansi_diff (80x24)", 24 * 80, || {
        let patch = generate_ansi_diff(&screen_a, &screen_b);
        std::hint::black_box(patch);
    });

    println!("\nBenchmarks completed successfully.");
}
