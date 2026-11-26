//! The fake of the `syntax` port: what the language's tests stand in for it with.

use super::super::interface::SyntaxPort;
use super::super::types::SourceText;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;
use ryo::languages;

/// What the language's tests stand in for `syntax` with: it reads any text as a file holding one token, the whole text.
pub(crate) struct SyntaxFake;

impl SyntaxPort for SyntaxFake {
    fn walk(
        &self,
        text: SourceText<'_>,
        _at: languages::SourcePath<'_>,
        builder: &mut languages::NodeTableBuilder<'_>,
    ) {
        use languages::{FieldCode, node_table};
        let end = languages::Offset(u32::try_from(text.0.len()).unwrap_or(u32::MAX));
        let start = languages::Offset(0);
        node_table::enter_node(builder, kind_bits(Kind::Node(Node::SourceFile)), FieldCode(0), start);
        node_table::token(builder, kind_bits(Kind::Token(Token::Characters)), FieldCode(0), start, end);
        node_table::leave_node(builder, end);
    }
}

#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
