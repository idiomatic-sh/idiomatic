//! The `fillers` service: the fact source a walk lends the engine over its language's fillers, and the run of one `fills!` item.

use super::types::{FileSource, Fill, FillItem, Fillers};
use crate::domain::facts::types::{FactSource, FileWords};
use ryo_relations::domain::relations::collections::relations::interface::RelationsItem;
use ryo_relations::domain::relations::types::Rows;

/// Lends the engine a language's file through its fillers: what a walk hands its visit as `FileFacts::facts`.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::fillers::service::file_source;
/// use ryo_language::domain::fillers::types::Fillers;
///
/// fn lent<W: Fillers>(file: &mut W::File<'_>) {
///     let _source = file_source::<W>(file);
/// }
/// ```
pub fn file_source<'a, 'f, W: Fillers>(file: &'a mut W::File<'f>) -> impl FactSource + 'a
where
    'f: 'a,
{
    FileSource::<W> { file }
}

/// Runs the `fills!` item `I` over the file: the body of a language's `Fillers` function for `I`'s relation.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::facts::types::FileWords;
/// use ryo_language::domain::fillers::service::filled;
/// use ryo_language::domain::fillers::types::{Fill, FillItem, Fillers};
/// use ryo_relations::domain::relations::collections::relations::interface::RelationsItem;
/// use ryo_relations::domain::relations::types::Rows;
///
/// fn through<I: FillItem>(
///     file: &mut <I::Walk as Fillers>::File<'_>,
///     words: &mut FileWords,
///     rows: &mut dyn Rows<<I::Relation as RelationsItem>::Row>,
/// ) -> Fill {
///     filled::<I>(file, words, rows)
/// }
/// ```
pub fn filled<I: FillItem>(
    file: &mut <I::Walk as Fillers>::File<'_>,
    words: &mut FileWords,
    rows: &mut dyn Rows<<I::Relation as RelationsItem>::Row>,
) -> Fill {
    I::fill(file, words, rows);
    Fill::Written
}
