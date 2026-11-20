//! The `fillers` service: the fact source a walk lends the engine over its language's fillers, and the run of one `fills!` item,
//! whole or by key.

use super::types::{FileSource, Fill, FillItem, Fillers, KeyedFillItem};
use crate::domain::facts::types::{FactSource, FileWords, FillExtent, Wanted};
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

/// Runs the `fills!` item `I` over the file: the body of a language's `Fillers` function for `I`'s relation, when the item has no
/// key clause. It writes every row of the relation whatever is wanted, and says so: asked for a key it answers
/// `Written(FillExtent::Whole)`.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::facts::types::{FileWords, Wanted};
/// use ryo_language::domain::fillers::service::filled;
/// use ryo_language::domain::fillers::types::{Fill, FillItem, Fillers};
/// use ryo_relations::domain::relations::collections::relations::interface::RelationsItem;
/// use ryo_relations::domain::relations::types::Rows;
///
/// fn through<I: FillItem>(
///     file: &mut <I::Walk as Fillers>::File<'_>,
///     words: &mut FileWords,
///     wanted: Wanted,
///     rows: &mut dyn Rows<<I::Relation as RelationsItem>::Row>,
/// ) -> Fill {
///     filled::<I>(file, words, wanted, rows)
/// }
/// ```
pub fn filled<I: FillItem>(
    file: &mut <I::Walk as Fillers>::File<'_>,
    words: &mut FileWords,
    wanted: Wanted,
    rows: &mut dyn Rows<<I::Relation as RelationsItem>::Row>,
) -> Fill {
    I::fill(file, words, rows);
    Fill::Written(match wanted {
        Wanted::Whole => FillExtent::Asked,
        Wanted::Key(_, _) => FillExtent::Whole,
    })
}

/// Runs the `fills!` item `I`, which has a key clause, over the file: the body of a language's `Fillers` function for `I`'s
/// relation. Asked for a key on a column of `I::KEYS` it writes that key's rows alone (`fill_key`) and answers
/// `Written(FillExtent::Asked)`; asked for a key on any other column it writes every row and answers
/// `Written(FillExtent::Whole)`.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::facts::types::{FileWords, Wanted};
/// use ryo_language::domain::fillers::service::filled_keyed;
/// use ryo_language::domain::fillers::types::{Fill, Fillers, KeyedFillItem};
/// use ryo_relations::domain::relations::collections::relations::interface::RelationsItem;
/// use ryo_relations::domain::relations::types::Rows;
///
/// fn through<I: KeyedFillItem>(
///     file: &mut <I::Walk as Fillers>::File<'_>,
///     words: &mut FileWords,
///     wanted: Wanted,
///     rows: &mut dyn Rows<<I::Relation as RelationsItem>::Row>,
/// ) -> Fill {
///     filled_keyed::<I>(file, words, wanted, rows)
/// }
/// ```
pub fn filled_keyed<I: KeyedFillItem>(
    file: &mut <I::Walk as Fillers>::File<'_>,
    words: &mut FileWords,
    wanted: Wanted,
    rows: &mut dyn Rows<<I::Relation as RelationsItem>::Row>,
) -> Fill {
    match wanted {
        Wanted::Key(column, key) if I::KEYS.contains(&column) => {
            I::fill_key(file, words, column, key, rows);
            Fill::Written(FillExtent::Asked)
        }
        Wanted::Whole | Wanted::Key(_, _) => filled::<I>(file, words, wanted, rows),
    }
}
