use ansi_parser::{
    InputEvent, KeyCode, Modifiers, MouseAction, MouseButton, MouseEvent, MouseProtocol,
    decode_input_sequence, encode_key, encode_mouse, encode_paste,
};

#[test]
fn encodes_keys_and_modifiers() {
    // Plain Up
    assert_eq!(encode_key(KeyCode::Up, Modifiers::NONE), b"\x1b[A");

    // Shift + Up (mod param 2)
    assert_eq!(encode_key(KeyCode::Up, Modifiers::SHIFT), b"\x1b[1;2A");

    // Ctrl + Up (mod param 5)
    assert_eq!(encode_key(KeyCode::Up, Modifiers::CTRL), b"\x1b[1;5A");

    // Ctrl + C (ASCII ETX 0x03)
    assert_eq!(encode_key(KeyCode::Char('c'), Modifiers::CTRL), vec![0x03]);

    // Alt + a
    assert_eq!(encode_key(KeyCode::Char('a'), Modifiers::ALT), b"\x1ba");

    // Function keys
    assert_eq!(encode_key(KeyCode::F(1), Modifiers::NONE), b"\x1bOP");
    assert_eq!(encode_key(KeyCode::F(5), Modifiers::NONE), b"\x1b[15~");
}

#[test]
fn encodes_mouse_events() {
    let event = MouseEvent {
        col: 10,
        row: 5,
        button: MouseButton::Left,
        action: MouseAction::Press,
        modifiers: Modifiers::NONE,
    };

    // SGR 1006 protocol: \x1b[<0;10;5M
    assert_eq!(
        encode_mouse(event, MouseProtocol::Sgr),
        b"\x1b[<0;10;5M".to_vec()
    );

    // Release: \x1b[<0;10;5m
    let release_event = MouseEvent {
        action: MouseAction::Release,
        ..event
    };
    assert_eq!(
        encode_mouse(release_event, MouseProtocol::Sgr),
        b"\x1b[<0;10;5m".to_vec()
    );
}

#[test]
fn encodes_and_decodes_bracketed_paste() {
    let payload = "Hello World\nLine 2";
    let encoded = encode_paste(payload);
    assert!(encoded.starts_with(b"\x1b[200~"));
    assert!(encoded.ends_with(b"\x1b[201~"));

    let (decoded, consumed) = decode_input_sequence(&encoded).unwrap();
    assert_eq!(consumed, encoded.len());
    assert_eq!(decoded, InputEvent::Paste(payload.to_string()));
}

#[test]
fn decodes_various_input_sequences() {
    // Arrow Up
    assert_eq!(
        decode_input_sequence(b"\x1b[A"),
        Some((
            InputEvent::Key {
                key: KeyCode::Up,
                modifiers: Modifiers::NONE,
            },
            3
        ))
    );

    // Ctrl + Up
    assert_eq!(
        decode_input_sequence(b"\x1b[1;5A"),
        Some((
            InputEvent::Key {
                key: KeyCode::Up,
                modifiers: Modifiers::CTRL,
            },
            6
        ))
    );

    // F1
    assert_eq!(
        decode_input_sequence(b"\x1bOP"),
        Some((
            InputEvent::Key {
                key: KeyCode::F(1),
                modifiers: Modifiers::NONE,
            },
            3
        ))
    );

    // Focus Gained
    assert_eq!(
        decode_input_sequence(b"\x1b[I"),
        Some((InputEvent::FocusGained, 3))
    );

    // SGR Mouse Drag with Shift
    assert_eq!(
        decode_input_sequence(b"\x1b[<36;15;8M"),
        Some((
            InputEvent::Mouse(MouseEvent {
                col: 15,
                row: 8,
                button: MouseButton::Left,
                action: MouseAction::Drag,
                modifiers: Modifiers::SHIFT,
            }),
            11
        ))
    );
}
