//! Reconstruct preserved logical string carriers and cache only syntactic outcomes.
use super::constants;
use super::types::{CarrierText, LiteralParseIndex, LiteralParseOutcome, LiteralParseState};
use crate::domain::{
    syntax::{
        service::{below, kind},
        types::Kind,
    },
    tokens::types::Token,
};
use ryo::languages::{self, NameText, Position, Symbols, Tree};
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
pub(crate) fn carrier(
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

/// Translate an application-owned outcome into the stable relation vocabulary.
pub(crate) const fn outcome_name(outcome: LiteralParseOutcome) -> NameText<'static> {
    match outcome {
        LiteralParseOutcome::Parsed => NameText(constants::PARSED),
        LiteralParseOutcome::Malformed => NameText(constants::MALFORMED),
        LiteralParseOutcome::Unavailable => NameText(constants::UNAVAILABLE),
    }
}
