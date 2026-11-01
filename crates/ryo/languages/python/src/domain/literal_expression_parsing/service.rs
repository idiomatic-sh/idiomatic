//! Reconstruct preserved logical string carriers and cache only syntactic outcomes.
use super::{
    interface::LiteralExpressionSyntax,
    types::{
        LiteralDialect, LiteralParseIndex, LiteralParseOccurrence, LiteralParseOutcome,
        LiteralParseState,
    },
};
use crate::domain::{
    model::types::Declarations,
    nodes::types::Node,
    semantic_syntax::types::Visibility,
    syntax::{
        service::{below, kind, literal, standing},
        types::{Kind, Literal, Standing},
    },
    tokens::types::Token,
};
use ryo::languages::{self, Position, Symbols, Tree};
use std::cell::Ref;
use std::collections::HashSet;

/// Missing logical carriers must remain observable through a partial relation read.
pub(crate) const INCOMPLETE: &str =
    "literal-expression parsing requires complete logical string carriers";

/// Drop source identities and outcomes while retaining allocation capacity.
pub(crate) fn clear(index: &mut LiteralParseIndex) {
    index.by_carrier.clear();
    index.occurrences.clear();
    index.state = LiteralParseState::Fresh;
}

/// Rebuild an injective spelling of the original string-token sequence.
/// Grouping permits multiline implicit concatenation; newlines delimit tokens
/// without changing any prefix, quote, escape, or literal-body byte.
fn carrier(tree: &Tree<'_>, node: Position, symbols: &Symbols) -> Option<String> {
    let mut source = String::new();
    source.try_reserve(3).ok()?;
    source.push_str("(\n");
    let mut tokens = 0_usize;
    for token in below(tree, node).filter(|at| kind(tree, *at) == Kind::Token(Token::String)) {
        let spelling = languages::text_of(
            symbols,
            tree.source,
            languages::text_at(tree.elements, token),
        )?;
        // A complete ordinary string-token envelope excludes lexer recovery and nonstrings.
        let _body = crate::domain::raw_format::service::body(spelling.0)?;
        source.try_reserve(spelling.0.len().checked_add(1)?).ok()?;
        source.push_str(spelling.0);
        source.push('\n');
        tokens = tokens.checked_add(1)?;
    }
    if tokens == 0 {
        return None;
    }
    source.try_reserve(1).ok()?;
    source.push(')');
    Some(source)
}

/// Return one cached result, reusing an already parsed embedded host when possible.
fn outcome<P: LiteralExpressionSyntax>(
    index: &mut LiteralParseIndex,
    source: String,
    embedded: bool,
) -> LiteralParseOutcome {
    let key = (LiteralDialect::PythonStringAnnotation, source);
    if let Some(outcome) = index.by_carrier.get(&key) {
        return *outcome;
    }
    if index.by_carrier.try_reserve(1).is_err() {
        return LiteralParseOutcome::Unavailable;
    }
    let result = if embedded {
        LiteralParseOutcome::Parsed
    } else {
        P::parse(&key.1, key.0)
    };
    let previous = index.by_carrier.insert(key, result);
    debug_assert!(previous.is_none());
    result
}

/// Build an optional success lookup; allocation failure merely disables reuse.
fn embedded_hosts(tree: &Tree<'_>) -> Option<HashSet<Position>> {
    let mut hosts = HashSet::new();
    for expression in languages::embedded_expressions(tree.elements) {
        hosts.try_reserve(1).ok()?;
        let _new_host = hosts.insert(expression.host);
    }
    Some(hosts)
}

/// A relation-level demand scans eligible carriers once; no query host pushdown is assumed.
pub(crate) fn prepare<'a, P: LiteralExpressionSyntax>(
    tree: &Tree<'_>,
    model: &'a Declarations,
    symbols: &mut Symbols,
) -> Ref<'a, LiteralParseIndex> {
    if model.literal_expression_parsing.borrow().state == LiteralParseState::Fresh {
        let mut index = model.literal_expression_parsing.borrow_mut();
        index.state = if languages::has_incomplete_syntax(tree.elements)
            || languages::has_incomplete_lexical_stream(tree.elements)
        {
            LiteralParseState::Partial
        } else {
            LiteralParseState::Complete
        };
        // Existing hidden successes are valid syntax evidence, without publishing their roots.
        let embedded = embedded_hosts(tree);
        for node in crate::domain::semantic_syntax::service::positions(tree) {
            if kind(tree, node) != Kind::Node(Node::Constant)
                || literal(tree, node) != Literal::String
                || standing(tree, node) != Standing::Own
                || crate::domain::semantic_syntax::service::visibility(tree, model, node, symbols)
                    != Visibility::Visible
            {
                continue;
            }
            if index.occurrences.try_reserve(1).is_err() {
                index.state = LiteralParseState::Partial;
                break;
            }
            let result =
                carrier(tree, node, symbols).map_or(LiteralParseOutcome::Unavailable, |source| {
                    outcome::<P>(
                        &mut index,
                        source,
                        embedded.as_ref().is_some_and(|hosts| hosts.contains(&node)),
                    )
                });
            if result == LiteralParseOutcome::Unavailable {
                index.state = LiteralParseState::Partial;
            }
            index.occurrences.push(LiteralParseOccurrence {
                node,
                outcome: result,
            });
        }
    }
    model.literal_expression_parsing.borrow()
}

/// Translate an application-owned outcome into the stable relation vocabulary.
pub(crate) const fn outcome_name(outcome: LiteralParseOutcome) -> &'static str {
    match outcome {
        LiteralParseOutcome::Parsed => "parsed",
        LiteralParseOutcome::Malformed => "malformed",
        LiteralParseOutcome::Unavailable => "unavailable",
    }
}
