//! Reconstruct preserved logical string carriers and cache only syntactic outcomes.
use super::constants;
use super::{
    interface::LiteralExpressionSyntax,
    types::{
        CarrierInput, CarrierKey, CarrierText, CarrierVisibility, HostState, LiteralDialect,
        LiteralParseIndex, LiteralParseOccurrence, LiteralParseOutcome, LiteralParseState,
    },
};
use crate::domain::{
    nodes::types::Node,
    syntax::{
        service::{below, kind, literal, standing},
        types::{Kind, Literal, Standing},
    },
    tokens::types::Token,
};
use ryo::languages::{self, NameText, Position, Symbols, Tree};
use std::cell::{Ref, RefCell};
use std::collections::HashSet;

/// Missing logical carriers must remain observable through a partial relation read.
pub(crate) const INCOMPLETE: &str = constants::INCOMPLETE;

/// Drop source identities and admit budgeted grammar outcomes for the next demand.
pub(crate) fn clear(index: &mut LiteralParseIndex) {
    crate::domain::grammar_cache::service::empty(&mut index.memo);
    for (key, outcome) in index.by_carrier.drain(..) {
        if outcome != LiteralParseOutcome::Unavailable {
            crate::domain::grammar_cache::service::admit(
                &mut index.memo,
                crate::domain::grammar_cache::types::MemoKey(key.input.0),
                outcome,
            );
        }
    }
    crate::domain::grammar_cache::service::reset_map(&mut index.by_carrier);
    crate::domain::grammar_cache::service::reset(&mut index.occurrences);
    index.embedded.clear();
    if index.embedded.capacity() > constants::EMBEDDED_CAPACITY {
        index.embedded = HashSet::new();
    }
    index.state = LiteralParseState::Fresh;
}

/// Rebuild an injective spelling of the original string-token sequence.
/// Grouping permits multiline implicit concatenation; newlines delimit tokens
/// without changing any prefix, quote, escape, or literal-body byte.
fn carrier(
    tree: &Tree<'_>,
    node: Position,
    symbols: &Symbols,
    source: &mut CarrierText,
) -> Option<()> {
    let source = &mut source.0;
    source.clear();
    if source.try_reserve(3).is_err() {
        return None;
    }
    source.push_str(constants::CARRIER_OPEN);
    let mut tokens = 0_usize;
    for token in below(tree, node).filter(|at| kind(tree, *at) == Kind::Token(Token::String)) {
        let spelling = languages::text_of(
            symbols,
            tree.source,
            languages::text_at(tree.elements, token),
        )?;
        // A complete ordinary string-token envelope excludes lexer recovery and nonstrings.
        let _body = crate::domain::raw_format::service::body(
            crate::domain::raw_format::types::RawFormatToken(spelling.0),
        )?;
        if source
            .try_reserve(spelling.0.len().checked_add(1)?)
            .is_err()
        {
            return None;
        }
        source.push_str(spelling.0);
        source.push('\n');
        tokens = tokens.checked_add(1)?;
    }
    if tokens == 0 {
        return None;
    }
    if source.try_reserve(1).is_err() {
        return None;
    }
    source.push(')');
    Some(())
}

/// Return one cached result, reusing an already parsed embedded host when possible.
fn outcome<P>(
    index: &mut LiteralParseIndex,
    assembled: CarrierText,
    host: HostState,
) -> LiteralParseOutcome
where
    P: LiteralExpressionSyntax,
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
    P: LiteralExpressionSyntax,
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
        index.state = if languages::has_incomplete_syntax(tree.elements) == languages::SyntaxCompleteness::Incomplete
            || languages::has_incomplete_lexical_stream(tree.elements) == languages::SyntaxCompleteness::Incomplete
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

/// Translate an application-owned outcome into the stable relation vocabulary.
pub(crate) const fn outcome_name(outcome: LiteralParseOutcome) -> NameText<'static> {
    match outcome {
        LiteralParseOutcome::Parsed => NameText(constants::PARSED),
        LiteralParseOutcome::Malformed => NameText(constants::MALFORMED),
        LiteralParseOutcome::Unavailable => NameText(constants::UNAVAILABLE),
    }
}
