//! The `trees` service: free functions named by verb that write a tree from a library's events, with no IO and no ports. An event
//! gives a node or a token with its span; the text from where the last token ends to where the event starts is the gap, which the
//! language's gap tokenizer writes as its own tokens (white space, markup, text its library could not read) before the event. A
//! gap tokenizer is given the builder and the stretch, and writes tokens that cover the stretch, every byte in one, in order.

use super::types::{Lossless, ReadableText};
use crate::domain::elements::service::{decoded_token, enter_node, leave_node, token};
use crate::domain::elements::types::TableBuilder;
use crate::domain::symbols::types::ByteSlice;
use ryo_relations::domain::relations::types::{KindBits, Offset};
use ryo_relations::domain::terms::types::NameText;

/// A tree written through `builder`, its tokens starting at `start`: zero for a file, or where the text the library reads starts.
#[must_use]
pub const fn lossless<'b, 'w>(
    builder: &'b mut TableBuilder<'w>,
    start: Offset,
) -> Lossless<'b, 'w> {
    Lossless {
        builder,
        end: start,
    }
}

/// Writes the gap from where the last token ends to `to` with the language's `gap` tokenizer, when there is one.
pub fn fill_to(
    tree: &mut Lossless<'_, '_>,
    to: Offset,
    gap: &mut impl FnMut(&mut TableBuilder<'_>, Offset, Offset),
) {
    if to > tree.end {
        gap(tree.builder, tree.end, to);
        tree.end = to;
    }
}

/// Writes a token of `kind` from `start` to `end`, after the gap before it. A token that starts before the last one ends starts
/// where it ends, so no byte is in two tokens.
pub fn lossless_token(
    tree: &mut Lossless<'_, '_>,
    kind: KindBits,
    start: Offset,
    end: Offset,
    gap: &mut impl FnMut(&mut TableBuilder<'_>, Offset, Offset),
) {
    fill_to(tree, start, gap);
    let start = start.max(tree.end);
    let end = end.max(start);
    token(tree.builder, kind, start, end);
    tree.end = end;
}

/// Writes a token of `kind` from `start` to `end` with the text `decoded` its library read from it, after the gap before it, as
/// [`lossless_token`] places it.
pub fn lossless_decoded_token(
    tree: &mut Lossless<'_, '_>,
    kind: KindBits,
    start: Offset,
    end: Offset,
    decoded: NameText<'_>,
    gap: &mut impl FnMut(&mut TableBuilder<'_>, Offset, Offset),
) {
    fill_to(tree, start, gap);
    let start = start.max(tree.end);
    let end = end.max(start);
    decoded_token(tree.builder, kind, start, end, decoded);
    tree.end = end;
}

/// Opens a node of `kind` at `start`, after the gap before it, or where the last token ends when the event starts before it.
pub fn lossless_enter(
    tree: &mut Lossless<'_, '_>,
    kind: KindBits,
    start: Offset,
    gap: &mut impl FnMut(&mut TableBuilder<'_>, Offset, Offset),
) {
    fill_to(tree, start, gap);
    enter_node(tree.builder, kind, start.max(tree.end));
}

/// Closes the innermost node at `end`, after the gap before it, or where the last token ends when that is later.
pub fn lossless_leave(
    tree: &mut Lossless<'_, '_>,
    end: Offset,
    gap: &mut impl FnMut(&mut TableBuilder<'_>, Offset, Offset),
) {
    fill_to(tree, end, gap);
    leave_node(tree.builder, end.max(tree.end));
}

/// The text of `bytes` a library that reads UTF-8 reads: all of it, or the bytes before the first that is not UTF-8.
#[must_use]
pub fn readable_text(bytes: ByteSlice<'_>) -> ReadableText<'_> {
    ReadableText(match std::str::from_utf8(bytes.0) {
        Ok(text) => text,
        Err(error) => std::str::from_utf8(bytes.0.get(..error.valid_up_to()).unwrap_or_default())
            .unwrap_or_default(),
    })
}
