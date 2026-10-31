//! Pair lexical INDENT and DEDENT tokens in their original order, without visual-column inference.
use super::types::{IndentationIndex, IndentationRegion, IndentationState};
use crate::domain::{
    syntax::{service::kind, types::Kind},
    tokens::types::Token,
};
use ryo::languages::{self, Offset, Position, Span, Tree};
use std::cell::{Ref, RefCell};
/// Invalidate a retained model without discarding allocated rows or stack capacity.
pub(crate) fn clear(index: &mut IndentationIndex) {
    index.rows.clear();
    index.stack.clear();
    index.state = IndentationState::Fresh;
}
/// Read the current file's regions, computing them only on the first request.
/// A mutable borrow ends before any rows are exposed to the relation reader.
#[must_use]
pub(crate) fn regions<'a>(
    tree: &Tree<'_>,
    cache: &'a RefCell<IndentationIndex>,
) -> Option<Ref<'a, IndentationIndex>> {
    let state = cache.borrow().state;
    if state == IndentationState::Fresh {
        build(tree, &mut cache.borrow_mut());
    }
    let index = cache.borrow();
    (index.state == IndentationState::Complete).then_some(index)
}
/// Match lexical delimiters, refusing malformed or incomplete token streams.
fn build(tree: &Tree<'_>, index: &mut IndentationIndex) {
    index.rows.clear();
    index.stack.clear();
    index.state = IndentationState::Incomplete;
    if languages::diagnostics(tree.elements).next().is_some() {
        return;
    }
    let Ok(length) = u32::try_from(tree.source.0.len()) else {
        return;
    };
    index.rows.push(IndentationRegion {
        id: Position(0),
        parent: Position(0),
        span: Span {
            start: Offset(0),
            end: Offset(length),
        },
        indentation_end: Offset(0),
    });
    for at in languages::positions(tree.elements) {
        match kind(tree, at) {
            Kind::Token(Token::Indent) => {
                let parent = index.stack.last().map_or(Position(0), |region| region.id);
                let start = languages::start_at(tree.elements, at);
                let end = languages::end_at(tree.elements, at);
                if start > end || end.0 > length {
                    return;
                }
                index.stack.push(IndentationRegion {
                    id: at,
                    parent,
                    span: Span { start, end },
                    indentation_end: end,
                });
            }
            Kind::Token(Token::Dedent) => {
                let Some(mut region) = index.stack.pop() else {
                    return;
                };
                let end = languages::end_at(tree.elements, at);
                if end < region.indentation_end || end.0 > length {
                    return;
                }
                region.span.end = end;
                index.rows.push(region);
            }
            Kind::Node(_) | Kind::Extra(_) | Kind::Token(_) => {}
        }
    }
    if !index.stack.is_empty() {
        return;
    }
    index.rows.sort_unstable_by_key(|region| region.id);
    index.state = IndentationState::Complete;
}
/// Intern the exact source prefix without decoding or normalizing tabs and form feeds.
pub(crate) fn prefix(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    region: &IndentationRegion,
) -> languages::Symbol {
    let start = languages::begin_name(symbols);
    languages::push_span(
        symbols,
        tree.source,
        region.span.start,
        region.indentation_end,
    );
    languages::end_name(symbols, tree.source, start)
}
