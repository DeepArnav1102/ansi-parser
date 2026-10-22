use crate::{CsiSequence, DcsSequence, EscapeSequence, Event, OscSequence, OscTerminator};
use alloc::vec::Vec;

const MAX_CSI_PARAMS: usize = 256;
const MAX_CSI_INTERMEDIATES: usize = 16;
const MAX_OSC_PAYLOAD: usize = 4096;
const MAX_DCS_PAYLOAD: usize = 32768;

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

    OscString {
        payload: Vec<u8>,
        saw_escape: bool,
    },

    OscIgnore {
        saw_escape: bool,
    },

    DcsEntry {
        parameter_bytes: Vec<u8>,
        intermediates: Vec<u8>,
        saw_intermediate: bool,
    },

    DcsString {
        parameter_bytes: Vec<u8>,
        intermediates: Vec<u8>,
        final_byte: u8,
        payload: Vec<u8>,
        saw_escape: bool,
    },

    DcsIgnore {
        saw_escape: bool,
    },
}

#[derive(Debug, Clone)]
pub struct Parser {
    state: State,
    enable_8bit_c1: bool,
}

impl Parser {
    /// Create a new incremental parser in ground state.
    pub fn new() -> Self {
        Self {
            state: State::Ground,
            enable_8bit_c1: false,
        }
    }

    /// Enable or disable raw 8-bit C1 control sequences (0x80..=0x9F).
    ///
    /// By default, this is disabled (`false`) because modern terminal streams are UTF-8,
    /// where bytes in 0x80..=0x9F are valid UTF-8 continuation bytes that should not be
    /// misidentified as C1 controls.
    pub fn with_8bit_c1(mut self, enabled: bool) -> Self {
        self.enable_8bit_c1 = enabled;
        self
    }

    /// Set 8-bit C1 controls enabled flag.
    pub fn set_8bit_c1(&mut self, enabled: bool) {
        self.enable_8bit_c1 = enabled;
    }

