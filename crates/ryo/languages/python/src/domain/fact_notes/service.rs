//! The `fact_notes` service: free functions named by verb over the domain's types, with no IO and no ports. It notes the imports a
//! file writes when the parse reads it, for the project to read; the port `notes` writes them as bytes and reads them back.

use ryo::languages::{self, Position, Tree};

use super::constants::{DOT, LOADERS};
use super::types::{DottedPath, Form, ImportNote, Level, Note, Spelling};
use crate::domain::extras::types::Extra;
use crate::domain::imports::service::imported;
use crate::domain::names::types::Imported;
use crate::domain::nodes::types::{Category, Node};
use crate::domain::scopes::service::nth;
use crate::domain::scopes::types::CategoryIndex;
use crate::domain::syntax::service::{below, child, kind, nodes, parent, written};
use crate::domain::syntax::types::{Kind, Text};
use crate::domain::tokens::types::Token;

/// Whether the string at `at` is an import specifier: the first argument of a call that loads a module by name, `__import__(..)` or
/// `importlib.import_module(..)` written as such. Python's `import` statements write no string.
#[must_use]
pub(crate) fn specifier(tree: &Tree<'_>, at: Position) -> languages::Flag {
    let arguments = parent(tree, at);
    let call = parent(tree, arguments);
    let loads = kind(tree, arguments) == Kind::Extra(Extra::CallArguments)
        && kind(tree, call) == Kind::Node(Node::Call)
        && nodes(tree, arguments).next() == Some(at)
        && nth(tree, call, Category::Expression, CategoryIndex(0))
            .is_some_and(|callee| LOADERS.contains(&written(tree, callee).0));
    if loads {
        languages::Flag::Raised
    } else {
        languages::Flag::Lowered
    }
}

/// The notes of the file whose tree is `tree`: one for each alias of its imports, in source order, as `static_import` reads them.
#[must_use]
pub(crate) fn noted(tree: &Tree<'_>) -> Note {
    let imports = languages::positions(tree.elements)
        .filter(|at| kind(tree, *at) == Kind::Node(Node::Alias))
        .filter_map(|alias| import_of(tree, alias))
        .collect();
    Note { imports }
}

/// The names the `NAME` tokens under the identifier at `identifier` write, in order: the parts of its dotted name.
fn parts_of<'a>(tree: &'a Tree<'_>, identifier: Position) -> impl Iterator<Item = Text<'a>> + 'a {
    below(tree, identifier)
        .filter(move |each| kind(tree, *each) == Kind::Token(Token::Name))
        .map(move |each| written(tree, each))
}

/// What the alias at `alias` imports: the dots and the module of its `from`, if it has one, then the name it writes.
fn import_of(tree: &Tree<'_>, alias: Position) -> Option<ImportNote> {
    let import = parent(tree, alias);
    let from = kind(tree, import) == Kind::Node(Node::ImportFrom);
    let mut level = 0_u32;
    let mut parts: Vec<Text<'_>> = Vec::new();
    if from {
        for each in languages::children(tree.elements, import) {
            let each_kind = kind(tree, each);
            if each_kind == Kind::Node(Node::Alias) {
                break;
            }
            if each_kind == Kind::Token(Token::Dot) {
                level = level.saturating_add(1);
            } else if each_kind == Kind::Token(Token::Ellipsis) {
                level = level.saturating_add(3);
            } else if each_kind == Kind::Extra(Extra::Identifier) {
                parts.extend(parts_of(tree, each));
            } else {
                // The `from` and `import` keywords and the white space between the parts say nothing of the module.
            }
        }
    }
    let form = if imported(tree, alias) == Imported::Everything {
        Form::Everything
    } else {
        parts.extend(parts_of(
            tree,
            child(tree, alias, Kind::Extra(Extra::Identifier))?,
        ));
        if from { Form::Name } else { Form::Module }
    };
    let mut path = String::new();
    for (place, part) in parts.iter().enumerate() {
        if place > 0 {
            path.push(DOT);
        }
        path.push_str(part.0);
    }
    let (start, end) = (
        languages::start_at(tree.elements, alias),
        languages::end_at(tree.elements, alias),
    );
    Some(ImportNote {
        form,
        level: Level(level),
        path: DottedPath(path.into()),
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
