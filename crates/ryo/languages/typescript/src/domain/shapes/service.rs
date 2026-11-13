//! The `shapes` service: free functions named by verb that build a file's structural classes once, bottom-up, over the shared
//! key index, with no IO and no ports. Which words make a key is TypeScript's; interning them is the kernel's.

use crate::domain::fields::constants::PARENTHESES;
use crate::domain::model::types::ReadIndexes;
use crate::domain::names::service::text;
use crate::domain::syntax::constants::{NUMERIC_KIND, NUMERIC_VALUE};
use crate::domain::syntax::service::{is_token, kind};
use crate::domain::syntax::types::{Kind, Leaf};
use crate::domain::tokens::types::Token;
use ryo::languages::{
    self, KeyState, NO_FIELD, Position, RetainedKeyIndex, ShapeWord, Symbols, Tree, intern_key,
    key_class_at, push_key_word,
};
use std::cell::Ref;

/// The word a token adds to its node's key; none for a comment, a gap, a parenthesis, or the digits of a numeric literal whose
/// value the node already holds.
fn token_word(
    tree: &Tree<'_>,
    at: Position,
    symbols: &mut Symbols,
    numeric: Option<languages::NameText<'_>>,
) -> Option<ShapeWord> {
    match kind(tree, at) {
        Kind::Token(Token::Line | Token::Block)
        | Kind::Extra(_)
        | Kind::Node(_)
        | Kind::Pattern(_) => None,
        Kind::Token(Token::Numeric) if numeric.is_some() => None,
        Kind::Token(Token::Punctuator)
            if PARENTHESES
                .iter()
                .any(|marker| marker.0 == text(tree, at).0) =>
        {
            None
        }
        Kind::Token(_) => Some(ShapeWord::Token(
            languages::kind_at(tree.elements, at),
            match languages::decoded_at(tree.elements, at) {
                Some(decoded) => languages::intern_derived(symbols, tree.source, decoded),
                None => languages::text_at(tree.elements, at),
            },
        )),
    }
}

/// Builds the key of the node at `at` in the index's scratch from its children's classes, already interned. A child's role is
/// not part of it: the children's order and the significant tokens between them say it, and reading a role scans the siblings
/// of every child.
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
    for property in [NUMERIC_KIND, NUMERIC_VALUE] {
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

/// The file's structural classes, built on the first call after the model was cleared. A tree the parser could not read
/// completely has none: no two of its nodes may seem equal for want of what it lost.
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
        let Ok(count) = usize::try_from(languages::end_of(tree.elements).0) else {
            return None;
        };
        if index
            .classes
            .try_reserve(count.saturating_sub(index.classes.len()))
            .is_err()
        {
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
            if key(tree, at, symbols, &mut index).is_none() {
                index.state = KeyState::Partial;
                continue;
            }
            let class = intern_key(&mut index, at);
            if let Some(slot) = index.classes.get_mut(raw) {
                *slot = Some(class);
            }
        }
        if index.state == KeyState::Fresh {
            index.state = KeyState::Complete;
        }
    }
    let index = held.borrow();
    (index.state == KeyState::Complete).then_some(index)
}
