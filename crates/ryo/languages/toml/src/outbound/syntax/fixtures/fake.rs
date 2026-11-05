//! The fake of the `syntax` port: what the language's tests stand in for it with.

use super::super::interface::SyntaxPort;
use super::super::types::SourceText;
use crate::domain::nodes::types::TomlNode;
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
        languages::enter_node(builder, kind_bits(Kind::Node(TomlNode::Toml)), start);
        languages::enter_node(builder, kind_bits(Kind::Node(TomlNode::Keyval)), start);
        languages::enter_node(builder, kind_bits(Kind::Node(TomlNode::Key)), start);
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
}

#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
