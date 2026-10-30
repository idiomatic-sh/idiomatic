//! Reading a source with the language's own parse, for the tests of the items of a relation: the rows the language writes of the
//! item's relation, each in full, and the causes for which it says the relation is read only in part.

use ryo::languages::{self, ReadItem};

use ryo::languages::contract::Reading;

/// The path every source is read as, which a file cell of a row shows: a file of json as the standard defines it.
pub(crate) const SHOWN: &str = "item.json";

/// The path a source is read as when it is json with comments.
pub(crate) const SHOWN_COMMENTED: &str = "item.jsonc";

/// What the item writes for `source`, read with the language's own parse as json as the standard defines it.
///
/// # Panics
///
/// When the source cannot be read.
#[must_use]
pub(crate) fn reading<Item>(source: &'static str) -> Reading
where
    Item: ReadItem,
{
    read_as::<Item>(SHOWN, source)
}

/// What the item writes for `source`, read with the language's own parse as json with comments.
///
/// # Panics
///
/// When the source cannot be read.
#[must_use]
pub(crate) fn reading_commented<Item>(source: &'static str) -> Reading
where
    Item: ReadItem,
{
    read_as::<Item>(SHOWN_COMMENTED, source)
}

/// What the item writes for `source` read as the file at `path`.
fn read_as<Item>(path: &'static str, source: &'static str) -> Reading
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
