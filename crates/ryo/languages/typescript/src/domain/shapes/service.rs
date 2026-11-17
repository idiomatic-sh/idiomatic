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
    self, NO_FIELD, Position, ShapeClasses, ShapeExtent, ShapeKey, ShapeStep, ShapeWord, Symbols,
    Tree, child_class, push_shape_word,
};

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

/// Builds the key of the node at `at` from its children's classes, already interned. A child's role is not part of it: the
/// children's order and the significant tokens between them say it, and reading a role scans the siblings of every child.
fn key(tree: &Tree<'_>, at: Position, symbols: &mut Symbols, key: &mut ShapeKey<'_>) -> Option<()> {
    push_shape_word(key, ShapeWord::Kind(languages::kind_at(tree.elements, at)));
    for property in [NUMERIC_KIND, NUMERIC_VALUE] {
        if let Some(value) = languages::property_at(tree.elements, at, property) {
            let value = languages::intern_derived(symbols, tree.source, value);
            push_shape_word(key, ShapeWord::Property(property, value));
        }
    }
    let numeric = languages::property_at(tree.elements, at, NUMERIC_VALUE);
    for child in languages::children(tree.elements, at) {
        let word = match is_token(tree, child) {
            Leaf::Node => ShapeWord::Child(NO_FIELD, child_class(key, child)?),
            Leaf::Token => match token_word(tree, child, symbols, numeric) {
                Some(word) => word,
                None => continue,
            },
        };
        push_shape_word(key, word);
    }
    Some(())
}

/// The file's structural classes, built on the first call after the model was cleared. A tree the parser could not read
/// completely has none: no two of its nodes may seem equal for want of what it lost.
pub(crate) fn prepare<'a>(
    tree: &Tree<'_>,
    indexes: &'a ReadIndexes,
    symbols: &mut Symbols,
) -> Option<ShapeClasses<'a>> {
    languages::prepare_shapes(
        &indexes.shapes,
        tree.elements,
        ShapeExtent::Table,
        |at, built| match key(tree, at, symbols, built) {
            Some(()) => ShapeStep::Built,
            None => ShapeStep::Unbuilt,
        },
    )
}
