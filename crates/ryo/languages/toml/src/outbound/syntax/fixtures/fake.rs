//! The fake of the `syntax` port: what the language's tests stand in for it with.

use super::super::interface::SyntaxPort;
use super::super::types::SourceText;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;
use ryo::languages;

/// What the language's tests stand in for `syntax` with: it reads any text as a document holding one key-value, whose key is the
/// text itself and whose value is an integer written as nothing.
pub(crate) struct SyntaxFake;

impl SyntaxPort for SyntaxFake {
    fn read(&self, text: SourceText<'_>, builder: &mut languages::TableBuilder<'_>) {
        let end = languages::Offset(u32::try_from(text.0.len()).unwrap_or(u32::MAX));
        let start = languages::Offset(0);
        languages::enter_node(builder, kind_bits(Kind::Node(Node::Toml)), start);
        languages::enter_node(builder, kind_bits(Kind::Node(Node::Keyval)), start);
        languages::enter_node(builder, kind_bits(Kind::Node(Node::Key)), start);
        languages::token(
            builder,
            kind_bits(Kind::Token(Token::UnquotedKey)),
            start,
            end,
        );
        languages::leave_node(builder, end);
        languages::token(builder, kind_bits(Kind::Token(Token::KeyvalSep)), end, end);
        languages::token(builder, kind_bits(Kind::Token(Token::Integer)), end, end);
        languages::leave_node(builder, end);
        languages::leave_node(builder, end);
    }

    fn walk(&self, text: SourceText<'_>, _values: bool, _decodings: &mut crate::domain::file_view::types::Decodings, builder: &mut languages::NodeTableBuilder<'_>) {
        use languages::{FieldCode, node_table};
        let end = languages::Offset(u32::try_from(text.0.len()).unwrap_or(u32::MAX));
        let start = languages::Offset(0);
        let none = FieldCode(0);
        node_table::enter_node(builder, kind_bits(Kind::Node(Node::Toml)), none, start);
        node_table::enter_node(builder, kind_bits(Kind::Node(Node::Keyval)), none, start);
        node_table::enter_node(builder, kind_bits(Kind::Node(Node::Key)), none, start);
        node_table::token(builder, kind_bits(Kind::Token(Token::UnquotedKey)), none, start, end);
        node_table::leave_node(builder, end);
        node_table::token(builder, kind_bits(Kind::Token(Token::KeyvalSep)), none, end, end);
        node_table::token(builder, kind_bits(Kind::Token(Token::Integer)), none, end, end);
        node_table::leave_node(builder, end);
        node_table::leave_node(builder, end);
    }
}

#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
