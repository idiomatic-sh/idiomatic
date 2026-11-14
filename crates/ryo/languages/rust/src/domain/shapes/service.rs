//! The `shapes` service: free functions named by verb that build a file's structural classes once, bottom-up, over the shared
//! key index, with no IO and no ports. Which words make a key is Rust's; interning them is the kernel's.

use crate::domain::model::types::ReadIndexes;
use crate::domain::nodes::types::Node;
use crate::domain::numeric_literals::constants::{NUMERIC_KIND, NUMERIC_SUFFIX, NUMERIC_VALUE};
use crate::domain::syntax::service::{is_token, kind};
use crate::domain::syntax::types::{Kind, Leaf};
use crate::domain::tokens::types::Token;
use ryo::languages::{
    self, KeyState, NO_FIELD, NameText, Position, RetainedKeyIndex, ShapeWord, Symbols, Tree,
    intern_key, key_class_at, push_key_word,
};
use std::cell::Ref;

/// The word a token adds to its node's key; none for white space, a comment, or the digits of a numeric literal whose value the
/// node already holds.
fn token_word(
    tree: &Tree<'_>,
    at: Position,
    symbols: &mut Symbols,
    numeric: Option<NameText<'_>>,
) -> Option<ShapeWord> {
    match kind(tree, at) {
        Kind::Token(Token::Whitespace | Token::Newline | Token::Comment)
        | Kind::Extra(_)
        | Kind::Node(_) => None,
        Kind::Token(Token::IntNumber | Token::FloatNumber) if numeric.is_some() => None,
        Kind::Token(_) => Some(ShapeWord::Token(
            languages::kind_at(tree.elements, at),
            match languages::decoded_at(tree.elements, at) {
                Some(decoded) => languages::intern_derived(symbols, tree.source, decoded),
                None => languages::text_at(tree.elements, at),
            },
        )),
    }
}

/// The one node a parenthesized expression, pattern or type holds, whose class it takes; none for any other node.
fn parenthesized(tree: &Tree<'_>, at: Position) -> Option<Position> {
    if !matches!(
        kind(tree, at),
        Kind::Node(Node::ParenExpr | Node::ParenPat | Node::ParenType)
    ) {
        return None;
    }
    let mut held =
        languages::children(tree.elements, at).filter(|child| is_token(tree, *child) == Leaf::Node);
    match (held.next(), held.next()) {
        (Some(only), None) => Some(only),
        (Some(_), Some(_)) | (None, _) => None,
    }
}

/// Builds the key of the node at `at` in the index's scratch from its children's classes, already interned. A child's role is
/// not part of it: the children's order and the significant tokens between them say it.
fn key(
    tree: &Tree<'_>,
    at: Position,
    symbols: &mut Symbols,
    index: &mut RetainedKeyIndex<Position, ShapeWord>,
) -> Option<()> {
    index.scratch.clear();
    push_key_word(
        index,
        ShapeWord::Kind(languages::kind_at(tree.elements, at)),
    )?;
    for property in [NUMERIC_KIND, NUMERIC_VALUE, NUMERIC_SUFFIX] {
        if let Some(value) = languages::property_at(tree.elements, at, property) {
            let value = languages::intern_derived(symbols, tree.source, value);
            push_key_word(index, ShapeWord::Property(property, value))?;
        }
    }
    let numeric = languages::property_at(tree.elements, at, NUMERIC_VALUE);
    for child in languages::children(tree.elements, at) {
        let word = match is_token(tree, child) {
            Leaf::Node => ShapeWord::Child(NO_FIELD, key_class_at(index, child)?),
            Leaf::Token => match token_word(tree, child, symbols, numeric) {
                Some(word) => word,
                None => continue,
            },
        };
        push_key_word(index, word)?;
    }
    Some(())
}

/// The structural classes of the file's own tree, built on the first call after the model was cleared. A tree the parser could
/// not read completely has none: no two of its nodes may seem equal for want of what it lost.
pub(crate) fn prepare<'a>(
    tree: &Tree<'_>,
    indexes: &'a ReadIndexes,
    symbols: &mut Symbols,
) -> Option<Ref<'a, RetainedKeyIndex<Position, ShapeWord>>> {
    let held = &indexes.shapes;
    if held.borrow().state == KeyState::Fresh {
        let mut index = held.borrow_mut();
        if languages::has_incomplete_syntax(tree.elements)
            == languages::SyntaxCompleteness::Incomplete
        {
            index.state = KeyState::Partial;
            return None;
        }
        // The file's own tree, without the expressions read again from a macro's arguments: those are added after it, under their
        // call, and are no child of any node of it.
        let count = languages::positions(tree.elements).count();
        let missing = count.saturating_sub(index.classes.len());
        if index.classes.try_reserve(missing).is_err() {
            index.state = KeyState::Partial;
            return None;
        }
        index.classes.resize(count, None);
        for raw in (0..count).rev() {
            let Ok(position) = u32::try_from(raw) else {
                return None;
            };
            let at = Position(position);
            if is_token(tree, at) == Leaf::Token {
                continue;
            }
            let class = match parenthesized(tree, at) {
                Some(inner) => key_class_at(&*index, inner),
                None => {
                    if key(tree, at, symbols, &mut index).is_none() {
                        index.state = KeyState::Partial;
                        continue;
                    }
                    Some(intern_key(&mut index, at))
                }
            };
            if let Some(slot) = index.classes.get_mut(raw) {
                *slot = class;
            }
        }
        if index.state == KeyState::Fresh {
            index.state = KeyState::Complete;
        }
    }
    let index = held.borrow();
    (index.state == KeyState::Complete).then_some(index)
}
