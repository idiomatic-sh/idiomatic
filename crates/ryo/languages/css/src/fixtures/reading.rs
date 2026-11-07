//! Reading a source with the language's own parse, for the tests of the items of a relation: the rows the language writes of the item's
//! relation, each in full, and the causes for which it says the relation is read only in part.

use crate::inbound::contracts::parse::contract::ParseContract;
use crate::outbound::syntax::factory::cssparser;
use ryo::languages::contract::Reading;
use ryo::languages::{self, ByteSlice, ReadItem};
use std::sync::Arc;

/// The path every source is read as, which a file cell of a row shows.
pub(crate) const SHOWN: &str = "item.css";

/// A file read with the language's own parse: its table and its names, for its readers to be run on.
pub(crate) struct Parsed {
    /// The table.
    pub elements: languages::Elements,
    /// The bytes.
    pub bytes: &'static [u8],
}

/// Reads `bytes` with the language's own parse, as the file numbered 0.
///
/// # Errors
///
/// When the language refuses the bytes, as it does bytes that are not UTF-8.
pub(crate) fn parsed(bytes: &'static [u8]) -> Result<Parsed, languages::ParseError> {
    let parse = ParseContract::new(Arc::new(cssparser()));
    let (mut elements, mut symbols) = (
        languages::Elements::default(),
        languages::Symbols::default(),
    );
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
    Ok(Parsed { elements, bytes })
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
