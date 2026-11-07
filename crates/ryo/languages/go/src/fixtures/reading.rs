//! Reading a source with the language's own parse, for the tests of the items of a relation: the rows the language writes of the
//! item's relation, each in full, and the causes for which it says the relation is read only in part.

use ryo::languages::{self, ByteSlice, ReadItem, Symbols};
use std::sync::Arc;

use crate::inbound::contracts::parse::contract::ParseContract;
use crate::outbound::syntax::factory::tree_sitter_go;
use ryo::languages::contract::Reading;

/// The path every source is read as, which a file cell of a row shows.
pub(crate) const SHOWN: &str = "item.go";

/// The path every module file is read as, which a file cell of a row shows.
pub(crate) const MODULE_SHOWN: &str = "go.mod";

/// What the item writes for the module file `source`, read with the language's own parse as a `go.mod`.
///
/// # Panics
///
/// When the source cannot be read.
#[must_use]
pub(crate) fn module_reading<Item>(source: &'static str) -> Reading
where
    Item: ReadItem,
{
    languages::contract::reading::<Item>(
        &crate::linked(),
        languages::SourcePath(MODULE_SHOWN),
        languages::SampleText(source),
    )
    .expect("the module file is read")
}

/// What the item writes for `source`, read with the language's own parse.
///
/// # Panics
///
/// When the source cannot be read.
#[must_use]
pub(crate) fn reading<Item>(source: &'static str) -> Reading
where
    Item: ReadItem,
{
    languages::contract::reading::<Item>(
        &crate::linked(),
        languages::SourcePath(SHOWN),
        languages::SampleText(source),
    )
    .expect("the source is read")
}

/// A file read with the language's own parse: its table and its names, for the services to be run on.
pub(crate) struct Parsed {
    /// The table.
    pub elements: languages::Elements,
    /// The names.
    pub symbols: Symbols,
    /// The bytes.
    pub bytes: &'static [u8],
}

/// Reads `bytes` with the language's own parse, as the file numbered 0.
///
/// # Errors
///
/// When the language refuses the bytes, which it never does for a go file.
#[cfg(test)]
pub(crate) fn parsed(bytes: &'static [u8]) -> Result<Parsed, languages::ParseError> {
    let parse = ParseContract::new(
        Arc::new(tree_sitter_go()),
        Arc::new(crate::outbound::notes::factory::postcard()),
    );
    let (mut elements, mut symbols) = (languages::Elements::default(), Symbols::default());
    languages::contract::build(
        &parse,
        languages::ParseRequest {
            path: languages::SourcePath(SHOWN),
            file: languages::FileId(0),
            bytes: ByteSlice(bytes),
            notes: languages::NoteRequest::Unwanted,
            fields: languages::FieldRequest::Unwanted,
        },
        &mut elements,
        &mut symbols,
    )?;
    Ok(Parsed {
        elements,
        symbols,
        bytes,
    })
}

/// What `question` says of the tree of `source`, read with the language's own parse.
///
/// # Panics
///
/// When the source cannot be read, which never happens for a go file.
#[cfg(test)]
pub(crate) fn asked<Answer>(
    source: &'static str,
    question: impl FnOnce(&languages::Tree<'_>) -> Answer,
) -> Answer {
    let file = parsed(source.as_bytes()).expect("the source is read");
    question(&languages::Tree {
        elements: &file.elements,
        source: ByteSlice(file.bytes),
        file: languages::FileId(0),
    })
}

/// Every element of the tree whose kind is `wanted`, in order.
#[cfg(test)]
pub(crate) fn of_kind(
    tree: &languages::Tree<'_>,
    wanted: crate::domain::syntax::types::Kind,
) -> Vec<languages::Position> {
    languages::positions(tree.elements)
        .filter(|at| crate::domain::syntax::service::kind(tree, *at) == wanted)
        .collect()
}

/// The text the element at `at` spans.
#[cfg(test)]
pub(crate) fn spans(tree: &languages::Tree<'_>, at: languages::Position) -> String {
    crate::domain::syntax::service::written(tree, at)
        .0
        .to_owned()
}

/// The kinds of the nodes of `go/ast` that `build` finds over the library's tree of `source`, in the order it finds them: what one
/// service of the domain finds, and nothing of the others.
///
/// # Panics
///
/// When the source cannot be read, which never happens for a go file.
#[cfg(test)]
pub(crate) fn found_by(
    source: &'static str,
    build: impl FnOnce(
        &crate::domain::grammar::types::Concrete<'_>,
        &mut crate::domain::pieces::types::Pieces,
    ),
) -> String {
    use crate::domain::syntax::service::kind_name;
    use crate::domain::syntax::types::Kind;
    let tree =
        crate::outbound::syntax::fixtures::library::concreted(source).expect("the source is read");
    let mut pieces = crate::domain::pieces::types::Pieces::default();
    build(&tree, &mut pieces);
    pieces
        .nodes
        .iter()
        .map(|placed| kind_name(Kind::Node(placed.kind)).0)
        .collect::<Vec<_>>()
        .join(" ")
}
