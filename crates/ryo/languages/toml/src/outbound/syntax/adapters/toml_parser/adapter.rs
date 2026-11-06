// the adapter is the parser's event receiver, one callback for each event of the library, and the tree it writes from them; splitting it would make a second file no adapter template places
//! The syntax port, implemented with `toml_parser`, the parser of the `toml` crate, for TOML 1.1: its events are written straight
//! into the element table, with no document built. The parser gives a flat stream of events, every byte of the file in one of them;
//! the adapter opens a node for each construct the events open (a table, a key-value, a key, an array, an inline table) and closes it
//! where the construct ends. White space and comments between the parts of a key stay in the key; those after its last part are
//! written after the key is closed. Arrays and inline tables nest no deeper than the `toml` crate reads, and the parser is told to
//! stop there.
//!
//! What the library says of the document is its diagnostics: the `toml` crate's recoverable parse reads the same text and reports
//! every error of syntax and of meaning (a duplicate key, a table defined twice, an inline table extended, a date that is none), each
//! where it is, and a document it finds no error in is read as values, which says what is wrong with a number. The library's decoder reads every key and string; what it decodes cleanly is recorded for the readers of the file.
//! The library's names stop here; what leaves is the language's own `Kind`.

use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::Node;
use crate::domain::tokens::types::Token;
use ryo::languages::{self, Count, NameText, Offset};
use toml_parser::decoder::{Encoding, ScalarKind};
use toml_parser::parser::{EventKind, EventReceiver, parse_document};
use toml_parser::{ErrorSink, ParseError, Raw, Source, Span};

use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use crate::outbound::syntax::interface::SyntaxPort;
use crate::outbound::syntax::types::SourceText;

use super::constants::{DIGIT_SEPARATOR, NESTING, NOT_UTF8};
use super::schemas::{event_kind, reports, scalar_kind, string_kind};
use super::types::{Decoded, Decoding, HeldSpace, Opened, Part, TableReadPhase, Scratch, TomlText};

/// The `toml_parser` syntax adapter.
#[derive(Debug)]
pub(crate) struct SyntaxTomlParserAdapter;

/// The offset of a byte index of the text.
fn offset(index: Count) -> Offset {
    Offset(u32::try_from(index.0).unwrap_or(u32::MAX))
}

/// Where a span of the parser starts.
fn start_of(span: Span) -> Offset {
    offset(Count(span.start()))
}

/// Where a span of the parser ends.
fn end_of(span: Span) -> Offset {
    offset(Count(span.end()))
}

/// Writes the text the parser gave no event for (it drops the lone `]` of an array table left open) as text it could not read: the
/// gap tokenizer of the tree, so that the tokens read back the whole file.
fn unread(builder: &mut languages::TableBuilder<'_>, from: Offset, to: Offset) {
    languages::token(builder, kind_bits(Kind::Extra(Extra::Error)), from, to);
}

/// The receiver of the parser's events: it writes them into the table as nodes and tokens.
struct Writer<'b, 'w, 's> {
    /// The tree, and where its last token ends.
    tree: languages::Lossless<'b, 'w>,
    /// The text parsed, which an unquoted scalar's bytes are read from to decode it.
    source: Source<'s>,
    /// The constructs open, the document first.
    open: Vec<Kind>,
    /// White space and text the parser could not read, met after a part of a key.
    held: Vec<HeldSpace>,
    /// Whether a header is being read, so a key opens no key-value.
    reading: TableReadPhase,
    /// An empty token that waits for the next one.
    empty: Option<HeldSpace>,
    /// How many arrays and inline tables are open.
    depth: Count,
    /// How many the parser was told not to open, whose closing it still gives.
    refused: Count,
    /// The text the decoder writes into, kept for the file.
    scratch: Scratch,
    /// What the library's decoder refused, which its recoverable parse leaves to the value's reader.
    refusals: Vec<ParseError>,
}

