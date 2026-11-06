//! The writer of the adapter: offset, start_of, end_of, unread and more.

use super::super::constants::{DIGIT_SEPARATOR, NESTING};
use super::super::schemas::scalar_kind;
use super::super::types::{Decoded, Decoding, HeldSpace, Opened, Part, Scratch, TableReadPhase};
use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use ryo::languages::{self, Count, NameText, Offset};
use toml_parser::decoder::{Encoding, ScalarKind};
use toml_parser::{ParseError, Raw, Source, Span};

/// The offset of a byte index of the text.
pub(in super::super) fn offset(index: Count) -> Offset {
    Offset(u32::try_from(index.0).unwrap_or(u32::MAX))
}

/// Where a span of the parser starts.
pub(in super::super) fn start_of(span: Span) -> Offset {
    offset(Count(span.start()))
}

/// Where a span of the parser ends.
pub(in super::super) fn end_of(span: Span) -> Offset {
    offset(Count(span.end()))
}

/// Writes the text the parser gave no event for (it drops the lone `]` of an array table left open) as text it could not read: the
/// gap tokenizer of the tree, so that the tokens read back the whole file.
pub(in super::super) fn unread(
    builder: &mut languages::TableBuilder<'_>,
    from: Offset,
    to: Offset,
) {
    languages::token(builder, kind_bits(Kind::Extra(Extra::Error)), from, to);
}

/// The receiver of the parser's events: it writes them into the table as nodes and tokens.
pub(in super::super) struct Writer<'b, 'w, 's> {
    /// The tree, and where its last token ends.
    pub(in super::super) tree: languages::Lossless<'b, 'w>,
    /// The text parsed, which an unquoted scalar's bytes are read from to decode it.
    pub(in super::super) source: Source<'s>,
    /// The constructs open, the document first.
    pub(in super::super) open: Vec<Kind>,
    /// White space and text the parser could not read, met after a part of a key.
    pub(in super::super) held: Vec<HeldSpace>,
    /// Whether a header is being read, so a key opens no key-value.
    pub(in super::super) reading: TableReadPhase,
    /// An empty token that waits for the next one.
    pub(in super::super) empty: Option<HeldSpace>,
    /// How many arrays and inline tables are open.
    pub(in super::super) depth: Count,
    /// How many the parser was told not to open, whose closing it still gives.
    pub(in super::super) refused: Count,
    /// The text the decoder writes into, kept for the file.
    pub(in super::super) scratch: Scratch,
    /// What the library's decoder refused, which its recoverable parse leaves to the value's reader.
    pub(in super::super) refusals: Vec<ParseError>,
}

