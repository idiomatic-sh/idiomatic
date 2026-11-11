//! The support both groups of the cases share: an adapter that counts its parses.
use crate::domain::literal_expression_parsing::types::{
    CarrierInput, LiteralDialect, LiteralParseOutcome,
};
use crate::outbound::literal_expression::adapters::ruff_string_annotation::grammar::RuffStringAnnotation;
use crate::outbound::literal_expression::interface::LiteralExpressionSyntaxPort;
use std::cell::Cell;

thread_local! {
    /// Relation-specific parser calls in the current isolated test thread.
    pub(super) static CALLS: Cell<usize> = const { Cell::new(0) };
}

/// Count actual adapter invocations without changing its syntactic decisions.
pub(super) struct Counting;

impl LiteralExpressionSyntaxPort for Counting {
    const GRAMMAR: crate::domain::format_syntax::types::GrammarIdentity =
        RuffStringAnnotation::GRAMMAR;

    fn parse(carrier: CarrierInput<'_>, dialect: LiteralDialect) -> LiteralParseOutcome {
        CALLS.with(|count| count.set(count.get() + 1));
        RuffStringAnnotation::parse(carrier, dialect)
    }
}