impl Writer<'_, '_, '_> {
    /// The kind of the innermost construct open: the document when none other is.
    fn top(&self) -> Kind {
        self.open.last().copied().unwrap_or(Kind::Node(Node::Toml))
    }

    /// Opens a construct at `at`, after any text before it the parser gave no event for: that text is not the construct's.
    fn enter(&mut self, kind: Kind, at: Offset) {
        languages::lossless_enter(&mut self.tree, kind_bits(kind), at, &mut unread);
        self.open.push(kind);
    }

    /// Closes the innermost construct open, after the tokens it holds; the document stays open.
    fn leave(&mut self) {
        self.flush();
        if self.open.len() > 1 {
            let _: Option<Kind> = self.open.pop();
            let end = self.tree.end;
            languages::lossless_leave(&mut self.tree, end, &mut unread);
        }
    }

    /// Writes a token of `kind` over `span`.
    fn token(&mut self, kind: Kind, span: Span) {
        self.write(HeldSpace {
            kind,
            start: start_of(span),
            end: end_of(span),
        });
    }

    /// Writes a token. The parser stands in for what is missing (a closing bracket) with an empty token where it noticed, and may
    /// give the text it could not read there afterwards: an empty token waits for the next one, so the tokens are written in the
    /// order of the file.
    fn write(&mut self, token: HeldSpace) {
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
    fn flush(&mut self) {
        if let Some(mut empty) = self.empty.take() {
            empty.start = empty.start.max(self.tree.end);
            empty.end = empty.start;
            self.put(empty);
        }
    }

    /// Writes a token, and before it any text the parser gave no event for.
    fn put(&mut self, token: HeldSpace) {
        languages::lossless_token(
            &mut self.tree,
            kind_bits(token.kind),
            token.start,
            token.end,
            &mut unread,
        );
    }

    /// Writes the white space and comments held after a part of a key.
    fn release(&mut self) {
        for each in std::mem::take(&mut self.held) {
            self.write(each);
        }
    }

    /// Closes the key, and the key-value it is in, when the line or the entry ends before its value.
    fn close_entry(&mut self) {
        if self.top() == Kind::Node(Node::Key) {
            self.leave();
            self.release();
        }
        if self.top() == Kind::Node(Node::Keyval) {
            self.leave();
        }
    }

    /// Closes everything below the document.
    fn close_all(&mut self) {
        self.release();
        while self.open.len() > 1 {
            self.leave();
        }
    }

    /// Closes the key-value whose value has just ended.
    fn value_ended(&mut self) {
        if self.top() == Kind::Node(Node::Keyval) {
            self.leave();
        }
    }

    /// Writes white space or text the parser could not read, or holds it when it comes after a part of a key, whose key may go on.
    fn trivia(&mut self, kind: Kind, span: Span) {
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
    fn header(&mut self, kind: Kind, bracket: Kind, span: Span) {
        self.close_all();
        self.enter(kind, start_of(span));
        self.token(bracket, span);
        self.reading = TableReadPhase::Header;
    }

    /// Writes the bracket of `kind` that ends a header, after closing its key.
    fn header_closed(&mut self, kind: Kind, span: Span) {
        if self.top() == Kind::Node(Node::Key) {
            self.leave();
        }
        self.release();
        self.token(kind, span);
        self.reading = TableReadPhase::Entries;
    }

    /// Opens an array or an inline table, or, past the nesting the library reads, writes its opening as text the parser could not
    /// read and tells the parser to stop.
    fn nest(&mut self, kind: Kind, bracket: Kind, span: Span) -> Opened {
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
    fn unnest(&mut self, kind: Kind, bracket: Kind, span: Span) {
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
    fn decode(&mut self, span: Span, encoding: Option<Encoding>, part: Part) -> Decoding {
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
    fn decoded(&mut self, kind: Kind, span: Span, said: Decoded) {
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
    fn scalar_value(&mut self, classified: ScalarKind, span: Span, said: Decoded) {
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

impl EventReceiver for Writer<'_, '_, '_> {
    fn std_table_open(&mut self, span: Span, _error: &mut dyn ErrorSink) {
        self.header(
            Kind::Node(Node::StdTable),
            event_kind(EventKind::StdTableOpen),
            span,
        );
    }

    fn std_table_close(&mut self, span: Span, _error: &mut dyn ErrorSink) {
        self.header_closed(event_kind(EventKind::StdTableClose), span);
    }

    fn array_table_open(&mut self, span: Span, _error: &mut dyn ErrorSink) {
        self.header(
            Kind::Node(Node::ArrayTable),
            event_kind(EventKind::ArrayTableOpen),
            span,
        );
    }

    fn array_table_close(&mut self, span: Span, _error: &mut dyn ErrorSink) {
        self.header_closed(event_kind(EventKind::ArrayTableClose), span);
    }

    fn inline_table_open(&mut self, span: Span, _error: &mut dyn ErrorSink) -> bool {
        self.nest(
            Kind::Node(Node::InlineTable),
            event_kind(EventKind::InlineTableOpen),
            span,
        ) == Opened::Opened
    }

    fn inline_table_close(&mut self, span: Span, _error: &mut dyn ErrorSink) {
        self.unnest(
            Kind::Node(Node::InlineTable),
            event_kind(EventKind::InlineTableClose),
            span,
        );
    }

    fn array_open(&mut self, span: Span, _error: &mut dyn ErrorSink) -> bool {
        self.nest(
            Kind::Node(Node::Array),
            event_kind(EventKind::ArrayOpen),
            span,
        ) == Opened::Opened
    }

    fn array_close(&mut self, span: Span, _error: &mut dyn ErrorSink) {
        self.unnest(
            Kind::Node(Node::Array),
            event_kind(EventKind::ArrayClose),
            span,
        );
    }

    fn simple_key(&mut self, span: Span, encoding: Option<Encoding>, _error: &mut dyn ErrorSink) {
        let Decoding { said, .. } = self.decode(span, encoding, Part::Key);
        if self.top() == Kind::Node(Node::Key) {
            self.release();
        } else if self.reading == TableReadPhase::Header {
            self.enter(Kind::Node(Node::Key), start_of(span));
        } else {
            self.enter(Kind::Node(Node::Keyval), start_of(span));
            self.enter(Kind::Node(Node::Key), start_of(span));
        }
        match encoding {
            Some(quoted) => self.decoded(string_kind(quoted), span, said),
            None => self.token(event_kind(EventKind::SimpleKey), span),
        }
    }

    fn key_sep(&mut self, span: Span, _error: &mut dyn ErrorSink) {
        self.release();
        self.token(event_kind(EventKind::KeySep), span);
    }

    fn key_val_sep(&mut self, span: Span, _error: &mut dyn ErrorSink) {
        if self.top() == Kind::Node(Node::Key) {
            self.leave();
        }
        self.release();
        self.token(event_kind(EventKind::KeyValSep), span);
    }

    fn scalar(&mut self, span: Span, encoding: Option<Encoding>, _error: &mut dyn ErrorSink) {
        let Decoding { classified, said } = self.decode(span, encoding, Part::Value);
        // A value left out is an empty scalar to the parser: it is no value.
        if span.is_empty() {
            self.token(Kind::Extra(Extra::Error), span);
        } else if let Some(quoted) = encoding {
            self.decoded(string_kind(quoted), span, said);
        } else {
            self.scalar_value(classified, span, said);
        }
        self.value_ended();
    }

    fn value_sep(&mut self, span: Span, _error: &mut dyn ErrorSink) {
        self.close_entry();
        let separator = if self.top() == Kind::Node(Node::InlineTable) {
            Token::InlineTableSep
        } else {
            Token::ArraySep
        };
        self.token(Kind::Token(separator), span);
    }

    fn whitespace(&mut self, span: Span, _error: &mut dyn ErrorSink) {
        self.trivia(event_kind(EventKind::Whitespace), span);
    }

    fn comment(&mut self, span: Span, _error: &mut dyn ErrorSink) {
        self.close_entry();
        self.token(event_kind(EventKind::Comment), span);
    }

    fn newline(&mut self, span: Span, _error: &mut dyn ErrorSink) {
        self.close_entry();
        self.token(event_kind(EventKind::Newline), span);
    }

    fn error(&mut self, span: Span, _error: &mut dyn ErrorSink) {
        self.trivia(event_kind(EventKind::Error), span);
    }
}

impl SyntaxPort for SyntaxTomlParserAdapter {
    fn read(&self, text: SourceText<'_>, builder: &mut languages::TableBuilder<'_>) {
        // TOML is UTF-8: the library reads the text up to the first byte that is not, and the rest is text it could not read.
        let content = TomlText(languages::readable_text(languages::ByteSlice(text.0)).0);
        let source = Source::new(content.0);
        let tokens = source.lex().into_vec();
        languages::enter_node(builder, kind_bits(Kind::Node(Node::Toml)), Offset(0));
        let mut writer = Writer {
            tree: languages::lossless(builder, Offset(0)),
            source,
            open: vec![Kind::Node(Node::Toml)],
            held: Vec::new(),
            reading: TableReadPhase::Entries,
            empty: None,
            depth: Count(0),
            refused: Count(0),
            scratch: Scratch(String::new()),
            refusals: Vec::new(),
        };
        // The event pass builds the tree; what it reports, the recoverable parse below reports too, with what it means.
        parse_document(&tokens, &mut writer, &mut |_: ParseError| {});
        writer.close_all();
        writer.flush();
        let length = offset(Count(text.0.len()));
        languages::fill_to(&mut writer.tree, length, &mut unread);
        languages::leave_node(writer.tree.builder, length);
        if content.0.len() < text.0.len() {
            languages::diagnose(
                writer.tree.builder,
                languages::ReportText(NOT_UTF8),
                offset(Count(content.0.len())),
                length,
            );
        }
        for report in reports(content.0, &writer.refusals) {
            languages::diagnose(
                writer.tree.builder,
                languages::ReportText(&report.message.0),
                offset(report.start),
                offset(report.end),
            );
        }
    }
}
