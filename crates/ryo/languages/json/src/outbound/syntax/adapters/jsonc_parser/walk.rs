//! The node table the walk writes: the tree of the library's tokens and constructs, numbered as the engine's table numbers it, each
//! node and token with its role under the construct open.

use super::interface::Sink;
use super::schemas::{piece_kind, token_piece};
use super::types::{JsoncText, Piece, Reporting};
use crate::domain::syntax::service::{field_bits, is_value, kind_bits};
use crate::domain::syntax::types::{Field, Kind, Valueness};
use crate::domain::tokens::types::Token;
use jsonc_parser::tokens::{Token as Scanned, TokenAndRange};
use ryo::languages::{FieldCode, NameText, NodeTableBuilder, Offset, node_table};

/// A construct open: what it is, and whether the name of the member it is has been written.
#[derive(Clone, Copy, Debug)]
struct Open {
    /// What it is.
    kind: Kind,
    /// Whether its first string, the name of a member, has been written.
    named: bool,
}

/// The tree written into the builder of a file's node table, and where its last token ends.
pub(super) struct Numbered<'b, 'w, 't> {
    /// What the tree is written through.
    pub builder: &'b mut NodeTableBuilder<'w>,
    /// Where the last token written ends.
    pub end: Offset,
    /// The constructs open, the file first.
    open: Vec<Open>,
    /// The text the library read.
    text: JsoncText<'t>,
    /// Whether text no token spans is told.
    reporting: Reporting,
    /// How many things have been said of the file.
    pub said: usize,
}

impl<'b, 'w, 't> Numbered<'b, 'w, 't> {
    /// A tree written through `builder`, whose file node is open.
    pub(super) fn new(
        builder: &'b mut NodeTableBuilder<'w>,
        text: JsoncText<'t>,
        reporting: Reporting,
    ) -> Self {
        Self {
            builder,
            end: Offset(0),
            open: vec![Open {
                kind: piece_kind(Piece::File),
                named: false,
            }],
            text,
            reporting,
            said: 0,
        }
    }

    /// The role `kind` has under the construct open: a member's first string is its name and its other value its value; each value
    /// of an array and the value of the file is a value.
    fn field(&mut self, kind: Kind) -> FieldCode {
        let field = match self.open.last_mut() {
            Some(parent) if parent.kind == Kind::Node(crate::domain::nodes::types::Node::Member) => {
                if kind == Kind::Token(Token::String) && !parent.named {
                    parent.named = true;
                    Field::Name
                } else if is_value(kind) == Valueness::Value {
                    Field::Value
                } else {
                    Field::None
                }
            }
            Some(parent)
                if matches!(
                    parent.kind,
                    Kind::Node(crate::domain::nodes::types::Node::Array | crate::domain::nodes::types::Node::JsonText)
                ) && is_value(kind) == Valueness::Value =>
            {
                Field::Value
            }
            Some(_) | None => Field::None,
        };
        FieldCode(field_bits(field).0)
    }

    /// Writes the bytes between two tokens: white space, then, if anything is left, text the parser could not read.
    fn fill_to(&mut self, to: Offset) {
        if to <= self.end {
            return;
        }
        let from = self.end;
        let (start, end) = (
            usize::try_from(from.0).unwrap_or(usize::MAX),
            usize::try_from(to.0).unwrap_or(usize::MAX),
        );
        let blanks = self.text.0.get(start..end).map_or(0, |gap| {
            gap.len() - gap.trim_start_matches([' ', '\t', '\n', '\r']).len()
        });
        let middle = Offset(u32::try_from(start.saturating_add(blanks)).unwrap_or(u32::MAX));
        if blanks > 0 {
            let kind = piece_kind(Piece::Whitespace);
            let field = self.field(kind);
            node_table::token(self.builder, kind_bits(kind), field, from, middle);
        }
        if middle < to {
            let kind = piece_kind(Piece::Unread);
            let field = self.field(kind);
            node_table::token(self.builder, kind_bits(kind), field, middle, to);
            if self.reporting == Reporting::Told && self.text.0.get(start..end).is_some() {
                self.diagnose(super::constants::UNREAD_TEXT, middle, to);
            }
        }
        self.end = to;
    }
}

impl Sink for Numbered<'_, '_, '_> {
    fn enter(&mut self, piece: Piece, start: Offset) {
        self.fill_to(start);
        let kind = piece_kind(piece);
        let field = self.field(kind);
        node_table::enter_node(self.builder, kind_bits(kind), field, start.max(self.end));
        self.open.push(Open { kind, named: false });
    }

    fn leave(&mut self, end: Offset) {
        self.fill_to(end);
        node_table::leave_node(self.builder, end.max(self.end));
        let _: Option<Open> = self.open.pop();
    }

    fn token(&mut self, scanned: &TokenAndRange<'_>) {
        let kind = piece_kind(token_piece(&scanned.token));
        let start = Offset(u32::try_from(scanned.range.start).unwrap_or(u32::MAX));
        let end = Offset(u32::try_from(scanned.range.end).unwrap_or(u32::MAX));
        self.fill_to(start);
        let start = start.max(self.end);
        let end = end.max(start);
        let field = self.field(kind);
        match &scanned.token {
            Scanned::String(decoded) => {
                node_table::decoded_token(self.builder, kind_bits(kind), field, start, end, NameText(decoded));
            }
            Scanned::OpenBrace
            | Scanned::CloseBrace
            | Scanned::OpenBracket
            | Scanned::CloseBracket
            | Scanned::Comma
            | Scanned::Colon
            | Scanned::Word(_)
            | Scanned::Boolean(_)
            | Scanned::Number(_)
            | Scanned::Null
            | Scanned::CommentLine(_)
            | Scanned::CommentBlock(_) => {
                node_table::token(self.builder, kind_bits(kind), field, start, end);
            }
        }
        self.end = end;
    }

    fn close(&mut self, length: Offset) {
        self.fill_to(length);
        node_table::leave_node(self.builder, length);
    }

    fn diagnose(&mut self, message: &str, start: Offset, end: Offset) {
        node_table::diagnose(self.builder, start, end, NameText(message));
        self.said += 1;
    }
}

