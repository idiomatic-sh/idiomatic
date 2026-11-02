//! Reading a source with the language's own parse, for the tests of the items of a relation: the rows the language writes of the item's
//! relation, each in full, and the causes for which it says the relation is read only in part.

use crate::inbound::contracts::parse::contract::ParseContract;
use crate::outbound::notes::factory::postcard;
use crate::outbound::syntax::factory::ruff_python_parser;
#[cfg(test)]
use ryo::languages::ReadItem;
#[cfg(test)]
use ryo::languages::contract::Reading;
use ryo::languages::{self, ByteSlice, Symbols};
use std::sync::Arc;

/// The path every source is read as, which a file cell of a row shows.
pub(crate) const SHOWN: &str = "item.py";

/// A file read with the language's own parse: its table and its names, for its readers to be run on.
pub(crate) struct Parsed<'a> {
    /// The table.
    pub elements: languages::Elements,
    /// The names.
    pub symbols: Symbols,
    /// The bytes.
    pub bytes: &'a [u8],
}

/// Reads `bytes` with the language's own parse, as the file numbered 0.
///
/// # Errors
///
/// When the language refuses the bytes, which it never does for a Python file.
pub(crate) fn parsed(bytes: &[u8]) -> Result<Parsed<'_>, languages::ParseError> {
    let parse = ParseContract::new(
        crate::settings::PythonParserSettings::default(),
        Arc::new(ruff_python_parser()),
        Arc::new(postcard()),
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

/// What the item writes for `source`, read with the language's own parse.
///
/// # Panics
///
/// When the source cannot be read.
#[must_use]
#[cfg(test)]
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

/// An expansion-aware semantic reader must publish the same visible node, file, and
/// independently expected source coordinates as `semantic-syntax`.
///
/// The caller supplies the relation's node, file, and span columns explicitly: a
/// lexical reader must not gain expansion participation merely by using this test.
#[cfg(test)]
pub(crate) fn assert_expanded_node_coordinate<Item>(
    source: &'static str,
    columns: (usize, usize, usize),
    expected_kind: languages::contract::Want,
    expected_span: languages::contract::Want,
) where
    Item: ReadItem,
{
    use crate::domain::relations::collections::relations::semantic_syntax::item::SemanticSyntaxItem;

    let relation = reading::<Item>(source);
    let syntax = reading::<SemanticSyntaxItem>(source);
    assert!(relation.causes.is_empty() && syntax.causes.is_empty());
    let matching: Vec<_> = relation
        .rows
        .iter()
        .filter(|row| row.get(columns.2) == Some(&expected_span))
        .collect();
    assert_eq!(
        matching.len(),
        1,
        "one relation row at the expected expanded span"
    );
    let row = matching[0];
    assert!(
        syntax.rows.iter().any(|element| {
            element.first() == row.get(columns.0)
                && element.get(1) == Some(&expected_kind)
                && element.get(3) == row.get(columns.1)
                && element.get(4) == Some(&expected_span)
        }),
        "expanded relation node must retain its semantic kind, file, and source span"
    );
}

/// Exercise the provider's real model initialization without requesting any semantic relation.
pub(crate) fn initialize_model(
    tree: &languages::Tree<'_>,
    model: &mut crate::domain::model::types::Declarations,
) {
    use languages::Parse as _;
    ParseContract::new(
        crate::settings::PythonParserSettings::default(),
        Arc::new(ruff_python_parser()),
        Arc::new(postcard()),
    )
    .file_model(tree, model);
}

/// Exercise an authored reader directly before its optional capability is advertised.
/// Uses the same typed reader and shared row collector; production capability gating is unchanged.
#[must_use]
#[cfg(test)]
pub(crate) fn proposed<Item>(source: &'static str) -> Reading
where
    Item: ReadItem<Model = crate::domain::model::types::Declarations>,
    languages::contract::Collect:
        languages::Rows<<Item::Relation as languages::CatalogRelation>::Row>,
{
    use languages::{CatalogRelation as _, Parse as _};
    let mut file = parsed(source.as_bytes()).expect("Python retains malformed source diagnostics");
    let tree = languages::Tree {
        elements: &file.elements,
        source: ByteSlice(file.bytes),
        file: languages::FileId(0),
    };
    let parse = ParseContract::new(
        crate::settings::PythonParserSettings::default(),
        Arc::new(ruff_python_parser()),
        Arc::new(postcard()),
    );
    let mut model = crate::domain::model::types::Declarations::default();
    parse.file_model(&tree, &mut model);
    let mut out = languages::contract::Collect::default();
    for at in languages::positions(tree.elements) {
        let selected = match Item::READER.reads {
            languages::Reads::Every => true,
            languages::Reads::Kinds(kinds) => {
                kinds.contains(&languages::kind_at(tree.elements, at))
            }
        };
        if selected {
            Item::READER
                .visit
                .visit(&tree, &model, &mut file.symbols, at, &mut out);
        }
    }
    languages::contract::written(
        out,
        file.symbols,
        (&[SHOWN], file.bytes),
        (languages::SourcePath(SHOWN), Item::Relation::relation()),
        parse.unknown_reasons(),
    )
}
