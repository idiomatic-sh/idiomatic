//! The `fact_notes` service: free functions named by verb over the domain's types, with no IO and no ports. It notes the modules a file
//! imports when the parse reads it, for the project of the same run to read.

use ryo::languages::{self, Position, Tree};

use super::types::{ImportNote, ImportSpelling, Note, Specifier};
use crate::domain::names::constants::REQUIRE;
use crate::domain::names::service::{nodes, text};
use crate::domain::nodes::types::TypeScriptNode;
use crate::domain::syntax::service::{kind, parent};
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

/// The module the element at `at` imports, when it is a declaration or an expression that imports one by a string: the first string
/// literal among its nodes, or among those of its `require(...)`.
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "five kinds of node import a module by a string; every other kind imports none"
)]
fn import_of(tree: &Tree<'_>, at: Position) -> Option<ImportNote> {
    let holder = match kind(tree, at) {
        Kind::Node(
            TypeScriptNode::ImportDeclaration
            | TypeScriptNode::ExportAllDeclaration
            | TypeScriptNode::ExportNamedDeclaration
            | TypeScriptNode::ImportExpression,
        ) => at,
        Kind::Node(TypeScriptNode::TSImportEqualsDeclaration) => nodes(tree, at)
            .find(|each| kind(tree, *each) == Kind::Node(TypeScriptNode::TSExternalModuleReference))?,
        _ => return None,
    };
    let specifier = nodes(tree, holder).find_map(|each| string_at(tree, each))?;
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
fn spelled(tree: &Tree<'_>, start: languages::Offset, end: languages::Offset) -> ImportSpelling {
    let text = tree
        .source
        .0
        .get(
            usize::try_from(start.0).unwrap_or(usize::MAX)
                ..usize::try_from(end.0).unwrap_or(usize::MAX),
        )
        .map(String::from_utf8_lossy)
        .unwrap_or_default();
    ImportSpelling(text.split_whitespace().collect::<String>().into())
}

/// Whether the string literal at `at` is an import specifier: the source of an `import`, an `export .. from`, an `import(..)`, an
/// `import x = require(..)` or an `import(..)` type, or the first argument of a `require(..)` call.
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "six kinds of node hold the string an import names its module by; every other kind holds none"
)]
pub(crate) fn specifier(tree: &Tree<'_>, at: Position) -> languages::Flag {
    let enclosing = parent(tree, at);
    let imports = match kind(tree, enclosing) {
        Kind::Node(
            TypeScriptNode::ImportDeclaration
            | TypeScriptNode::ExportAllDeclaration
            | TypeScriptNode::ExportNamedDeclaration
            | TypeScriptNode::ImportExpression
            | TypeScriptNode::TSExternalModuleReference
            | TypeScriptNode::TSImportType,
        ) => true,
        Kind::Node(TypeScriptNode::TSLiteralType) => {
            kind(tree, parent(tree, enclosing)) == Kind::Node(TypeScriptNode::TSImportType)
        }
        Kind::Node(TypeScriptNode::CallExpression) => {
            let mut parts = nodes(tree, enclosing);
            parts.next().is_some_and(|callee| {
                kind(tree, callee) == Kind::Node(TypeScriptNode::Identifier)
                    && text(tree, callee).0 == REQUIRE
            }) && parts.next() == Some(at)
        }
        _ => false,
    };
    if imports {
        languages::Flag::Raised
    } else {
        languages::Flag::Lowered
    }
}
