//! Parse preserved logical token spellings without decoding or concatenating bodies.
use super::types::{
    RawCompleteness, RawFormatBody, RawFormatIndex, RawFormatOccurrence, RawFormatSegment,
    RawFormatToken, RawSegmentIndex, RawTokenOrdinal,
};
use crate::domain::format_syntax::types::FormatResultIndex;
use crate::domain::format_syntax::{
    interface::FormatSyntax,
    types::{FormatDialect, FormatParseResult, FormatParseState},
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

/// Discard current source identities and bodies while retaining collection capacities.
pub(crate) fn clear(index: &mut RawFormatIndex) {
    crate::domain::grammar_cache::service::empty(&mut index.memo);
    for (input, at) in index.by_input.drain(..) {
        if let Some(value) = index.values.get_mut(at.0) {
            let result = std::mem::replace(value, FormatParseResult::Unavailable);
            let bytes = crate::domain::format_parsing::service::owned_bytes(&result);
            if result != FormatParseResult::Unavailable {
                crate::domain::grammar_cache::service::admit(
                    &mut index.memo,
                    input.0,
                    result,
                    bytes,
                );
            }
        }
    }
    crate::domain::grammar_cache::service::reset_map(&mut index.by_input);
    crate::domain::grammar_cache::service::reset(&mut index.values);
    crate::domain::grammar_cache::service::reset(&mut index.occurrences);
    crate::domain::grammar_cache::service::reset(&mut index.segments);
    index.state = FormatParseState::Fresh;
}

/// Check the ordinary string-token envelope and borrow its raw body unchanged.
pub(crate) fn body(token: RawFormatToken<'_>) -> Option<RawFormatBody<'_>> {
    let token = token.0;
    let bytes = token.as_bytes();
    let prefix = usize::from(matches!(bytes.first(), Some(b'r' | b'R' | b'u' | b'U')));
    let quoted = token.get(prefix..)?;
    let quote = *quoted.as_bytes().first()?;
    if !matches!(quote, b'\'' | b'"') {
        return None;
    }
    let width = if quoted
        .as_bytes()
        .get(..3)
        .is_some_and(|first| first.iter().all(|byte| *byte == quote))
    {
        3
    } else {
        1
    };
    let end = quoted.len().checked_sub(width)?;
    if end < width
        || !quoted
            .as_bytes()
            .get(end..)?
            .iter()
            .all(|byte| *byte == quote)
    {
        return None;
    }
    quoted.get(width..end).map(RawFormatBody)
}

/// Retain one grammar result for each distinct logical token body.
fn cached<P>(index: &mut RawFormatIndex, body: RawFormatBody<'_>) -> Option<FormatResultIndex>
where
    P: FormatSyntax,
{
    let body = body.0;
    if let Some(cached) = index.by_input.get(body) {
        return Some(*cached);
    }
    let reused = crate::domain::grammar_cache::service::take(&mut index.memo, body);
    let mut owned = String::new();
    if (reused.is_none() && owned.try_reserve(body.len()).is_err())
        || !crate::domain::grammar_cache::service::reserve(&mut index.by_input)
        || index.values.try_reserve(1).is_err()
    {
        return None;
    }
    let (owned, result) = if let Some(reused) = reused {
        reused
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
fn occurrence<P>(
    tree: &Tree<'_>,
    index: &mut RawFormatIndex,
    node: Position,
    symbols: &Symbols,
    source_complete: RawCompleteness,
) where
    P: FormatSyntax,
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
pub(crate) fn prepare<'a, P>(
    tree: &Tree<'_>,
    model: &'a Declarations,
    symbols: &mut Symbols,
) -> Ref<'a, RawFormatIndex>
where
    P: FormatSyntax,
{
    if !crate::domain::grammar_cache::service::is_grammar::<P, _>(
        &model.raw_format.borrow().memo,
        P::RAW_GRAMMAR.0,
    ) {
        clear(&mut model.raw_format.borrow_mut());
    }
    if model.raw_format.borrow().state == FormatParseState::Fresh {
        let mut index = model.raw_format.borrow_mut();
        crate::domain::grammar_cache::service::grammar::<P, _>(&mut index.memo, P::RAW_GRAMMAR.0);
        let source_complete = if !languages::has_incomplete_syntax(tree.elements)
            && !languages::has_incomplete_lexical_stream(tree.elements)
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
        for node in crate::domain::semantic_syntax::service::positions(tree) {
            if kind(tree, node) != Kind::Node(Node::Constant)
                || literal(tree, node) != Literal::String
                || standing(tree, node) != Standing::Own
                || crate::domain::semantic_syntax::service::visibility(tree, model, node, symbols)
                    != Visibility::Visible
            {
                continue;
            }
            occurrence::<P>(tree, &mut index, node, symbols, source_complete);
        }
        crate::domain::grammar_cache::service::empty(&mut index.memo);
    }
    model.raw_format.borrow()
}

/// The token sequence belonging to one retained source expression.
pub(crate) fn segments<'a>(
    index: &'a RawFormatIndex,
    occurrence: &RawFormatOccurrence,
) -> &'a [RawFormatSegment] {
    index
        .segments
        .get(occurrence.first.0..occurrence.end.0)
        .unwrap_or_default()
}

/// A whole parser outcome, shared by equal raw bodies.
pub(crate) fn result<'a>(
    index: &'a RawFormatIndex,
    segment: &RawFormatSegment,
) -> Option<&'a FormatParseResult> {
    index.values.get(segment.cached?.0)
}

/// Completeness reported by either demand-driven raw reader.
pub(crate) fn state(index: &RawFormatIndex) -> FormatParseState {
    index.state
}
