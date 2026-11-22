//! The `fillers` service: the fact source a walk lends the engine over its language's fillers, and the run of one `fills!`
//! item.

use super::types::{FileSource, Fill, FillItem, Fillers};
use crate::domain::facts::types::{FactSource, FileWords};
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

/// Runs the `fills!` item `I` over the file: the body of a language's `Fillers` function for `I`'s relation. It writes every
/// row of the relation and answers `Written`.
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

/// SPIKE (2026-11-21, never a contract): what the fills of one relation came to in a run counted under `RYO_X_FILLS`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FillCounts {
    /// The files whose one request asked the relation.
    pub asked: u64,
    /// The rows written.
    pub rows: u64,
    /// The time inside the fills, in nanoseconds (the first fill of a file that needs the language's model pays its build).
    pub nanos: u64,
    /// The fills of a file in which nothing but a need's row term read the relation after.
    pub unread: u64,
    /// The rows those fills wrote.
    pub unread_rows: u64,
    /// The time inside those fills, in nanoseconds.
    pub unread_nanos: u64,
}

/// SPIKE: one fill of the file a thread is scanning.
#[derive(Clone, Copy, Debug)]
struct FileFill {
    /// The relation.
    relation: Relation,
    /// The rows it wrote.
    rows: u64,
    /// Its time, in nanoseconds.
    nanos: u64,
}

thread_local! {
    /// SPIKE: the fills of the file this thread is scanning.
    static FILE_FILLS: std::cell::RefCell<Vec<FileFill>> = const { std::cell::RefCell::new(Vec::new()) };
    /// SPIKE: the relations a reader that is no need's row term reached in the file this thread is scanning.
    static FILE_READS: std::cell::RefCell<Vec<Relation>> = const { std::cell::RefCell::new(Vec::new()) };
    /// SPIKE: whether this thread is answering a need's row term now.
    static NEED_ASKING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// SPIKE: this thread starts (`true`) or ends answering a need's row term: a reach of a relation meanwhile is no read.
///
/// # Examples
///
/// ```
/// ryo_language::domain::fillers::service::fill_need_asking(false);
/// ```
pub fn fill_need_asking(asking: bool) {
    NEED_ASKING.with(|held| held.set(asking));
}

/// SPIKE: a reader reached a relation in the file this thread is scanning; counted when the run counts its fills and the
/// reader is no need's row term.
///
/// # Examples
///
/// ```
/// use ryo_relations::domain::relations::types::Relation;
///
/// ryo_language::domain::fillers::service::fill_read(Relation::Comment);
/// ```
pub fn fill_read(relation: Relation) {
    if fill_begun().is_some() && !NEED_ASKING.with(std::cell::Cell::get) {
        FILE_READS.with(|reads| {
            let mut reads = reads.borrow_mut();
            if !reads.contains(&relation) {
                reads.push(relation);
            }
        });
    }
}

/// SPIKE: the file this thread was scanning is done: each of its fills whose relation no reader reached is counted
/// unread.
///
/// # Examples
///
/// ```
/// ryo_language::domain::fillers::service::file_fills_settled();
/// ```
pub fn file_fills_settled() {
    if fill_begun().is_none() {
        return;
    }
    let fills = FILE_FILLS.with(|fills| std::mem::take(&mut *fills.borrow_mut()));
    let reads = FILE_READS.with(|reads| std::mem::take(&mut *reads.borrow_mut()));
    if let Ok(mut counts) = FILL_COUNTS.lock() {
        for fill in fills.iter().filter(|fill| !reads.contains(&fill.relation)) {
            let held = counts.entry(fill.relation).or_default();
            held.unread = held.unread.saturating_add(1);
            held.unread_rows = held.unread_rows.saturating_add(fill.rows);
            held.unread_nanos = held.unread_nanos.saturating_add(fill.nanos);
        }
    }
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

/// SPIKE: counts one relation of a file's one request: the rows its filler wrote and its time.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::fillers::service::fill_counted;
/// use ryo_relations::domain::relations::types::Relation;
///
/// fill_counted(Relation::Comment, 0, std::time::Instant::now());
/// ```
pub fn fill_counted(relation: Relation, rows: usize, begun: std::time::Instant) {
    let nanos = u64::try_from(begun.elapsed().as_nanos()).unwrap_or(u64::MAX);
    FILE_FILLS.with(|fills| {
        fills.borrow_mut().push(FileFill {
            relation,
            rows: u64::try_from(rows).unwrap_or(u64::MAX),
            nanos,
        });
    });
    if let Ok(mut counts) = FILL_COUNTS.lock() {
        let held = counts.entry(relation).or_default();
        held.asked = held.asked.saturating_add(1);
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
