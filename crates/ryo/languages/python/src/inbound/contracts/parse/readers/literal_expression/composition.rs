//! Bind syntactic literal validity independently of embedded AST publication.
use super::constants::PYTHON_STRING_ANNOTATION;
use crate::application::literal_expression_parsing::service::prepare;
use crate::domain::file_view::types::PythonWalk;
use crate::domain::literal_expression_parsing::constants::CARRIER_OPEN;
use crate::domain::literal_expression_parsing::types::{
    CarrierInput, LiteralDialect, LiteralParseOutcome,
};
use crate::domain::literal_facts::service::string_tokens;
use crate::domain::model::names::service::fixed_word;
use crate::domain::node::service::{node_cell, nodes_of_kind, span_at, written};
use crate::domain::node::types::{Numbered, SyntaxFile};
use crate::domain::raw_format::service::body;
use crate::domain::raw_format::types::RawFormatToken;
use crate::domain::{
    literal_expression_parsing::{
        service::{INCOMPLETE, outcome_name},
        types::LiteralParseState,
    },
    model::types::Declarations,
    nodes::types::Node,
    syntax::{service::kind_bits, types::Kind},
};
use crate::outbound::literal_expression::adapters::ruff_string_annotation::grammar::{
    RuffStringAnnotation, parsed_in_place, string_carrier,
};
use crate::outbound::literal_expression::interface::LiteralExpressionSyntaxPort as _;
use ruff_python_ast::AnyNodeRef;
use ruff_text_size::Ranged as _;
use ryo::languages;
use std::convert::identity;

/// The statically bound optional literal parse reader.
pub(crate) struct LiteralExpressionParseItem;

ryo::reads! {
    LiteralExpressionParseItem => languages::LiteralExpressionParse, model(Declarations) |tree, model, at, symbols| {
        kinds: [Kind::Node(Node::Module)] as kind_bits, expanded,
        given index = prepare(&RuffStringAnnotation, tree, &model.literal_expression_parsing, symbols, crate::domain::semantic_syntax::service::positions(tree), |node, symbols| (crate::domain::semantic_syntax::service::visibility(tree, model, node, symbols) == crate::domain::semantic_syntax::types::Visibility::Visible).into());
        each (index.occurrences.iter()) as occurrence {
            given dialect = languages::intern_derived(symbols, tree.source, languages::NameText(PYTHON_STRING_ANNOTATION));
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

/// Whether the literal has the source tokens required by the annotation grammar.
pub(crate) fn available(file: &SyntaxFile<'_>, node: AnyNodeRef<'_>, tokens: &mut Vec<languages::Position>) -> bool {
    string_tokens(file, node, tokens);
    let untold = tokens
        .iter()
        .any(|token| body(RawFormatToken(written(file, *token).0)).is_none());
    !untold && !tokens.is_empty()
}


/// Whether the string `node` reads as an expression the way a string annotation is read: its string tokens as they are written,
/// one on each line between parentheses, parsed by the string-annotation grammar. Unavailable for a string with no token or
/// with a token whose body is not told from its quotes.
fn parsed_as_annotation(
    file: &SyntaxFile<'_>,
    node: AnyNodeRef<'_>,
    tokens: &mut Vec<languages::Position>,
    carrier: &mut crate::domain::literal_facts::types::TextBuffer,
    wants_outcome: bool,
) -> Option<LiteralParseOutcome> {
    if !available(file, node, tokens) {
        return Some(LiteralParseOutcome::Unavailable);
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
        return wants_outcome.then(|| parsed_in_place(literal, file.text.0));
    }
    carrier.0.clear();
    carrier.0.push_str(CARRIER_OPEN);
    for token in tokens.iter() {
        carrier.0.push_str(written(file, *token).0);
        carrier.0.push('\n');
    }
    carrier.0.push(')');
    if !wants_outcome {
        return (!string_carrier(&carrier.0)).then_some(LiteralParseOutcome::Unavailable);
    }
    Some(RuffStringAnnotation.parse(
        CarrierInput(&carrier.0),
        LiteralDialect::PythonStringAnnotation,
    ))
}

// The engine fills `literal-expression-parse` from the walked file: every row of the file, once.
ryo::fills! {
    LiteralExpressionParseItem => languages::LiteralExpressionParse, walk(PythonWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::LiteralExpressionParse as languages::CatalogRelation>::Row;
        let syntax = file.syntax;
        let memo = &mut file.model.words.get_mut().fixed;
        let columns = <languages::LiteralExpressionParse as languages::CatalogRelation>::columns();
        let selected = |column| rows.wants_column(columns.iter().position(|c| *c == column).expect("catalog column"));
        let wants_outcome = selected(languages::LiteralExpressionParseColumn::Outcome);
        let wants_dialect = selected(languages::LiteralExpressionParseColumn::Dialect);
        let mut dialect = None;
        let carrier = &mut file.buffers.text;
        let mut tokens = Vec::new();
        let mut whole = if syntax.completeness.syntax == languages::SyntaxCompleteness::Incomplete {
            languages::SyntaxCompleteness::Incomplete
        } else {
            syntax.completeness.lexical
        };
        for Numbered { id, node } in nodes_of_kind(&syntax, Kind::Node(Node::Constant)) {
            if !matches!(node, AnyNodeRef::ExprStringLiteral(_)) {
                continue;
            }
            let outcome = parsed_as_annotation(&syntax, node, &mut tokens, carrier, wants_outcome);
            if outcome == Some(LiteralParseOutcome::Unavailable) {
                whole = languages::SyntaxCompleteness::Incomplete;
            }
            rows.write(Row {
                id: node_cell(&syntax, id),
                dialect: if wants_dialect {
                    *dialect.get_or_insert_with(|| fixed_word(words, memo, languages::NameText(PYTHON_STRING_ANNOTATION)))
                } else {
                    languages::Symbol(0)
                },
                outcome: outcome.filter(|_| wants_outcome).map_or(languages::Symbol(0), |o| fixed_word(words, memo, outcome_name(o))),
                file: syntax.file,
                span: span_at(&syntax, id),
            });
        }
        if whole == languages::SyntaxCompleteness::Incomplete {
            rows.partial(languages::ReportText(INCOMPLETE), languages::Missing::Any);
        }
    }
}
