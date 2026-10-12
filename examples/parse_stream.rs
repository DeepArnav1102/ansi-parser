use ansi_parser::{Event, Parser};

fn main() {
    let mut parser = Parser::new();

    let chunks: &[&[u8]] = &[b"Hello ", b"\x1b", b"[31", b"m", b"Red", b"\x1b[0m"];

    for chunk in chunks {
        for event in parser.feed(chunk) {
            match event {
                Event::Print(bytes) => {
                    println!("TEXT: {:?}", String::from_utf8_lossy(&bytes));
                }
                Event::Control(byte) => {
                    println!("CONTROL: {byte:#04x}");
                }
                Event::Csi(sequence) => {
                    println!(
                        "CSI: params={:?}, final={:?}",
                        String::from_utf8_lossy(&sequence.parameter_bytes),
                        sequence.final_byte as char
                    );
                }
                Event::Escape(sequence) => {
                    println!("ESCAPE: {:?}", sequence);
                }
                Event::Osc(sequence) => {
                    println!("OSC: {:?}", sequence);
                }
            }
        }
    }
}
