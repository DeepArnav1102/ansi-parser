use crate::style::{StyleTracker, StyledSpan};
use crate::{Event, Parser};
use std::collections::VecDeque;
use std::io::{self, Read};

pub struct AnsiReader<R> {
    reader: R,
    parser: Parser,
    buffer: [u8; 8192],
    queue: VecDeque<Event>,
    reached_eof: bool,
}

impl<R: Read> AnsiReader<R> {
    /// Create a new ANSI streaming reader from an I/O source.
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            parser: Parser::new(),
            buffer: [0u8; 8192],
            queue: VecDeque::new(),
            reached_eof: false,
        }
    }

    pub fn next_event(&mut self) -> io::Result<Option<Event>> {
        loop {
            if let Some(event) = self.queue.pop_front() {
                return Ok(Some(event));
            }

            if self.reached_eof {
                return Ok(None);
            }

            let n = self.reader.read(&mut self.buffer)?;
            if n == 0 {
                self.reached_eof = true;
                return Ok(None);
            }

            self.parser
                .feed_each(&self.buffer[..n], |event| self.queue.push_back(event));
        }
    }
}

impl<R: Read> Iterator for AnsiReader<R> {
    type Item = io::Result<Event>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.next_event() {
            Ok(Some(ev)) => Some(Ok(ev)),
            Ok(None) => None,
            Err(e) => Some(Err(e)),
        }
    }
}

pub struct AnsiSpanReader<R> {
    inner: AnsiReader<R>,
    tracker: StyleTracker,
}

impl<R: Read> AnsiSpanReader<R> {
    /// Create a new styled span reader from an I/O source.
    pub fn new(reader: R) -> Self {
        Self {
            inner: AnsiReader::new(reader),
            tracker: StyleTracker::new(),
        }
    }

    /// Read the next styled text span from the stream.
    pub fn next_span(&mut self) -> io::Result<Option<StyledSpan>> {
        while let Some(event) = self.inner.next_event()? {
            if let Some(span) = self.tracker.process_event(&event) {
                return Ok(Some(span));
            }
        }
        Ok(None)
    }
}

impl<R: Read> Iterator for AnsiSpanReader<R> {
    type Item = io::Result<StyledSpan>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.next_span() {
            Ok(Some(span)) => Some(Ok(span)),
            Ok(None) => None,
            Err(e) => Some(Err(e)),
        }
    }
}

/// A pull-based iterator that streams [`Event`] tokens out of an in-memory byte slice.
pub struct ParserIter<'a> {
    parser: &'a mut Parser,
    chunks: &'a [u8],
    queue: VecDeque<Event>,
    fed: bool,
}

impl<'a> ParserIter<'a> {
    /// Create a new parser iterator over a byte slice.
    pub fn new(parser: &'a mut Parser, input: &'a [u8]) -> Self {
        Self {
            parser,
            chunks: input,
            queue: VecDeque::new(),
            fed: false,
        }
    }
}

impl<'a> Iterator for ParserIter<'a> {
    type Item = Event;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.fed {
            self.fed = true;
            self.parser
                .feed_each(self.chunks, |event| self.queue.push_back(event));
        }

        self.queue.pop_front()
    }
}
