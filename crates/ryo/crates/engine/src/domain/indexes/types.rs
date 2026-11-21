//! The types of the `indexes` concept: a worker's per-file indexes, each the rows of one source grouped by every bound
//! column as one key. Crate-private.

use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::Count;

/// A file's number in a worker's life: a slot stamped with another file's number holds nothing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct FileStamp(pub(crate) u32);

/// A row of a source, by its number: what an index's stretches hold.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct SourceRow(pub(crate) u32);

/// One word of an index's table: a cell of a key, or an entry's run as `start << 32 | end`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct TableWord(pub(crate) u64);

/// A key of an index being built, by its number among the keys in the order first seen.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct KeyNumber(pub(crate) u32);

/// While an index is built, of one key: how many rows hold it, then where its next row goes among the index's rows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct StretchPlace(pub(crate) u32);

/// The hash of a key, or of a row.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct RowHash(pub(crate) u64);

/// How many bits of a hash do not name an entry of a table.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct HashShift(pub(crate) u32);

/// One column of a source as a loop reads it: the cell of row `r` is `cells[first + r * stride]`. A filled column has stride
/// 1; a made table's column has the table's width, since its rows are kept row by row.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ColumnView<'a> {
    /// The cells the column lies in.
    pub(crate) cells: &'a [CellWord],
    /// Where its first cell is.
    pub(crate) first: Count,
    /// How far apart its cells are.
    pub(crate) stride: Count,
}

/// The least and the greatest cell of a key's first column.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct KeyBounds {
    /// The least cell.
    pub(crate) least: CellWord,
    /// The greatest cell.
    pub(crate) greatest: CellWord,
}

/// A worker's indexes: one table of keys and one run of rows per index, each in an arena that is emptied per file and never
/// freed.
#[derive(Debug, Default)]
pub(crate) struct IndexArena {
    /// Every index's table, one after the other: per entry the key's cells, then its run.
    pub(crate) entries: Vec<TableWord>,
    /// Every index's rows, each index's grouped by key.
    pub(crate) rows: Vec<SourceRow>,
    /// Each index's place in the arenas, by `IndexSite`.
    pub(crate) slots: Vec<IndexSlot>,
    /// While an index is built: the key each row holds; or, by a column of small codes, the key each code names.
    pub(crate) places: Vec<KeyNumber>,
    /// While an index is built: each key's count, then its next place.
    pub(crate) groups: Vec<StretchPlace>,
    /// While an index is built by a column of small codes: how many rows hold each code.
    pub(crate) counts: Vec<StretchPlace>,
    /// While an index is built: how many rows its source has.
    pub(crate) pending: Count,
    /// While an index is built: each row's key, row by row.
    pub(crate) keys: Vec<TableWord>,
    /// While an index is built: how many cells a key has.
    pub(crate) width: Count,
    /// While an index is built: the bounds of its key's first column.
    pub(crate) bounds: KeyBounds,
    /// While an index is built: the number of its source's first row. A relation filled whole after keys were asked
    /// starts past their rows.
    pub(crate) base: SourceRow,
    /// How many rows at the front of `rows` each hold their own number: what an index of rows by their own id reads. They
    /// are kept from file to file, so such an index is built by writing nothing.
    pub(crate) own: Count,
    /// How many such rows the largest file so far asked for: the front is grown to it when the next file starts, never
    /// while a file's stretches are held.
    pub(crate) wanted: Count,
}

/// What a build by counting is told of its key: the least cell, how far the greatest lies past it, and how many rows
/// the source has.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CountedKey {
    /// The least cell a row may hold.
    pub(crate) least: CellWord,
    /// How far the greatest cell lies past the least.
    pub(crate) span: Count,
    /// How many rows the source has.
    pub(crate) rows: Count,
}

/// How the cells of a one-cell key lie.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum KeyCells {
    /// Close together: its rows are placed by counting.
    Close,
    /// Apart: its keys are entered in the open table.
    Apart,
}

/// Where one index lies, and the file it was built for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct IndexSlot {
    /// The file it was built for.
    pub(crate) stamp: FileStamp,
    /// Where its table starts among the entries.
    pub(crate) table: Count,
    /// How many words its table has.
    pub(crate) words: Count,
    /// How many entries a hashed table has less one: a power of two less one.
    pub(crate) mask: Count,
    /// How many bits of a hash do not name an entry.
    pub(crate) shift: HashShift,
    /// How a key finds its entry.
    pub(crate) addressing: Addressing,
    /// How many cells a key has.
    pub(crate) width: Count,
    /// Where its rows start among the rows.
    pub(crate) first: Count,
    /// How many rows it holds.
    pub(crate) count: Count,
}

/// One built index, lent for reading.
#[derive(Clone, Copy, Debug)]
pub(crate) struct IndexRead<'a> {
    /// Its table.
    pub(crate) entries: &'a [TableWord],
    /// Its rows, grouped by key.
    pub(crate) rows: &'a [SourceRow],
    /// How many entries the table has less one.
    pub(crate) mask: Count,
    /// How many bits of a hash do not name an entry.
    pub(crate) shift: HashShift,
    /// How a key finds its entry.
    pub(crate) addressing: Addressing,
    /// How many cells a key has.
    pub(crate) width: Count,
}

/// The rows of one key: a stretch of an index's rows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct RowRun {
    /// Where the stretch starts.
    pub(crate) start: Count,
    /// Where it ends.
    pub(crate) end: Count,
}

/// How a key finds its stretch of an index's rows. A reader is handed the stretch whichever way it was found: an index
/// has one form, a key's rows side by side, and three ways of being built, chosen by the shape of its key.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Addressing {
    /// By the key's hash, in an open table of the distinct keys, each with its stretch: any cells, any width.
    #[default]
    Hashed,
    /// By the key's one cell less the least cell the source holds: a column of ids or codes that lie close together. The
    /// table holds no key: word `c` is where the stretch of the cell `least + c` starts and word `c + 1` where it ends, so
    /// the rows are placed by counting and a key is found by a subtraction.
    Counted(CellWord),
    /// By the key's one cell less the least, which is the row itself: the rows of a source by their own id. The index
    /// has no table, and its rows are the arena's rows that hold their own number.
    Rows(CellWord),
}

/// Whether a site's index is built for the file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum IndexBuilt {
    /// It is built for the file.
    Built,
    /// It is not: the site holds nothing, or another file's index.
    Unbuilt,
}

/// One end of a range over an ordered index: the cell, and whether a row equal to it is in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RangeEnd {
    /// The cell.
    pub(crate) cell: CellWord,
    /// Whether a row equal to the cell is in the range.
    pub(crate) end: crate::domain::steps::types::BoundKind,
}
