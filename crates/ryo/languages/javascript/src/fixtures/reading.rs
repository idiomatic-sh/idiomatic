//! Reading a source with the language's own parse, for the tests of the items of a relation: the rows the language writes of the
//! item's relation, each in full, and the causes for which it says the relation is read only in part.

use ryo::languages::{self, ReadItem};

use ryo::languages::contract::Reading;

/// The path every source is read as, which a file cell of a row shows.
pub(crate) const SHOWN: &str = "item.js";

/// What the item writes for `source`, read with the language's own parse as the file at `path`.
///
/// # Panics
///
/// When the source cannot be read.
#[must_use]
pub(crate) fn reading_at<Item>(path: &'static str, source: &'static str) -> Reading
where
    Item: ReadItem,
{
    languages::contract::reading::<Item>(
        &crate::linked(),
        languages::SourcePath(path),
        languages::SampleText(source),
    )
    .expect("the source is read")
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
