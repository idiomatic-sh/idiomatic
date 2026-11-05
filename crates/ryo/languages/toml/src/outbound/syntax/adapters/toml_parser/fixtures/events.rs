//! The library's own events of a file, as the parser gives them: what the walk reads the library's tree from. `toml_parser` gives a
//! stream of events, every byte of the file in one of them; each is recorded with what it is to the constructs of TOML and the kind of
//! its token, the scalars as the library's decoder classifies them. Arrays and inline tables nest no deeper than the `toml` crate
//! reads, as the adapter tells the parser.

use ryo::languages::Offset;
use toml_parser::decoder::{Encoding, ScalarKind};
use toml_parser::parser::{EventReceiver, parse_document};
use toml_parser::{ErrorSink, ParseError, Raw, Source, Span};

use super::super::constants::NESTING;
use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::TomlNode;
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;

/// The token kind of a string, by the encoding the library gives it, named here and not through the adapter's schemas.
fn string_kind(encoding: Encoding) -> Kind {
    Kind::Token(match encoding {
        Encoding::BasicString => Token::BasicString,
        Encoding::LiteralString => Token::LiteralString,
        Encoding::MlBasicString => Token::MlBasicString,
        Encoding::MlLiteralString => Token::MlLiteralString,
    })
}

/// The kind of an unquoted scalar by the class the library's decoder gives it: an unquoted string is no value of TOML, so text the
/// parser could not read.
fn scalar_kind(class: ScalarKind) -> Kind {
    match class {
        ScalarKind::String => Kind::Extra(Extra::Error),
        ScalarKind::Boolean(_) => Kind::Token(Token::Boolean),
        ScalarKind::DateTime => Kind::Token(Token::DateTime),
        ScalarKind::Float => Kind::Token(Token::Float),
        ScalarKind::Integer(_) => Kind::Token(Token::Integer),
    }
}

/// What an event of the parser is to the constructs: the opening or closing of a header, or of an array or an inline table; a part
/// of a key, or the dot between two; the `=` of a key-value; a value; what ends an entry before its value (a comma, a comment or a
/// line end); or white space or text the parser could not read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Role {
    /// The `[` or `[[` of a header, opening the table of the kind it carries.
    HeaderOpen(Kind),
    /// The `]` or `]]` that ends a header.
    HeaderClose,
    /// The bracket or brace that opens an array or an inline table of the kind it carries.
    Open(Kind),
    /// The bracket or brace that closes an array or an inline table of the kind it carries.
    Close(Kind),
    /// One part of a key, bare or quoted.
    KeyPart,
    /// The `.` between two parts of a dotted key.
    KeyDot,
    /// The `=` between a key and its value.
    Equals,
    /// A scalar value.
    Value,
    /// What ends an entry before its value: a comma, a comment or a line end.
    Ends,
    /// White space, or text the parser could not read.
    Plain,
}

/// One event of the parser: its role, the kind of its token and its bytes.
#[derive(Clone, Copy, Debug)]
pub(super) struct Recorded {
    /// What the event is to the constructs of TOML.
    pub role: Role,
    /// The kind of the event's token.
    pub kind: Kind,
    /// The offset of the event's first byte.
    pub start: Offset,
    /// The offset just past the event's last byte.
    pub end: Offset,
}

/// The events as they come, with the nesting the parser is told to stop at.
struct Recorder<'s> {
    /// The text parsed, which an unquoted scalar's bytes are read from to classify it.
    source: Source<'s>,
    /// The events recorded so far, in the order the parser gave them.
    events: Vec<Recorded>,
    /// How many arrays and inline tables are open, which the parser is told to stop nesting at.
    depth: usize,
    /// How many openings past the nesting limit were refused, whose closings are refused too.
    refused: usize,
    /// The arrays and inline tables open, the innermost last: what a comma separates.
    nesting: Vec<Kind>,
}

impl Recorder<'_> {
    /// Records the event over `span` with its role and its token's kind.
    fn push(&mut self, role: Role, kind: Kind, span: Span) {
        self.events.push(Recorded {
            role,
            kind,
            start: Offset(u32::try_from(span.start()).unwrap_or(u32::MAX)),
            end: Offset(u32::try_from(span.end()).unwrap_or(u32::MAX)),
        });
    }

    /// Opens an array or an inline table of `kind` at its `bracket`, unless the nesting limit refuses it: whether the parser
    /// goes on into it.
    fn open(&mut self, kind: Kind, bracket: Kind, span: Span) -> bool {
        if self.depth >= NESTING.0 {
            self.refused += 1;
            self.push(Role::Plain, Kind::Extra(Extra::Error), span);
            return false;
        }
        self.depth += 1;
        self.nesting.push(kind);
        self.push(Role::Open(kind), bracket, span);
        true
    }

    /// Closes the innermost array or inline table of `kind` at its `bracket`, or records a refused one's closing as text the
    /// parser could not read.
    fn close(&mut self, kind: Kind, bracket: Kind, span: Span) {
        if self.refused > 0 {
            self.refused -= 1;
            self.push(Role::Plain, Kind::Extra(Extra::Error), span);
            return;
        }
        self.depth = self.depth.saturating_sub(1);
        let _: Option<Kind> = self.nesting.pop();
        self.push(Role::Close(kind), bracket, span);
    }
}