    /// Incremental zero-allocation feeding: processes input and invokes `callback`
    /// for every parsed [`Event`].
    pub fn feed_each<F>(&mut self, input: &[u8], mut callback: F)
    where
        F: FnMut(Event),
    {
        let mut text = Vec::new();

        for &byte in input {
            let state = core::mem::replace(&mut self.state, State::Ground);

            match state {
                State::Ground => {
                    if self.enable_8bit_c1 {
                        match byte {
                            0x90 => {
                                flush_text(&mut text, &mut callback);
                                self.state = State::DcsEntry {
                                    parameter_bytes: Vec::new(),
                                    intermediates: Vec::new(),
                                    saw_intermediate: false,
                                };
                                continue;
                            }
                            0x9B => {
                                flush_text(&mut text, &mut callback);
                                self.state = State::Csi {
                                    parameter_bytes: Vec::new(),
                                    intermediates: Vec::new(),
                                    saw_intermediate: false,
                                };
                                continue;
                            }
                            0x9D => {
                                flush_text(&mut text, &mut callback);
                                self.state = State::OscString {
                                    payload: Vec::new(),
                                    saw_escape: false,
                                };
                                continue;
                            }
                            _ => {}
                        }
                    }

                    match byte {
                        0x1B => {
                            flush_text(&mut text, &mut callback);
                            self.state = State::Escape {
                                intermediates: Vec::new(),
                            };
                        }

                        0x00..=0x1F => {
                            flush_text(&mut text, &mut callback);
                            callback(Event::Control(byte));
                        }

                        0x7F => {}

                        _ => text.push(byte),
                    }
                }

                State::Escape { mut intermediates } => match byte {
                    0x18 | 0x1A => {
                        flush_text(&mut text, &mut callback);
                    }

                    0x1B => {
                        flush_text(&mut text, &mut callback);
                        self.state = State::Escape {
                            intermediates: Vec::new(),
                        };
                    }

                    b'[' if intermediates.is_empty() => {
                        flush_text(&mut text, &mut callback);
                        self.state = State::Csi {
                            parameter_bytes: Vec::new(),
                            intermediates: Vec::new(),
                            saw_intermediate: false,
                        };
                    }

                    b']' if intermediates.is_empty() => {
                        flush_text(&mut text, &mut callback);
                        self.state = State::OscString {
                            payload: Vec::new(),
                            saw_escape: false,
                        };
                    }

                    b'P' if intermediates.is_empty() => {
                        flush_text(&mut text, &mut callback);
                        self.state = State::DcsEntry {
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
                        flush_text(&mut text, &mut callback);

                        callback(Event::Escape(EscapeSequence {
                            intermediates,
                            final_byte: byte,
                        }));
                    }

                    _ => {
                        flush_text(&mut text, &mut callback);
                    }
                },

                State::Csi {
                    mut parameter_bytes,
                    mut intermediates,
                    mut saw_intermediate,
                } => match byte {
                    0x18 | 0x1A => {
                        flush_text(&mut text, &mut callback);
                    }

                    0x1B => {
                        flush_text(&mut text, &mut callback);
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
                        flush_text(&mut text, &mut callback);

                        callback(Event::Csi(CsiSequence {
                            parameter_bytes,
                            intermediates,
                            final_byte: byte,
                        }));
                    }

                    0x00..=0x1F => {
                        flush_text(&mut text, &mut callback);
                        callback(Event::Control(byte));

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

                State::OscString {
                    mut payload,
                    saw_escape,
                } => {
                    if saw_escape && byte == b'\\' {
                        callback(Event::Osc(OscSequence {
                            payload,
                            terminator: OscTerminator::St,
                        }));
                    } else if byte == 0x07 {
                        if saw_escape {
                            if payload.len() >= MAX_OSC_PAYLOAD {
                                self.state = State::OscIgnore { saw_escape: false };
                                continue;
                            }
                            payload.push(0x1B);
                        }

                        callback(Event::Osc(OscSequence {
                            payload,
                            terminator: OscTerminator::Bell,
                        }));
                    } else if byte == 0x1B {
                        if saw_escape {
                            if payload.len() >= MAX_OSC_PAYLOAD {
                                self.state = State::OscIgnore { saw_escape: true };
                                continue;
                            }
                            payload.push(0x1B);
                        }

                        self.state = State::OscString {
                            payload,
                            saw_escape: true,
                        };
                    } else {
                        if saw_escape {
                            if payload.len() >= MAX_OSC_PAYLOAD {
                                self.state = State::OscIgnore { saw_escape: false };
                                continue;
                            }
                            payload.push(0x1B);
                        }

                        if payload.len() >= MAX_OSC_PAYLOAD {
                            self.state = State::OscIgnore { saw_escape: false };
                        } else {
                            payload.push(byte);

                            self.state = State::OscString {
                                payload,
                                saw_escape: false,
                            };
                        }
                    }
                }

                State::OscIgnore { saw_escape } => {
                    if (saw_escape && byte == b'\\') || byte == 0x07 {
                        self.state = State::Ground;
                    } else {
                        self.state = State::OscIgnore {
                            saw_escape: byte == 0x1B,
                        };
                    }
                }

                State::DcsEntry {
                    mut parameter_bytes,
                    mut intermediates,
                    mut saw_intermediate,
                } => match byte {
                    0x18 | 0x1A => {
                        flush_text(&mut text, &mut callback);
                    }

                    0x1B => {
                        flush_text(&mut text, &mut callback);
                        self.state = State::Escape {
                            intermediates: Vec::new(),
                        };
                    }

                    0x30..=0x3F if !saw_intermediate => {
                        if parameter_bytes.len() < MAX_CSI_PARAMS {
                            parameter_bytes.push(byte);
                            self.state = State::DcsEntry {
                                parameter_bytes,
                                intermediates,
                                saw_intermediate,
                            };
                        } else {
                            self.state = State::DcsIgnore { saw_escape: false };
                        }
                    }

                    0x20..=0x2F => {
                        saw_intermediate = true;

                        if intermediates.len() < MAX_CSI_INTERMEDIATES {
                            intermediates.push(byte);
                            self.state = State::DcsEntry {
                                parameter_bytes,
                                intermediates,
                                saw_intermediate,
                            };
                        } else {
                            self.state = State::DcsIgnore { saw_escape: false };
                        }
                    }

                    0x40..=0x7E => {
                        flush_text(&mut text, &mut callback);

                        self.state = State::DcsString {
                            parameter_bytes,
                            intermediates,
                            final_byte: byte,
                            payload: Vec::new(),
                            saw_escape: false,
                        };
                    }

                    _ => {
                        self.state = State::DcsIgnore { saw_escape: false };
                    }
                },

                State::DcsString {
                    parameter_bytes,
                    intermediates,
                    final_byte,
                    mut payload,
                    saw_escape,
                } => {
                    if saw_escape && byte == b'\\' {
                        callback(Event::Dcs(DcsSequence {
                            parameter_bytes,
                            intermediates,
                            final_byte,
                            payload,
                            terminator: OscTerminator::St,
                        }));
                    } else if byte == 0x07 {
                        if saw_escape {
                            if payload.len() >= MAX_DCS_PAYLOAD {
                                self.state = State::DcsIgnore { saw_escape: false };
                                continue;
                            }
                            payload.push(0x1B);
                        }

                        callback(Event::Dcs(DcsSequence {
                            parameter_bytes,
                            intermediates,
                            final_byte,
                            payload,
                            terminator: OscTerminator::Bell,
                        }));
                    } else if byte == 0x1B {
                        if saw_escape {
                            if payload.len() >= MAX_DCS_PAYLOAD {
                                self.state = State::DcsIgnore { saw_escape: true };
                                continue;
                            }
                            payload.push(0x1B);
                        }

                        self.state = State::DcsString {
                            parameter_bytes,
                            intermediates,
                            final_byte,
                            payload,
                            saw_escape: true,
                        };
                    } else {
                        if saw_escape {
                            if payload.len() >= MAX_DCS_PAYLOAD {
                                self.state = State::DcsIgnore { saw_escape: false };
                                continue;
                            }
                            payload.push(0x1B);
                        }

                        if payload.len() >= MAX_DCS_PAYLOAD {
                            self.state = State::DcsIgnore { saw_escape: false };
                        } else {
                            payload.push(byte);

                            self.state = State::DcsString {
                                parameter_bytes,
                                intermediates,
                                final_byte,
                                payload,
                                saw_escape: false,
                            };
                        }
                    }
                }

                State::DcsIgnore { saw_escape } => {
                    if (saw_escape && byte == b'\\') || byte == 0x07 {
                        self.state = State::Ground;
                    } else {
                        self.state = State::DcsIgnore {
                            saw_escape: byte == 0x1B,
                        };
                    }
                }
            }
        }

        flush_text(&mut text, &mut callback);
    }

    pub fn feed(&mut self, input: &[u8]) -> Vec<Event> {
        let mut events = Vec::new();
        self.feed_each(input, |event| events.push(event));
        events
    }

    /// Return an incremental pull-based iterator over [`Event`] items in `input`.
    #[cfg(feature = "std")]
    pub fn iter<'a>(&'a mut self, input: &'a [u8]) -> crate::stream::ParserIter<'a> {
        crate::stream::ParserIter::new(self, input)
    }
}

impl Default for Parser {
    fn default() -> Self {
        Self::new()
    }
}

fn flush_text<F>(text: &mut Vec<u8>, callback: &mut F)
where
    F: FnMut(Event),
{
    if !text.is_empty() {
        callback(Event::Print(core::mem::take(text)));
    }
}
