//! The `syntax` service: free functions named by verb over the domain's types, with no IO and no
//! ports. It maps a kind to the number the element table holds and back, says its name, and reads the kind of an element of a file's
//! syntax tree: a position is an element of the table; its kind is the language's own.

use ryo::languages::{self, Tree};

use super::constants::{DOC_OPEN, EMPTY_BLOCK, RECEIVER};
use super::types::{Kind, Leaf};
use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::Node;
use crate::domain::tokens::types::Token;

ryo::languages::kinds!(pub Kind { Node(Node), Token(Token), Extra(Extra) } unknown Kind::Extra(Extra::Error));

/// The kind of the element at `at`.
#[must_use]
pub(crate) fn kind(tree: &Tree<'_>, at: languages::Position) -> Kind {
    kind_of(languages::kind_at(tree.elements, at))
}

/// Whether the element at `at` is a token, a leaf, and not a node.
#[must_use]
pub(crate) fn is_token(tree: &Tree<'_>, at: languages::Position) -> Leaf {
    if languages::flags_at(tree.elements, at).0 & languages::TOKEN.0 == 0 {
        Leaf::Node
    } else {
        Leaf::Token
    }
}

/// The parent of the element at `at`; the root is its own.
#[must_use]
pub(crate) fn parent(tree: &Tree<'_>, at: languages::Position) -> languages::Position {
    languages::parent_at(tree.elements, at)
}

/// The name of the kind of the element at `at`, as the `kind` column of the tree holds it.
pub(crate) fn kind_symbol(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    at: languages::Position,
) -> languages::Symbol {
    let element = kind(tree, at);
    languages::vocabulary_symbol(symbols, tree.source, kind_bits(element), kind_name(element))
}

/// The text of the token at `at`; no text for a node.
#[must_use]
pub(crate) fn token_text(tree: &Tree<'_>, at: languages::Position) -> languages::Symbol {
    if is_token(tree, at) == Leaf::Token {
        languages::text_at(tree.elements, at)
    } else {
        languages::NO_SYMBOL
    }
}

/// The last element below the element at `at`, in pre-order; `at` itself when it has none.
#[must_use]
pub(crate) fn last(tree: &Tree<'_>, at: languages::Position) -> languages::Position {
    languages::last_at(tree.elements, at)
}

/// The bytes of the element at `at`, as the file spells them.
fn written<'a>(tree: &Tree<'a>, at: languages::Position) -> languages::NameText<'a> {
    let from = usize::try_from(languages::start_at(tree.elements, at).0).unwrap_or(usize::MAX);
    let to = usize::try_from(languages::end_at(tree.elements, at).0).unwrap_or(usize::MAX);
    languages::NameText(
        tree.source
            .0
            .get(from..to)
            .and_then(|bytes| std::str::from_utf8(bytes).ok()) // the bytes of a node that are not text have no text, which the reader takes as an empty name: the row keeps its place either way
            .unwrap_or_default(),
    )
}

/// Whether the comment at `at` is documentation, as TypeScript reads it: a block comment that opens with `/**`, the empty `/**/`
/// aside.
#[must_use]
pub(crate) fn documentation(tree: &Tree<'_>, at: languages::Position) -> languages::Flag {
    let raw = written(tree, at).0;
    if raw.starts_with(DOC_OPEN) && raw != EMPTY_BLOCK {
        languages::Flag::Raised
    } else {
        languages::Flag::Lowered
    }
}

/// How the parameter at `at` takes its argument: a rest element takes the arguments left over, a `this` parameter types the
/// receiver, and every other is positional.
#[must_use]
pub(crate) fn parameter_kind(tree: &Tree<'_>, at: languages::Position) -> languages::ParameterKind {
    match kind(tree, at) {
        Kind::Node(Node::RestElement) => languages::ParameterKind::VariadicPositional,
        Kind::Node(Node::Identifier)
            if written(tree, at)
                .0
                .strip_prefix(RECEIVER)
                .is_some_and(|rest| {
                    rest.chars().next().is_none_or(|character| {
                        !(character == '_' || character == '$' || character.is_alphanumeric())
                    })
                }) =>
        {
            languages::ParameterKind::Receiver
        }
        Kind::Node(_) | Kind::Token(_) | Kind::Extra(_) => languages::ParameterKind::Positional,
    }
}
