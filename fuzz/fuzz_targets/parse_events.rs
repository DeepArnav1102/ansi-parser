#![no_main]

use libfuzzer_sys::fuzz_target;

// Feed arbitrary bytes to the parser in one shot. Guards against panics,
// unbounded memory growth, and integer overflow in the state machine.
fuzz_target!(|data: &[u8]| {
    let mut parser = ansi_parser::Parser::new();
    for event in parser.feed(data) {
        match event {
            ansi_parser::Event::Osc(osc) => {
                assert!(osc.payload.len() <= 4096, "OSC payload limit violated");
            }
            ansi_parser::Event::Dcs(dcs) => {
                assert!(dcs.payload.len() <= 32768, "DCS payload limit violated");
            }
            ansi_parser::Event::Csi(csi) => {
                assert!(
                    csi.parameter_bytes.len() <= 256 && csi.intermediates.len() <= 16,
                    "CSI buffer limit violated"
                );
            }
            _ => {}
        }
    }
});