impl EventReceiver for Recorder<'_> {
    fn std_table_open(&mut self, span: Span, _error: &mut dyn ErrorSink) {
        self.push(
            Role::HeaderOpen(Kind::Node(TomlNode::StdTable)),
            Kind::Token(Token::StdTableOpen),
            span,
        );
    }

    fn std_table_close(&mut self, span: Span, _error: &mut dyn ErrorSink) {
        self.push(Role::HeaderClose, Kind::Token(Token::StdTableClose), span);
    }

    fn array_table_open(&mut self, span: Span, _error: &mut dyn ErrorSink) {
        self.push(
            Role::HeaderOpen(Kind::Node(TomlNode::ArrayTable)),
            Kind::Token(Token::ArrayTableOpen),
            span,
        );
    }

    fn array_table_close(&mut self, span: Span, _error: &mut dyn ErrorSink) {
        self.push(Role::HeaderClose, Kind::Token(Token::ArrayTableClose), span);
    }

    fn inline_table_open(&mut self, span: Span, _error: &mut dyn ErrorSink) -> bool {
        self.open(
            Kind::Node(TomlNode::InlineTable),
            Kind::Token(Token::InlineTableOpen),
            span,
        )
    }

    fn inline_table_close(&mut self, span: Span, _error: &mut dyn ErrorSink) {
        self.close(
            Kind::Node(TomlNode::InlineTable),
            Kind::Token(Token::InlineTableClose),
            span,
        );
    }

    fn array_open(&mut self, span: Span, _error: &mut dyn ErrorSink) -> bool {
        self.open(Kind::Node(TomlNode::Array), Kind::Token(Token::ArrayOpen), span)
    }

    fn array_close(&mut self, span: Span, _error: &mut dyn ErrorSink) {
        self.close(
            Kind::Node(TomlNode::Array),
            Kind::Token(Token::ArrayClose),
            span,
        );
    }

    fn simple_key(&mut self, span: Span, encoding: Option<Encoding>, _error: &mut dyn ErrorSink) {
        let kind = encoding.map_or(Kind::Token(Token::UnquotedKey), string_kind);
        self.push(Role::KeyPart, kind, span);
    }

    fn key_sep(&mut self, span: Span, _error: &mut dyn ErrorSink) {
        self.push(Role::KeyDot, Kind::Token(Token::DotSep), span);
    }

    fn key_val_sep(&mut self, span: Span, _error: &mut dyn ErrorSink) {
        self.push(Role::Equals, Kind::Token(Token::KeyvalSep), span);
    }

    fn scalar(&mut self, span: Span, encoding: Option<Encoding>, _error: &mut dyn ErrorSink) {
        let kind = match encoding {
            _ if span.is_empty() => Kind::Extra(Extra::Error),
            Some(quoted) => string_kind(quoted),
            // The library's decoder classifies an unquoted scalar.
            None => scalar_kind(
                Raw::new_unchecked(
                    self.source
                        .input()
                        .get(span.start()..span.end())
                        .unwrap_or_default(),
                    None,
                    span,
                )
                .decode_scalar(&mut String::new(), &mut |_: ParseError| {}),
            ),
        };
        self.push(Role::Value, kind, span);
    }

    fn value_sep(&mut self, span: Span, _error: &mut dyn ErrorSink) {
        let separator = if self.nesting.last() == Some(&Kind::Node(TomlNode::InlineTable)) {
            Token::InlineTableSep
        } else {
            Token::ArraySep
        };
        self.push(Role::Ends, Kind::Token(separator), span);
    }

    fn whitespace(&mut self, span: Span, _error: &mut dyn ErrorSink) {
        self.push(Role::Plain, Kind::Token(Token::Ws), span);
    }

    fn comment(&mut self, span: Span, _error: &mut dyn ErrorSink) {
        self.push(Role::Ends, Kind::Token(Token::Comment), span);
    }

    fn newline(&mut self, span: Span, _error: &mut dyn ErrorSink) {
        self.push(Role::Ends, Kind::Token(Token::Newline), span);
    }

    fn error(&mut self, span: Span, _error: &mut dyn ErrorSink) {
        self.push(Role::Plain, Kind::Extra(Extra::Error), span);
    }
}

/// The parser's events of `text`, in the order it gives them.
pub(super) fn events(text: &str) -> Vec<Recorded> {
    let source = Source::new(text);
    let lexed = source.lex().into_vec();
    let mut recorder = Recorder {
        source,
        events: Vec::new(),
        depth: 0,
        refused: 0,
        nesting: Vec::new(),
    };
    parse_document(&lexed, &mut recorder, &mut |_: ParseError| {});
    recorder.events
}
