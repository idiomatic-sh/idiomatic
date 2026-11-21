//! The `fillers` service: the fact source a walk lends the engine over its language's fillers, and the run of one `fills!` item,
//! whole or by key.

use super::types::{FileSource, Fill, FillItem, Fillers, KeyedFillItem};
use crate::domain::facts::types::{FactSource, FileWords, FillExtent, Wanted};
use ryo_relations::domain::relations::collections::relations::interface::RelationsItem;
use ryo_relations::domain::relations::types::{Relation, Rows};

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

/// SPIKE (2026-11-21, never a contract): what the fills of one relation came to in a run counted under `RYO_X_FILLS`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FillCounts {
    /// The fills asked whole.
    pub whole: u64,
    /// The fills asked for one key.
    pub keyed: u64,
    /// The rows written.
    pub rows: u64,
    /// The time inside the fills, in nanoseconds (the first fill of a file that needs the language's model pays its build).
    pub nanos: u64,
}

/// SPIKE: every relation's counts, for the run.
static FILL_COUNTS: std::sync::Mutex<std::collections::BTreeMap<Relation, FillCounts>> =
    std::sync::Mutex::new(std::collections::BTreeMap::new());

/// SPIKE: the time now when the run counts its fills (`RYO_X_FILLS` is set); none otherwise.
///
/// # Examples
///
/// ```
/// let _begun = ryo_language::domain::fillers::service::fill_begun();
/// ```
#[must_use]
pub fn fill_begun() -> Option<std::time::Instant> {
    static COUNTING: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    COUNTING
        .get_or_init(|| std::env::var_os("RYO_X_FILLS").is_some())
        .then(std::time::Instant::now)
}

/// SPIKE: counts one fill of a relation: whether it was asked whole or for a key, the rows it wrote and its time.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::facts::types::Wanted;
/// use ryo_language::domain::fillers::service::fill_counted;
/// use ryo_relations::domain::relations::types::Relation;
///
/// fill_counted(Relation::Comment, Wanted::Whole, 0, std::time::Instant::now());
/// ```
pub fn fill_counted(relation: Relation, wanted: Wanted, rows: usize, begun: std::time::Instant) {
    let nanos = u64::try_from(begun.elapsed().as_nanos()).unwrap_or(u64::MAX);
    if let Ok(mut counts) = FILL_COUNTS.lock() {
        let held = counts.entry(relation).or_default();
        match wanted {
            Wanted::Whole => held.whole = held.whole.saturating_add(1),
            Wanted::Key(_, _) => held.keyed = held.keyed.saturating_add(1),
        }
        held.rows = held
            .rows
            .saturating_add(u64::try_from(rows).unwrap_or(u64::MAX));
        held.nanos = held.nanos.saturating_add(nanos);
    }
}

/// SPIKE: the run's counts so far, taken (the next call answers what was counted since).
///
/// # Examples
///
/// ```
/// let _counts = ryo_language::domain::fillers::service::fill_counts_taken();
/// ```
#[must_use]
pub fn fill_counts_taken() -> Vec<(Relation, FillCounts)> {
    match FILL_COUNTS.lock() {
        Ok(mut counts) => std::mem::take(&mut *counts).into_iter().collect(),
        Err(_) => Vec::new(),
    }
}
