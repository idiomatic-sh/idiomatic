//! Bind syntactic literal validity independently of embedded AST publication.
use super::constants::PYTHON_STRING_ANNOTATION;

use crate::domain::file_view::types::PythonWalk;
use crate::domain::literal_expression_parsing::constants::CARRIER_OPEN;
use crate::domain::literal_expression_parsing::types::{CarrierInput, LiteralDialect, LiteralParseOutcome};
use crate::domain::literal_facts::service::string_tokens;
use crate::domain::model::names::service::fixed_word;
use crate::domain::node::service::{node_cell, semantic_nodes_of_kind, span_at, written};
use crate::domain::node::types::{Numbered, SyntaxFile};
use crate::domain::raw_format::service::body;
use crate::domain::raw_format::types::RawFormatToken;
use crate::domain::{literal_expression_parsing::{service::{INCOMPLETE, outcome_name}}, nodes::types::Node, syntax::{types::Kind}};
use crate::outbound::literal_expression::adapters::ruff_string_annotation::grammar::{RuffStringAnnotation, parsed_in_place};
use crate::outbound::literal_expression::interface::LiteralExpressionSyntaxPort as _;
use ruff_python_ast::AnyNodeRef;
use ruff_text_size::Ranged as _;
use ryo::languages;


/// The statically bound optional literal parse reader.
pub(crate) struct LiteralExpressionParseItem;

/// Whether the string `node` reads as an expression the way a string annotation is read: its string tokens as they are written,
/// one on each line between parentheses, parsed by the string-annotation grammar. Unavailable for a string with no token or
/// with a token whose body is not told from its quotes.
fn parsed_as_annotation(
    file: &SyntaxFile<'_>,
    node: AnyNodeRef<'_>,
    tokens: &mut Vec<languages::Position>,
    carrier: &mut crate::domain::literal_facts::types::TextBuffer,
) -> LiteralParseOutcome {
    string_tokens(file, node, tokens);
    let untold = tokens
        .iter()
        .any(|token| body(RawFormatToken(written(file, *token).0)).is_none());
    if untold || tokens.is_empty() {
        return LiteralParseOutcome::Unavailable;
    }
    // SPIKE (track X): the string is a node of the file's own tree, so the annotation grammar reads it where it is
    // written: no carrier is assembled and the string is not parsed a second time to get the node the tree holds
    // (that was 900 ns a row). A file the parser did not read whole keeps the carrier.
    if file.completeness.syntax != languages::SyntaxCompleteness::Incomplete
        && let AnyNodeRef::ExprStringLiteral(literal) = node
        && file
            .text
            .0
            .get(literal.range().start().to_usize()..literal.range().end().to_usize())
            .is_some()
    {
        return parsed_in_place(literal, file.text.0);
    }
    carrier.0.clear();
    carrier.0.push_str(CARRIER_OPEN);
    for token in tokens.iter() {
        carrier.0.push_str(languages::node_table::decoded_of(file.table, *token).unwrap_or_else(|| written(file, *token)).0);
        carrier.0.push('\n');
    }
    carrier.0.push(')');
    RuffStringAnnotation.parse(
        CarrierInput(&carrier.0),
        LiteralDialect::PythonStringAnnotation,
    )
}

// The engine fills `literal-expression-parse` from the walked file: every row of the file, once.
ryo::fills! {
    LiteralExpressionParseItem => languages::LiteralExpressionParse, walk(PythonWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::LiteralExpressionParse as languages::CatalogRelation>::Row;
        let syntax = file.syntax;
        let memo = &mut file.model.words.get_mut().fixed;
        let carrier = &mut file.buffers.text;
        let mut tokens = Vec::new();
        let mut whole = if syntax.completeness.syntax == languages::SyntaxCompleteness::Incomplete {
            languages::SyntaxCompleteness::Incomplete
        } else {
            syntax.completeness.lexical
        };
        for Numbered { id, node } in semantic_nodes_of_kind(&syntax, Kind::Node(Node::Constant)) {
            if !matches!(node, AnyNodeRef::ExprStringLiteral(_)) {
                continue;
            }
            let outcome = parsed_as_annotation(&syntax, node, &mut tokens, carrier);
            if outcome == LiteralParseOutcome::Unavailable {
                whole = languages::SyntaxCompleteness::Incomplete;
            }
            rows.write(Row {
                id: node_cell(&syntax, id),
                dialect: fixed_word(words, memo, languages::NameText(PYTHON_STRING_ANNOTATION)),
                outcome: fixed_word(words, memo, outcome_name(outcome)),
                file: syntax.file,
                span: span_at(&syntax, id),
            });
        }
        if whole == languages::SyntaxCompleteness::Incomplete {
            rows.partial(languages::ReportText(INCOMPLETE), languages::Missing::Any);
        }
    }
}
