//! The `raw_format` operation over its parser port: it drives the injected parser over a tree's occurrences and retains what it answers.
use crate::domain::format_syntax::types::FormatResultIndex;
use crate::domain::format_syntax::types::{
    FormatDialect, FormatParseResult, FormatParseState, FormatVisibility,
};
use crate::domain::raw_format::service::{body, clear};
use crate::domain::raw_format::types::{
    RawCompleteness, RawFormatBody, RawFormatIndex, RawFormatOccurrence, RawFormatSegment,
    RawFormatToken, RawSegmentIndex, RawTokenOrdinal,
};
use crate::domain::{
    nodes::types::Node,
    syntax::{
        service::{below, kind, literal, standing},
        types::{Kind, Literal, Standing},
    },
    tokens::types::Token,
};
use crate::outbound::format_syntax::interface::FormatSyntaxPort;
use ryo::languages::{self, Position, Symbols, Tree};
use std::cell::{Ref, RefCell};

/// Retain one grammar result for each distinct logical token body.
pub(crate) fn cached<P>(
    index: &mut RawFormatIndex,
    body: RawFormatBody<'_>,
) -> Option<FormatResultIndex>
where
    P: FormatSyntaxPort,
{
    let body = body.0;
    if let Some(cached) = index.by_input.get(body) {
        return Some(*cached);
    }
    let reused = crate::domain::grammar_cache::service::take(
        &mut index.memo,
        crate::domain::grammar_cache::types::Lookup(body),
    );
    let mut owned = String::new();
    if (reused.is_none() && owned.try_reserve(body.len()).is_err())
        || crate::domain::grammar_cache::service::reserve(&mut index.by_input)
            == crate::domain::grammar_cache::types::Reservation::Refused
        || index.values.try_reserve(1).is_err()
    {
        return None;
    }
    let (owned, result) = if let Some(reused) = reused {
        (reused.key.0, reused.value)
    } else {
        owned.push_str(body);
        (
            owned,
            P::parse(
                crate::domain::format_syntax::types::FormatInput(body),
                FormatDialect::PercentString,
            ),
        )
    };
    if result == FormatParseResult::Unavailable {
        index.state = FormatParseState::Partial;
    }
    let cached = FormatResultIndex(index.values.len());
    index.values.push(result);
    let previous = index.by_input.insert(owned.into(), cached);
    debug_assert!(
        previous.is_none(),
        "a previously absent raw body is inserted once"
    );
    Some(cached)
}

/// Record the token segments belonging to one source expression.
pub(crate) fn occurrence<P>(
    tree: &Tree<'_>,
    index: &mut RawFormatIndex,
    node: Position,
    symbols: &Symbols,
    source_complete: RawCompleteness,
) where
    P: FormatSyntaxPort,
{
    if index.occurrences.try_reserve(1).is_err() {
        index.state = FormatParseState::Partial;
        return;
    }
    let first = index.segments.len();
    let mut complete = source_complete;
    let mut fields = 0_usize;
    for token in below(tree, node).filter(|each| kind(tree, *each) == Kind::Token(Token::String)) {
        if index.segments.try_reserve(1).is_err() {
            index.state = FormatParseState::Partial;
            complete = RawCompleteness::Partial;
            break;
        }
        let cached = languages::text_of(
            symbols,
            tree.source,
            languages::text_at(tree.elements, token),
        )
        .and_then(|text| body(RawFormatToken(text.0)))
        .and_then(|body| cached::<P>(index, body));
        if cached
            .and_then(|at| index.values.get(at.0))
            .is_none_or(|result| *result == FormatParseResult::Unavailable)
        {
            index.state = FormatParseState::Partial;
            complete = RawCompleteness::Partial;
        }
        let first_field = fields;
        if let Some(FormatParseResult::Parsed(parsed)) =
            cached.and_then(|at| index.values.get(at.0))
        {
            if let Some(next) = fields.checked_add(parsed.fields.len()) {
                fields = next;
            } else {
                index.state = FormatParseState::Partial;
                complete = RawCompleteness::Partial;
            }
        }
        index.segments.push(RawFormatSegment {
            token,
            ordinal: RawTokenOrdinal(index.segments.len() - first),
            first_field: crate::domain::format_syntax::types::FormatFieldOrdinal(first_field),
            cached,
        });
    }
    let end = index.segments.len();
    if first == end {
        index.state = FormatParseState::Partial;
        complete = RawCompleteness::Partial;
    }
    index.occurrences.push(RawFormatOccurrence {
        node,
        first: RawSegmentIndex(first),
        end: RawSegmentIndex(end),
        complete,
    });
}

/// Both raw readers share one demand-driven cache; decoded and brace caches are untouched.
/// The caller supplies semantic source order and visibility without lending other indexes.
pub(crate) fn prepare<'a, P>(
    tree: &Tree<'_>,
    held: &'a RefCell<RawFormatIndex>,
    symbols: &mut Symbols,
    positions: impl Iterator<Item = Position>,
    mut visibility: impl FnMut(Position, &mut Symbols) -> FormatVisibility,
) -> Ref<'a, RawFormatIndex>
where
    P: FormatSyntaxPort,
{
    if crate::domain::grammar_cache::service::grammar_state::<P, _>(
        &held.borrow().memo,
        crate::domain::grammar_cache::types::DialectName(P::RAW_GRAMMAR.0),
    ) == crate::domain::grammar_cache::types::GrammarState::Stale
    {
        clear(&mut held.borrow_mut());
    }
    if held.borrow().state == FormatParseState::Fresh {
        let mut index = held.borrow_mut();
        crate::domain::grammar_cache::service::grammar::<P, _>(
            &mut index.memo,
            crate::domain::grammar_cache::types::DialectName(P::RAW_GRAMMAR.0),
        );
        let source_complete = if languages::has_incomplete_syntax(tree.elements)
            == languages::SyntaxCompleteness::Complete
            && languages::has_incomplete_lexical_stream(tree.elements)
                == languages::SyntaxCompleteness::Complete
        {
            RawCompleteness::Complete
        } else {
            RawCompleteness::Partial
        };
        index.state = if source_complete == RawCompleteness::Complete {
            FormatParseState::Complete
        } else {
            FormatParseState::Partial
        };
        for node in positions {
            if kind(tree, node) != Kind::Node(Node::Constant)
                || literal(tree, node) != Literal::String
                || standing(tree, node) != Standing::Own
                || visibility(node, symbols) != FormatVisibility::Visible
            {
                continue;
            }
            occurrence::<P>(tree, &mut index, node, symbols, source_complete);
        }
        crate::domain::grammar_cache::service::empty(&mut index.memo);
    }
    held.borrow()
}
