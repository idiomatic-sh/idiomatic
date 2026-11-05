//! The arguments a decorator of the TypeScript tree passes.
use crate::domain::names::constants::{
    BLOCK_COMMENT, BLOCK_END, CLOSE_PARAMETERS, LINE_COMMENT, OPEN_PARAMETERS,
};
use crate::domain::names::service::nodes;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::{service::kind, types::Kind};
use ryo::languages::{self, ByteSlice, Offset, Position, Symbol, Symbols, Tree};
/// The arguments the decorator at `at` passes, as written between the parentheses of its call, or nothing when it is no call.
pub(crate) fn arguments(tree: &Tree<'_>, symbols: &mut Symbols, at: Position) -> Symbol {
    let call = nodes(tree, at).find(|child| kind(tree, *child) == Kind::Node(Node::CallExpression));
    let between = call.and_then(|call| {
        let mut parts = nodes(tree, call);
        let callee = parts.next()?;
        let mut after = languages::end_at(tree.elements, callee);
        if let Some(generic) = parts.next()
            && kind(tree, generic) == Kind::Node(Node::TSTypeParameterInstantiation)
        {
            after = languages::end_at(tree.elements, generic);
        }
        // The call's own bytes are read, not the tokens its children hold: a decorator written before `export` keeps its parentheses
        // in the tokens of the node around it.
        let close = usize::try_from(languages::end_at(tree.elements, call).0)
            .unwrap_or(usize::MAX)
            .checked_sub(1)?;
        let from = usize::try_from(after.0).unwrap_or(usize::MAX);
        let bytes = tree.source.0;
        if bytes.get(close) != CLOSE_PARAMETERS.as_bytes().first() {
            return None;
        }
        let open = usize::try_from(open_parenthesis(ByteSlice(bytes.get(from..close)?))?.0)
            .unwrap_or(usize::MAX)
            .checked_add(from)?;
        Some((
            Offset(u32::try_from(open.checked_add(1)?).unwrap_or(u32::MAX)),
            Offset(u32::try_from(close).unwrap_or(u32::MAX)),
        ))
    });
    let begun = languages::begin_name(symbols);
    if let Some((from, to)) = between {
        languages::push_span(symbols, tree.source, from, to);
    }
    languages::end_name(symbols, tree.source, begun)
}

/// Where the first parenthesis that is code is in `bytes`: one inside a block comment or a line comment between the callee and the
/// arguments is not it.
fn open_parenthesis(bytes: ByteSlice<'_>) -> Option<Offset> {
    let bytes = bytes.0;
    let mut at = 0;
    while at < bytes.len() {
        let rest = bytes.get(at..)?;
        if let Some(inside) = rest.strip_prefix(BLOCK_COMMENT.as_bytes()) {
            let end = inside
                .windows(BLOCK_END.len())
                .position(|pair| pair == BLOCK_END.as_bytes())?;
            at += BLOCK_COMMENT.len() + end + BLOCK_END.len();
        } else if let Some(inside) = rest.strip_prefix(LINE_COMMENT.as_bytes()) {
            at += LINE_COMMENT.len() + inside.iter().position(|each| *each == b'\n')? + 1;
        } else if rest.starts_with(OPEN_PARAMETERS.as_bytes()) {
            return Some(Offset(u32::try_from(at).unwrap_or(u32::MAX)));
        } else {
            at += 1;
        }
    }
    None
}
