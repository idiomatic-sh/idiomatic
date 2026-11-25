//! The `fact_notes` service: free functions named by verb over the domain's types, with no IO and no ports. It notes the packages a file
//! imports when the parse reads it, for the project to read; the bytes the notes are written as are the port `notes`'s.

use ryo::languages::{self, Position, node_table};

use crate::domain::fact_notes::constants::LINE_END;
use crate::domain::fact_notes::types::{ImportNote, ImportPath, Note, Spelling};
use crate::domain::file_view::types::Syntax as Tree;
use crate::domain::syntax::service::kind_bits;
use crate::domain::walked::names::string_token;
use crate::domain::nodes::types::Node;
use crate::domain::walked::syntax::{decoded_at, end_at, nodes, start_at};
use crate::domain::syntax::types::Kind;

/// The notes of the file whose tree is `tree`: each package it imports, one for each spec of its `import` declarations, in source
/// order.
#[must_use]
pub(crate) fn noted(tree: &Tree<'_>) -> Note {
    let imports = node_table::ids_of_kind(tree.table, kind_bits(Kind::Node(Node::ImportSpec)))
        .iter()
        .filter_map(|at| import_of(tree, *at))
        .collect();
    Note { imports }
}

/// The package the import spec at `at` names: the decoded text of its path, the last node of the spec; none for a path that is no text
/// or holds a line end.
fn import_of(tree: &Tree<'_>, at: Position) -> Option<ImportNote> {
    let path = nodes(tree, at).last()?;
    let decoded = decoded_at(tree, string_token(tree, path)?)?;
    if decoded.contains(LINE_END) {
        return None;
    }
    let (start, end) = (
        start_at(tree, at),
        end_at(tree, at),
    );
    Some(ImportNote {
        path: ImportPath(decoded.into()),
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
