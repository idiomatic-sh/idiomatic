//! The `shapes` service: free functions named by verb that build a file's structural classes once, bottom-up, over the shared
//! key index, with no IO and no ports. Which words make a key is Rust's; interning them is the kernel's.

use crate::domain::model::types::ReadIndexes;
use crate::domain::nodes::types::Node;
use crate::domain::numeric_literals::constants::{NUMERIC_KIND, NUMERIC_SUFFIX, NUMERIC_VALUE};
use crate::domain::syntax::service::{is_token, kind};
use crate::domain::syntax::types::{Kind, Leaf};
use crate::domain::tokens::types::Token;
use ryo::languages::{
    self, NO_FIELD, NameText, Position, ShapeClasses, ShapeExtent, ShapeKey, ShapeStep, ShapeWord,
    Symbols, Tree, child_class, push_shape_word,
};

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

/// Builds the key of the node at `at` from its children's classes, already interned. A child's role is not part of it: the
/// children's order and the significant tokens between them say it.
fn key(tree: &Tree<'_>, at: Position, symbols: &mut Symbols, key: &mut ShapeKey<'_>) -> Option<()> {
    push_shape_word(key, ShapeWord::Kind(languages::kind_at(tree.elements, at)));
    for property in [NUMERIC_KIND, NUMERIC_VALUE, NUMERIC_SUFFIX] {
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

/// The structural classes of the file's own tree, built on the first call after the model was cleared: the expressions read
/// again from a macro's arguments are added after it, under their call, and are no child of any node of it. A parenthesized
/// node takes the class of the one node it holds. A tree the parser could not read completely has none: no two of its nodes
/// may seem equal for want of what it lost.
pub(crate) fn prepare<'a>(
    tree: &Tree<'_>,
    indexes: &'a ReadIndexes,
    symbols: &mut Symbols,
) -> Option<ShapeClasses<'a>> {
    languages::prepare_shapes(
        &indexes.shapes,
        tree.elements,
        ShapeExtent::OwnTree,
        |at, built| match parenthesized(tree, at) {
            Some(inner) => ShapeStep::SameAs(inner),
            None => match key(tree, at, symbols, built) {
                Some(()) => ShapeStep::Built,
                None => ShapeStep::Unbuilt,
            },
        },
    )
}