impl Writer<'_, '_, '_> {
    /// The kind of the innermost construct open: the document when none other is.
    pub(in super::super) fn top(&self) -> Kind {
        self.open.last().copied().unwrap_or(Kind::Node(Node::Toml))
    }

    /// Opens a construct at `at`, after any text before it the parser gave no event for: that text is not the construct's.
    pub(in super::super) fn enter(&mut self, kind: Kind, at: Offset) {
        languages::lossless_enter(&mut self.tree, kind_bits(kind), at, &mut unread);
        self.open.push(kind);
    }

    /// Closes the innermost construct open, after the tokens it holds; the document stays open.
    pub(in super::super) fn leave(&mut self) {
        self.flush();
        if self.open.len() > 1 {
            let _: Option<Kind> = self.open.pop();
            let end = self.tree.end;
            languages::lossless_leave(&mut self.tree, end, &mut unread);
        }
    }

    /// Writes a token of `kind` over `span`.
    pub(in super::super) fn token(&mut self, kind: Kind, span: Span) {
        self.write(HeldSpace {
            kind,
            start: start_of(span),
            end: end_of(span),
        });
    }

    /// Writes a token. The parser stands in for what is missing (a closing bracket) with an empty token where it noticed, and may
    /// give the text it could not read there afterwards: an empty token waits for the next one, so the tokens are written in the
    /// order of the file.
    pub(in super::super) fn write(&mut self, token: HeldSpace) {
        if token.start == token.end {
            self.flush();
            self.empty = Some(token);
            return;
        }
        if let Some(empty) = self.empty
            && empty.start <= token.start
        {
            self.flush();
        }
        self.put(token);
    }

    /// Writes the empty token that waits, where the tree has got to.
    pub(in super::super) fn flush(&mut self) {
        if let Some(mut empty) = self.empty.take() {
            empty.start = empty.start.max(self.tree.end);
            empty.end = empty.start;
            self.put(empty);
        }
    }

    /// Writes a token, and before it any text the parser gave no event for.
    pub(in super::super) fn put(&mut self, token: HeldSpace) {
        languages::lossless_token(
            &mut self.tree,
            kind_bits(token.kind),
            token.start,
            token.end,
            &mut unread,
        );
    }

    /// Writes the white space and comments held after a part of a key.
    pub(in super::super) fn release(&mut self) {
        for each in std::mem::take(&mut self.held) {
            self.write(each);
        }
    }

    /// Closes the key, and the key-value it is in, when the line or the entry ends before its value.
    pub(in super::super) fn close_entry(&mut self) {
        if self.top() == Kind::Node(Node::Key) {
            self.leave();
            self.release();
        }
        if self.top() == Kind::Node(Node::Keyval) {
            self.leave();
        }
    }

    /// Closes everything below the document.
    pub(in super::super) fn close_all(&mut self) {
        self.release();
        while self.open.len() > 1 {
            self.leave();
        }
    }

    /// Closes the key-value whose value has just ended.
    pub(in super::super) fn value_ended(&mut self) {
        if self.top() == Kind::Node(Node::Keyval) {
            self.leave();
        }
    }

    /// Writes white space or text the parser could not read, or holds it when it comes after a part of a key, whose key may go on.
    pub(in super::super) fn trivia(&mut self, kind: Kind, span: Span) {
        if self.top() == Kind::Node(Node::Key) {
            self.held.push(HeldSpace {
                kind,
                start: start_of(span),
                end: end_of(span),
            });
        } else {
            self.token(kind, span);
        }
    }

    /// Opens a header of `kind` at its `bracket`, closing every construct open before it.
    pub(in super::super) fn header(&mut self, kind: Kind, bracket: Kind, span: Span) {
        self.close_all();
        self.enter(kind, start_of(span));
        self.token(bracket, span);
        self.reading = TableReadPhase::Header;
    }

    /// Writes the bracket of `kind` that ends a header, after closing its key.
    pub(in super::super) fn header_closed(&mut self, kind: Kind, span: Span) {
        if self.top() == Kind::Node(Node::Key) {
            self.leave();
        }
        self.release();
        self.token(kind, span);
        self.reading = TableReadPhase::Entries;
    }

    /// Opens an array or an inline table, or, past the nesting the library reads, writes its opening as text the parser could not
    /// read and tells the parser to stop.
    pub(in super::super) fn nest(&mut self, kind: Kind, bracket: Kind, span: Span) -> Opened {
        if self.depth.0 >= NESTING.0 {
            self.refused.0 += 1;
            self.token(Kind::Extra(Extra::Error), span);
            return Opened::Refused;
        }
        self.depth.0 += 1;
        self.enter(kind, start_of(span));
        self.token(bracket, span);
        Opened::Opened
    }

    /// Closes an array or an inline table.
    pub(in super::super) fn unnest(&mut self, kind: Kind, bracket: Kind, span: Span) {
        if self.refused.0 > 0 {
            self.refused.0 -= 1;
            self.token(Kind::Extra(Extra::Error), span);
            return;
        }
        self.close_entry();
        self.token(bracket, span);
        if self.top() == kind {
            self.leave();
            self.depth.0 = self.depth.0.saturating_sub(1);
        }
        self.value_ended();
    }

    /// Decodes the key or string of `span` with the library's decoder, and records what it decoded when it read it cleanly.
    pub(in super::super) fn decode(
        &mut self,
        span: Span,
        encoding: Option<Encoding>,
        part: Part,
    ) -> Decoding {
        let text = self
            .source
            .input()
            .get(span.start()..span.end())
            .unwrap_or_default();
        let raw = Raw::new_unchecked(text, encoding, span);
        let mut said = Decoded::Clean;
        let refusals = &mut self.refusals;
        let mut refused = |error: ParseError| {
            said = Decoded::Refused;
            refusals.push(error);
        };
        self.scratch.0.clear();
        let classified = if part == Part::Key {
            raw.decode_key(&mut self.scratch.0, &mut refused);
            ScalarKind::String
        } else {
            raw.decode_scalar(&mut self.scratch.0, &mut refused)
        };
        Decoding { classified, said }
    }

    /// Writes the token of a quoted key or string the decoder read cleanly, with the text it decoded, or else as a plain token.
    pub(in super::super) fn decoded(&mut self, kind: Kind, span: Span, said: Decoded) {
        if said == Decoded::Refused || span.is_empty() {
            self.token(kind, span);
            return;
        }
        let (start, end) = (start_of(span), end_of(span));
        if let Some(empty) = self.empty
            && empty.start <= start
        {
            self.flush();
        }
        languages::lossless_decoded_token(
            &mut self.tree,
            kind_bits(kind),
            start,
            end,
            NameText(&self.scratch.0),
            &mut unread,
        );
    }

    /// Writes an unquoted scalar: a boolean, an integer, a float or a date-time the decoder read cleanly with the value it decoded,
    /// the boolean as `true` or `false`, the integer in decimal, read from the digits the decoder kept in their radix as the `toml`
    /// crate reads them (`i64::from_str_radix`), the float in the shortest decimal form that reads back as it, from its digits without
    /// their underscores as the `toml` crate reads them, and the date-time as RFC 3339 writes it, as `toml_datetime` reads and writes
    /// it; an integer past the range TOML allows, a scalar the library does not read, and a string, as a plain token.
    pub(in super::super) fn scalar_value(
        &mut self,
        classified: ScalarKind,
        span: Span,
        said: Decoded,
    ) {
        let kind = scalar_kind(classified);
        let written = self
            .source
            .input()
            .get(span.start()..span.end())
            .unwrap_or_default();
        let value = match classified {
            ScalarKind::Boolean(value) => Some(value.to_string()),
            // an integer TOML refuses (out of range) has no value: the token keeps its text and gets no decoded value
            ScalarKind::Integer(radix) => {
                match i64::from_str_radix(&self.scratch.0, radix.value()) {
                    Ok(value) => Some(value.to_string()),
                    Err(_) => None,
                }
            }
            // a float that does not read has no value: the token keeps its text and gets no decoded value
            ScalarKind::Float => {
                let digits = written
                    .chars()
                    .filter(|each| *each != DIGIT_SEPARATOR)
                    .collect::<String>();
                match digits.parse::<f64>() {
                    Ok(value) => Some(format!("{value:?}")),
                    Err(_) => None,
                }
            }
            // a date-time that does not read has no value: the token keeps its text and gets no decoded value
            ScalarKind::DateTime => match written.parse::<toml::value::Datetime>() {
                Ok(value) => Some(value.to_string()),
                Err(_) => None,
            },
            ScalarKind::String => None,
        };
        match value {
            Some(value) if said == Decoded::Clean => {
                self.scratch.0 = value;
                self.decoded(kind, span, said);
            }
            Some(_) | None => self.token(kind, span),
        }
    }
}
