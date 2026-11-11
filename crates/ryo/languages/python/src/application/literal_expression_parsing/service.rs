//! The `literal_expression_parsing` operation over its parser port: it drives the injected parser over a tree's occurrences and retains what it answers.
use crate::domain::literal_expression_parsing::service::{carrier, clear};
use crate::domain::literal_expression_parsing::types::{
    CarrierInput, CarrierKey, CarrierText, CarrierVisibility, HostState, LiteralDialect,
    LiteralParseIndex, LiteralParseOccurrence, LiteralParseOutcome, LiteralParseState,
};
use crate::domain::{
    nodes::types::Node,
    syntax::{
        service::{kind, literal, standing},
        types::{Kind, Literal, Standing},
    },
};
use crate::outbound::literal_expression::interface::LiteralExpressionSyntaxPort;
use ryo::languages::{self, Position, Symbols, Tree};
use std::cell::{Ref, RefCell};

/// Return one cached result, reusing an already parsed embedded host when possible.
pub(crate) fn outcome<P>(
    index: &mut LiteralParseIndex,
    assembled: CarrierText,
    host: HostState,
) -> LiteralParseOutcome
where
    P: LiteralExpressionSyntaxPort,
{
    let key = CarrierKey {
        dialect: LiteralDialect::PythonStringAnnotation,
        input: assembled,
    };
    if let Some(outcome) = index.by_carrier.get(&key).copied() {
        crate::domain::grammar_cache::service::scratch(
            &mut index.memo,
            crate::domain::grammar_cache::types::Scratch(key.input.0),
        );
        return outcome;
    }
    if crate::domain::grammar_cache::service::reserve(&mut index.by_carrier)
        == crate::domain::grammar_cache::types::Reservation::Refused
    {
        return LiteralParseOutcome::Unavailable;
    }
    let (stored_input, result) = if let Some(taken) = crate::domain::grammar_cache::service::take(
        &mut index.memo,
        crate::domain::grammar_cache::types::Lookup(&key.input.0),
    ) {
        crate::domain::grammar_cache::service::scratch(
            &mut index.memo,
            crate::domain::grammar_cache::types::Scratch(key.input.0),
        );
        (CarrierText(taken.key.0), taken.value)
    } else {
        (key.input, LiteralParseOutcome::Unavailable)
    };
    let result = if result != LiteralParseOutcome::Unavailable {
        result
    } else if host == HostState::Embedded {
        LiteralParseOutcome::Parsed
    } else {
        P::parse(CarrierInput(&stored_input.0), key.dialect)
    };
    let previous = index.by_carrier.insert(
        CarrierKey {
            dialect: key.dialect,
            input: stored_input,
        },
        result,
    );
    debug_assert!(
        previous.is_none(),
        "a checked carrier has no cached outcome yet"
    );
    result
}

/// A relation-level demand scans eligible carriers once; no query host pushdown is assumed.
/// Source order and candidate visibility come from the caller's semantic projection.
pub(crate) fn prepare<'a, P>(
    tree: &Tree<'_>,
    held: &'a RefCell<LiteralParseIndex>,
    symbols: &mut Symbols,
    positions: impl Iterator<Item = Position>,
    mut visibility: impl FnMut(Position, &mut Symbols) -> CarrierVisibility,
) -> Ref<'a, LiteralParseIndex>
where
    P: LiteralExpressionSyntaxPort,
{
    if crate::domain::grammar_cache::service::grammar_state::<P, _>(
        &held.borrow().memo,
        crate::domain::grammar_cache::types::DialectName(P::GRAMMAR.0),
    ) == crate::domain::grammar_cache::types::GrammarState::Stale
    {
        clear(&mut held.borrow_mut());
    }
    if held.borrow().state == LiteralParseState::Fresh {
        let mut index = held.borrow_mut();
        crate::domain::grammar_cache::service::grammar::<P, _>(
            &mut index.memo,
            crate::domain::grammar_cache::types::DialectName(P::GRAMMAR.0),
        );
        index.state = if languages::has_incomplete_syntax(tree.elements)
            == languages::SyntaxCompleteness::Incomplete
            || languages::has_incomplete_lexical_stream(tree.elements)
                == languages::SyntaxCompleteness::Incomplete
        {
            LiteralParseState::Partial
        } else {
            LiteralParseState::Complete
        };
        // Existing hidden successes are valid syntax evidence, without publishing their roots.
        index.embedded.clear();
        let mut embedded_available = true;
        for expression in languages::embedded_expressions(tree.elements) {
            if index.embedded.try_reserve(1).is_err() {
                embedded_available = false;
                break;
            }
            let _new_host = index.embedded.insert(expression.host);
        }
        for node in positions {
            if kind(tree, node) != Kind::Node(Node::Constant)
                || literal(tree, node) != Literal::String
                || standing(tree, node) != Standing::Own
                || visibility(node, symbols) != CarrierVisibility::Visible
            {
                continue;
            }
            if index.occurrences.try_reserve(1).is_err() {
                index.state = LiteralParseState::Partial;
                break;
            }
            let mut source = CarrierText(std::mem::take(&mut index.memo.scratch).0);
            let result = if carrier(tree, node, symbols, &mut source).is_some() {
                let host = if embedded_available && index.embedded.contains(&node) {
                    HostState::Embedded
                } else {
                    HostState::Standalone
                };
                outcome::<P>(&mut index, source, host)
            } else {
                crate::domain::grammar_cache::service::scratch(
                    &mut index.memo,
                    crate::domain::grammar_cache::types::Scratch(source.0),
                );
                LiteralParseOutcome::Unavailable
            };
            if result == LiteralParseOutcome::Unavailable {
                index.state = LiteralParseState::Partial;
            }
            index.occurrences.push(LiteralParseOccurrence {
                node,
                outcome: result,
            });
        }
        crate::domain::grammar_cache::service::empty(&mut index.memo);
    }
    held.borrow()
}
