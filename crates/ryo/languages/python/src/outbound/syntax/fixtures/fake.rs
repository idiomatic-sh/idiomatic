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

    fn walk(
        &self,
        text: SourceText<'_>,
        _source_kind: PythonSourceKind,
        _reported: crate::outbound::syntax::types::ReportedSyntax,
        builder: &mut languages::NodeTableBuilder<'_>,
        names: crate::domain::model::names::types::WalkNames,
        read: &mut dyn FnMut(crate::domain::node::types::Walked<'_, '_, '_>) -> Vec<languages::Position>,
    ) {
        use crate::domain::node::types::{Wholeness, WalkedTree};
        use languages::{FieldCode, SyntaxCompleteness, node_table};
        let end = languages::Offset(u32::try_from(text.0.len()).unwrap_or(u32::MAX));
        let start = languages::Offset(0);
        let none = FieldCode(0);
        node_table::enter_node(builder, kind_bits(Kind::Node(Node::Module)), none, start);
        node_table::enter_node(builder, kind_bits(Kind::Node(Node::Expr)), none, start);
        node_table::enter_node(builder, kind_bits(Kind::Node(Node::Name)), none, start);
        node_table::token(
            builder,
            kind_bits(Kind::Token(Token::Name)),
            none,
            start,
            end,
        );
        node_table::leave_node(builder, end);
        node_table::leave_node(builder, end);
        node_table::leave_node(builder, end);
        let _nothing = read(crate::domain::node::types::Walked::Whole(
            builder,
            WalkedTree {
                nodes: Vec::new(),
                strings: Vec::new(),
                imports: Vec::new(),
                relative_froms: Vec::new(),
                segments: Vec::new(),
                parents: Vec::new(),
                ends: Vec::new(),
                annotations: crate::domain::model::types::BindingAnnotationTiming::Immediate,
                declarations: crate::domain::model::types::AnnotatedDeclarations::Unbound,
                text: languages::NameText(std::str::from_utf8(text.0).unwrap_or_default()),
                completeness: Wholeness {
                    syntax: SyntaxCompleteness::Complete,
                    lexical: SyntaxCompleteness::Complete,
                },
                names,
            },
        ));
    }
}

#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
