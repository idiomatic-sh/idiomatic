//! Bind syntactic literal validity independently of embedded AST publication.
use crate::domain::{
    literal_expression_parsing::{
        service::{INCOMPLETE, outcome_name, prepare},
        types::LiteralParseState,
    },
    model::types::Declarations,
    nodes::types::Node,
    syntax::{service::kind_bits, types::Kind},
};
use crate::outbound::literal_expression::adapters::ruff_string_annotation::grammar::RuffStringAnnotation;
use ryo::languages;
use std::convert::identity;

/// The statically bound optional literal parse reader.
pub(super) struct LiteralExpressionParseItem;
ryo::reads! {
    LiteralExpressionParseItem => languages::LiteralExpressionParse, model(Declarations) |tree, model, at, symbols| {
        kinds: [Kind::Node(Node::Module)] as kind_bits,
        given index = prepare::<RuffStringAnnotation>(tree, &model.literal_expression_parsing, symbols, crate::domain::semantic_syntax::service::positions(tree), |node, symbols| (crate::domain::semantic_syntax::service::visibility(tree, model, node, symbols) == crate::domain::semantic_syntax::types::Visibility::Visible).into());
        each (index.occurrences.iter()) as occurrence {
            given dialect = languages::intern_derived(symbols, tree.source, languages::NameText("python-string-annotation"));
            given outcome = languages::intern_derived(symbols, tree.source, outcome_name(occurrence.outcome));
            row { id: node(occurrence.node), dialect: value(dialect), outcome: value(outcome), file: file, span: span(occurrence.node), }
        }
        when identity(index.state) {
            LiteralParseState::Fresh => { partial INCOMPLETE => languages::Missing::Any; }
            LiteralParseState::Partial => { partial INCOMPLETE => languages::Missing::Any; }
            LiteralParseState::Complete => {}
        }
    }
}
#[cfg(test)]
mod tests;
