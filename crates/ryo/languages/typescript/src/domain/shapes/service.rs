//! The `shapes` service: free functions named by verb that build a file's structural classes once, bottom-up, over the shared
//! key index, with no IO and no ports. Which words make a key is TypeScript's; interning them is the kernel's.

use crate::domain::fields::constants::PARENTHESES;
use crate::domain::model::types::ReadIndexes;
use crate::domain::names::service::text;
use crate::domain::syntax::constants::{NUMERIC_KIND, NUMERIC_VALUE};
use crate::domain::syntax::reading::{Interner, Syntax};
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
    tree: &impl Syntax,
    at: Position,
    symbols: &mut impl Interner,
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
            tree.kind_bits(at),
            match tree.decoded_of(at) {
                Some(decoded) => symbols.derived(tree.bytes(), decoded),
                None => tree.spelling(symbols, at),
            },
        )),
    }
}

/// Builds the key of the node at `at` from its children's classes, already interned. A child's role is not part of it: the
/// children's order and the significant tokens between them say it, and reading a role scans the siblings of every child.
fn key(tree: &impl Syntax, at: Position, symbols: &mut impl Interner, key: &mut ShapeKey<'_>) -> Option<()> {
    let mut built = Vec::new();
    words(tree, at, symbols, &mut |word| built.push(word), &|child| child_class(key, child))?;
    for word in built {
        push_shape_word(key, word);
    }
    Some(())
}

/// The words of the key of the node at `at`, each handed to `push`; `class` answers the class a child already has.
pub(crate) fn words(
    tree: &impl Syntax,
    at: Position,
    symbols: &mut impl Interner,
    push: &mut dyn FnMut(ShapeWord),
    class: &dyn Fn(Position) -> Option<Position>,
) -> Option<()> {
    push(ShapeWord::Kind(tree.kind_bits(at)));
    for property in [NUMERIC_KIND, NUMERIC_VALUE] {
        if let Some(value) = tree.property_of(at, property) {
            let value = symbols.derived(tree.bytes(), value);
            push(ShapeWord::Property(property, value));
        }
    }
    let numeric = tree.property_of(at, NUMERIC_VALUE);
    for child in tree.children_of(at) {
        let word = match is_token(tree, child) {
            Leaf::Node => ShapeWord::Child(NO_FIELD, class(child)?),
            Leaf::Token => match token_word(tree, child, symbols, numeric) {
                Some(word) => word,
                None => continue,
            },
        };
        push(word);
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
