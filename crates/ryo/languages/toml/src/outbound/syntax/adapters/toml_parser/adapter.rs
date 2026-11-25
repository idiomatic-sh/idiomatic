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

use super::constants::NOT_UTF8;
use super::schemas::{event_kind, reports, string_kind};
use super::types::{Decoding, Opened, Part, Report, Scratch, TableReadPhase, TomlText};
use super::walk::Numbered;
use super::writer::interface::Sink;
use super::writer::service::{Writer, offset, start_of};
use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::{field_bits, kind_bits};
use crate::domain::syntax::types::Field;
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;
use crate::outbound::syntax::interface::SyntaxPort;
use crate::outbound::syntax::types::SourceText;
use ryo::languages::{self, Count, Offset};
use toml_parser::decoder::Encoding;
use toml_parser::parser::{EventKind, EventReceiver, parse_document};
use toml_parser::{ErrorSink, ParseError, Source, Span};

/// The `toml_parser` syntax adapter.
#[derive(Debug)]
pub(crate) struct SyntaxTomlParserAdapter;

impl<T: Sink> EventReceiver for Writer<'_, T> {
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

/// What reading the events of a document into a tree leaves besides the tree.
pub(super) struct Events<T> {
    /// The tree, written whole.
    pub tree: T,
    /// How many bytes of the text are UTF-8, which the library reads.
    pub readable: Count,
    /// What the library said is wrong with the document.
    pub reports: Vec<Report>,
}

/// Reads the events of `text` into `tree`, which already holds the document node, and closes the document.
pub(super) fn events<T: Sink>(tree: T, text: &[u8], values: bool) -> Events<T> {
    // TOML is UTF-8: the library reads the text up to the first byte that is not, and the rest is text it could not read.
    let content = TomlText(languages::readable_text(languages::ByteSlice(text)).0);
    let source = Source::new(content.0);
    let tokens = source.lex().into_vec();
    let mut writer = Writer {
        values,
        tree,
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
    writer.tree.finish(offset(Count(text.len())));
    let reports = reports(content.0, &writer.refusals);
    Events {
        tree: writer.tree,
        readable: Count(content.0.len()),
        reports,
    }
}

impl SyntaxPort for SyntaxTomlParserAdapter {
    fn read(&self, text: SourceText<'_>, builder: &mut languages::TableBuilder<'_>) {
        languages::enter_node(builder, kind_bits(Kind::Node(Node::Toml)), Offset(0));
        // These relations expose decoded scalar values or depend on decoded keys.
        let values = builder.decode_for(&[
            languages::Relation::StringLiteral, languages::Relation::Mapping,
            languages::Relation::Entry, languages::Relation::SequenceElement,
        ]);
        let length = offset(Count(text.0.len()));
        let Events { tree, readable, reports } =
            events(languages::lossless(builder, Offset(0)), text.0, values);
        let builder = tree.builder;
        if readable.0 < text.0.len() {
            languages::diagnose(builder, languages::ReportText(NOT_UTF8), offset(readable), length);
        }
        for report in reports {
            languages::diagnose(
                builder,
                languages::ReportText(&report.message.0),
                offset(report.start),
                offset(report.end),
            );
        }
    }

    fn walk(&self, text: SourceText<'_>, values: bool, builder: &mut languages::NodeTableBuilder<'_>) {
        let length = offset(Count(text.0.len()));
        languages::node_table::enter_node(
            builder,
            kind_bits(Kind::Node(Node::Toml)),
            languages::FieldCode(field_bits(Field::None).0),
            Offset(0),
        );
        let Events { tree, readable, reports } = events(Numbered::new(builder), text.0, values);
        let builder = tree.builder;
        let accepted = reports.is_empty() && readable.0 == text.0.len();
        if readable.0 < text.0.len() {
            languages::node_table::diagnose(builder, offset(readable), length, languages::NameText(NOT_UTF8));
        }
        for report in reports {
            languages::node_table::diagnose(
                builder,
                offset(report.start),
                offset(report.end),
                languages::NameText(&report.message.0),
            );
        }
        languages::node_table::set_syntax_acceptance(
            builder,
            if accepted {
                languages::SyntaxAcceptance::Accepted
            } else {
                languages::SyntaxAcceptance::Rejected
            },
        );
    }
}
