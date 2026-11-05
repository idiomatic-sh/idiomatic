//! The `fact_notes` service: free functions named by verb over the domain's types, with no IO and no ports. It notes the modules a file
//! imports when the parse reads it, for the project to link the files of the run by.

use ryo::languages::{self, Position, Tree};

use super::types::{ImportNote, Note, Specifier, Spelling};
use crate::domain::nodes::types::Node;
use crate::domain::syntax::constants::REQUIRE;
use crate::domain::syntax::service::{kind, nodes, parent, spelling};
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;

/// The notes of the file whose tree is `tree`: each module it imports by a string, in source order.
#[must_use]
pub(crate) fn noted(tree: &Tree<'_>) -> Note {
    let imports = languages::positions(tree.elements)
        .filter_map(|at| import_of(tree, at))
        .collect();
    Note { imports }
}

/// The decoded value of the string literal at `at`, if it is one.
fn string_at<'a>(tree: &'a Tree<'_>, at: Position) -> Option<languages::NameText<'a>> {
    let token = languages::children(tree.elements, at)
        .find(|child| kind(tree, *child) == Kind::Token(Token::String))?;
    languages::decoded_at(tree.elements, token)
}

/// The string literal the call at `at` passes first, when it calls `require` by its name.
fn required(tree: &Tree<'_>, at: Position) -> Option<Position> {
    let mut parts = nodes(tree, at);
    let callee = parts.next()?;
    (kind(tree, callee) == Kind::Node(Node::Identifier) && spelling(tree, callee).0 == REQUIRE)
        .then(|| parts.next())
        .flatten()
}

/// Whether the string literal at `at` is an import specifier: the string an import, an `export .. from` or an `import(..)` names its
/// module by, or the first argument of a `require(..)`; the same strings the notes hold.
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "five kinds of node import a module by a string; every other kind imports none"
)]
pub(crate) fn specifier(tree: &Tree<'_>, at: Position) -> languages::Flag {
    let enclosing = parent(tree, at);
    let imports = match kind(tree, enclosing) {
        Kind::Node(
            Node::ImportDeclaration
            | Node::ExportAllDeclaration
            | Node::ExportNamedDeclaration
            | Node::ImportExpression,
        ) => true,
        Kind::Node(Node::CallExpression) => required(tree, enclosing) == Some(at),
        _ => false,
    };
    if imports {
        languages::Flag::Raised
    } else {
        languages::Flag::Lowered
    }
}

/// The module the element at `at` imports, when it is a declaration or an expression that imports one by a string: the first string
/// literal among its nodes, or the first argument of its `require(...)`.
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "five kinds of node import a module by a string; every other kind imports none"
)]
fn import_of(tree: &Tree<'_>, at: Position) -> Option<ImportNote> {
    let specifier = match kind(tree, at) {
        Kind::Node(
            Node::ImportDeclaration
            | Node::ExportAllDeclaration
            | Node::ExportNamedDeclaration
            | Node::ImportExpression,
        ) => nodes(tree, at).find_map(|each| string_at(tree, each))?,
        Kind::Node(Node::CallExpression) => string_at(tree, required(tree, at)?)?,
        _ => return None,
    };
    let (start, end) = (
        languages::start_at(tree.elements, at),
        languages::end_at(tree.elements, at),
    );
    Some(ImportNote {
        specifier: Specifier(specifier.0.into()),
        start,
        end,
        text: spelled(tree, start, end),
    })
}

/// The text from `start` to `end` as the file writes it, its whitespace left out.
fn spelled(tree: &Tree<'_>, start: languages::Offset, end: languages::Offset) -> Spelling {
    let text = tree
        .source
        .0
        .get(
            usize::try_from(start.0).unwrap_or(usize::MAX)
                ..usize::try_from(end.0).unwrap_or(usize::MAX),
        )
        .map(String::from_utf8_lossy)
        .unwrap_or_default();
    Spelling(text.split_whitespace().collect::<String>().into())
}
