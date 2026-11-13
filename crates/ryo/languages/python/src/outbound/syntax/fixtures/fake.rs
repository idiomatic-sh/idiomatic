//! The fake of the `syntax` port: what the language's tests stand in for it with.

use super::super::interface::SyntaxPort;
use super::super::types::{PythonSourceKind, SourceText};
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;
use ryo::languages;

/// What the language's tests stand in for `syntax` with: it reads any text as a module holding one expression statement, a name whose
/// token is the text itself.
pub(crate) struct SyntaxFake;

impl SyntaxPort for SyntaxFake {
    fn expand_annotations(
        &self,
        _source: SourceText<'_>,
        carriers: &[crate::outbound::syntax::types::DeferredAnnotation],
        _builder: &mut ryo::languages::TableBuilder<'_>,
    ) {
        assert!(
            carriers.is_empty(),
            "this syntax fixture supplies no deferred type carrier"
        );
    }

    fn read(
        &self,
        text: SourceText<'_>,
        _source_kind: PythonSourceKind,
        builder: &mut languages::TableBuilder<'_>,
    ) {
        let end = languages::Offset(u32::try_from(text.0.len()).unwrap_or(u32::MAX));
        let start = languages::Offset(0);
        languages::enter_node(builder, kind_bits(Kind::Node(Node::Module)), start);
        languages::enter_node(builder, kind_bits(Kind::Node(Node::Expr)), start);
        languages::enter_node(builder, kind_bits(Kind::Node(Node::Name)), start);
        languages::token(builder, kind_bits(Kind::Token(Token::Name)), start, end);
        languages::leave_node(builder, end);
        languages::leave_node(builder, end);
        languages::leave_node(builder, end);
    }
}

#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
