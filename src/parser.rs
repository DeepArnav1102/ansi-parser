use crate::{CsiSequence, EscapeSequence, Event};

const MAX_CSI_PARAMS: usize = 256;
const MAX_CSI_INTERMEDIATES: usize = 16;

#[derive(Debug, Clone)]
enum State {
    Ground,

    Escape {
        intermediates: Vec<u8>,
    },

    Csi {
        parameter_bytes: Vec<u8>,
        intermediates: Vec<u8>,
        saw_intermediate: bool,
    },

    CsiIgnore,
}

#[derive(Debug, Clone)]
pub struct Parser {
    state: State,
}

impl Parser {
    pub fn new() -> Self {
        Self {
            state: State::Ground,
        }
    }

    pub fn feed(&mut self, input: &[u8]) -> Vec<Event> {
        let mut events = Vec::new();
        let mut text = Vec::new();

        for &byte in input {
            let state = std::mem::replace(&mut self.state, State::Ground);

            match state {
                State::Ground => match byte {
                    0x1B => {
                        flush_text(&mut text, &mut events);
                        self.state = State::Escape {
                            intermediates: Vec::new(),
                        };
                    }

                    0x00..=0x1F => {
                        flush_text(&mut text, &mut events);
                        events.push(Event::Control(byte));
                    }

                    0x7F => {}

                    _ => text.push(byte),
                },

                State::Escape { mut intermediates } => match byte {
                    0x18 | 0x1A => {
                        flush_text(&mut text, &mut events);
                    }

                    0x1B => {
                        flush_text(&mut text, &mut events);
                        self.state = State::Escape {
                            intermediates: Vec::new(),
                        };
                    }

                    b'[' if intermediates.is_empty() => {
                        flush_text(&mut text, &mut events);
                        self.state = State::Csi {
                            parameter_bytes: Vec::new(),
                            intermediates: Vec::new(),
                            saw_intermediate: false,
                        };
                    }

                    0x20..=0x2F => {
                        if intermediates.len() < MAX_CSI_INTERMEDIATES {
                            intermediates.push(byte);
                            self.state = State::Escape { intermediates };
                        } else {
                            self.state = State::Ground;
                        }
                    }

                    0x30..=0x7E => {
                        flush_text(&mut text, &mut events);

                        events.push(Event::Escape(EscapeSequence {
                            intermediates,
                            final_byte: byte,
                        }));
                    }

                    _ => {
                        flush_text(&mut text, &mut events);
                    }
                },

                State::Csi {
                    mut parameter_bytes,
                    mut intermediates,
                    mut saw_intermediate,
                } => match byte {
                    0x18 | 0x1A => {
                        flush_text(&mut text, &mut events);
                    }

                    0x1B => {
                        flush_text(&mut text, &mut events);
                        self.state = State::Escape {
                            intermediates: Vec::new(),
                        };
                    }

                    0x30..=0x3F if !saw_intermediate => {
                        if parameter_bytes.len() < MAX_CSI_PARAMS {
                            parameter_bytes.push(byte);
                            self.state = State::Csi {
                                parameter_bytes,
                                intermediates,
                                saw_intermediate,
                            };
                        } else {
                            self.state = State::CsiIgnore;
                        }
                    }

                    0x20..=0x2F => {
                        saw_intermediate = true;

                        if intermediates.len() < MAX_CSI_INTERMEDIATES {
                            intermediates.push(byte);
                            self.state = State::Csi {
                                parameter_bytes,
                                intermediates,
                                saw_intermediate,
                            };
                        } else {
                            self.state = State::CsiIgnore;
                        }
                    }

                    0x40..=0x7E => {
                        flush_text(&mut text, &mut events);

                        events.push(Event::Csi(CsiSequence {
                            parameter_bytes,
                            intermediates,
                            final_byte: byte,
                        }));
                    }

                    0x00..=0x1F => {
                        flush_text(&mut text, &mut events);
                        events.push(Event::Control(byte));

                        self.state = State::Csi {
                            parameter_bytes,
                            intermediates,
                            saw_intermediate,
                        };
                    }

                    _ => {
                        self.state = State::CsiIgnore;
                    }
                },

                State::CsiIgnore => match byte {
                    0x18 | 0x1A => {}

                    0x1B => {
                        self.state = State::Escape {
                            intermediates: Vec::new(),
                        };
                    }

                    0x40..=0x7E => {}

                    _ => {
                        self.state = State::CsiIgnore;
                    }
                },
            }
        }

        flush_text(&mut text, &mut events);

        events
    }
}

impl Default for Parser {
    fn default() -> Self {
        Self::new()
    }
}

fn flush_text(text: &mut Vec<u8>, events: &mut Vec<Event>) {
    if !text.is_empty() {
        events.push(Event::Print(std::mem::take(text)));
    }
}
