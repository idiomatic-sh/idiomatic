//! The `hash_index` item of the `kernels` collection: rows keyed into the worker's one keyed-row table, with no allocation per
//! row and no key copied out, by one name column's text or by the cells of key columns.

#[cfg(test)]
use super::super::interface::{KernelsItem, Reach};
use super::super::scan_facts::item::{cell_of, count_of, rows_of};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::contract::types::PredicateOutcome;
use crate::domain::physical_plan::types::keyed_rows::value::{
    IndexKeyHash, IndexedRow, KeyWords, KeyedRows, KeyedRun, NO_ROW, NO_SLOT, hash_of_cells,
    key_words, same_cells, word_hash,
};
#[cfg(test)]
use crate::domain::physical_plan::types::lowering::value::Lowering;
use crate::domain::physical_plan::types::walk_scratch::value::WalkScratch;
use crate::domain::physical_plan::types::{KeyedRowAt, KeyedSide};
use crate::domain::table_scratch::types::SetMembership;
use crate::domain::table_texts::types::Texts;
use ryo_language::domain::symbols::service::hash_in;
use ryo_relations::domain::relations::service::from_bits;
use ryo_relations::domain::relations::types::{Bits, CellKind, ColumnsView, Value};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Direction};
use ryo_values::domain::rows::types::{RowCount, RowIndex};

/// `hash_index`, one item of the `kernels` collection.
#[cfg(test)]
pub(crate) struct HashIndexItem;

#[cfg(test)]
impl KernelsItem for HashIndexItem {
    fn reached_by(&self, lowering: &Lowering) -> Reach {
        let runs = {
            matches!(
                lowering,
                Lowering::Lookup { .. }
                    | Lowering::Probe { .. }
                    | Lowering::Group { .. }
                    | Lowering::Reach { .. }
                    | Lowering::SeededReach { .. }
                    | Lowering::Axis { .. }
                    | Lowering::OrderedWalk { .. }
                    | Lowering::Worklist { .. }
                    | Lowering::Components { .. }
            )
        };
        if runs { Reach::Runs } else { Reach::Skips }
    }
}

/// Empties `rows` for the rows of `count`, keeping its capacity.
fn cleared(rows: &mut KeyedRows, count: RowCount) {
    rows.cleared();
    rows.next.resize(count.0, NO_ROW);
}

/// Spike (ryo-jenl): a hashed build holds no position-addressed room (one representation's room per index, so a pooled
/// index never keeps both).
fn hashed_only(rows: &mut KeyedRows) {
    if rows.slots.capacity() > 0 {
        rows.slots = Vec::new();
    }
    if rows.runs.capacity() > 0 {
        rows.runs = Vec::new();
    }
}

/// Adds `row` to the key `same` finds under `hash`, or as a key of its own.
fn insert(
    rows: &mut KeyedRows,
    hash: IndexKeyHash,
    row: IndexedRow,
    same: impl Fn(&KeyedRun) -> PredicateOutcome,
) {
    match rows
        .keys
        .find_mut(hash.0, |run| same(run) == PredicateOutcome::Holds)
    {
        Some(run) => {
            if let Some(link) = rows
                .next
                .get_mut(usize::try_from(run.last.0).unwrap_or(usize::MAX))
            {
                *link = row;
            }
            run.last = row;
            run.rows = RowCount(run.rows.0.saturating_add(1));
        }
        None => {
            let _: hashbrown::hash_table::OccupiedEntry<'_, KeyedRun> = rows.keys.insert_unique(
                hash.0,
                KeyedRun {
                    hash,
                    first: row,
                    last: row,
                    rows: RowCount(1),
                },
                |run| run.hash.0,
            );
        }
    }
}

/// Whether a key's hash is `hash`.
fn hashed(run: &KeyedRun, hash: IndexKeyHash) -> PredicateOutcome {
    if run.hash == hash {
        PredicateOutcome::Holds
    } else {
        PredicateOutcome::Fails
    }
}

/// Spike (ryo-jenl): counts a build of `side` in the key census.
fn census(side: KeyedSide<'_>, rows: &mut KeyedRows) {
    if !crate::spike::keystats() {
        return;
    }
    let kinds = super::super::scan_facts::item::kinds_of(side.rows);
    let class = crate::spike::key_class(side.on.iter().map(|column| {
        kinds.get(usize::try_from(column.0).unwrap_or(usize::MAX)).copied().unwrap_or(CellKind::Flag)
    }));
    rows.census = u8::try_from(class).unwrap_or(4);
    let count = count_of(side.rows);
    crate::spike::key_rows(0, class, count.0 as u64);
    if let [one, two] = side.on {
        let kind = |column: &ColumnIndex| kinds.get(usize::try_from(column.0).unwrap_or(usize::MAX)).copied().unwrap_or(CellKind::Flag);
        crate::spike::key_pair([kind(one), kind(two)], count.0 as u64);
    }
    if class == 0 && count.0 > 0 {
        let (mut low, mut high, mut file, mut files) = (u64::MAX, 0u64, None, false);
        for at in 0..count.0 {
            if let Some(Value::Node(node)) = cell_of(side.rows, RowIndex(at), side.on[0]) {
                let place = node.0 & 0xffff_ffff;
                low = low.min(place);
                high = high.max(place);
                let here = node.0 >> 32;
                if file.is_some_and(|seen| seen != here) {
                    files = true;
                }
                file = Some(here);
            }
        }
        crate::spike::key_range(high.saturating_sub(low) + 1, count.0 as u64, files);
    }
}

/// Keys the relation's rows by the hash of `column`'s text into `rows`: each key's first and last row, and each row's next
/// of the same key, ascending. A cell with no text is no key.
pub(crate) fn texts(
    view: &ColumnsView<'_>,
    column: ColumnIndex,
    texts: &Texts<'_>,
    rows: &mut KeyedRows,
) {
    cleared(rows, rows_of(view));
    hashed_only(rows);
    if crate::spike::keystats() {
        crate::spike::key_rows(5, 2, rows_of(view).0 as u64);
    }
    let Some(cells) = view
        .columns
        .get(usize::try_from(column.0).unwrap_or(usize::MAX))
    else {
        return;
    };
    for (bits, row) in cells.0.iter().zip((0..).map(IndexedRow)) {
        let Value::Symbol(symbol) = from_bits(CellKind::Symbol, Bits(*bits)) else {
            continue;
        };
        let Some(hash) = hash_in(texts.symbols, symbol) else {
            continue;
        };
        let hash = IndexKeyHash(hash.0);
        insert(rows, hash, row, |run| hashed(run, hash));
    }
}

/// The hash of a row's cells in its key columns.
///
/// # Errors
///
/// `NoSuchColumn` for a key column past the row.
pub(crate) fn hash_of(at: KeyedRowAt<'_>) -> Result<IndexKeyHash, PortableOperatorFault> {
    let mut missing = None;
    let hash = hash_of_cells(at.side.on.iter().map_while(|column| {
        let cell = cell_of(at.side.rows, at.row, *column);
        if cell.is_none() {
            missing = Some(*column);
        }
        cell
    }));
    match missing {
        Some(index) => Err(PortableOperatorFault::NoSuchColumn { index }),
        None => Ok(hash),
    }
}

/// Whether two rows hold the same cells in their key columns, read in place.
fn same_key(left: KeyedRowAt<'_>, right: KeyedRowAt<'_>) -> PredicateOutcome {
    same_cells(
        left.side
            .on
            .iter()
            .map(|column| cell_of(left.side.rows, left.row, *column)),
        right
            .side
            .on
            .iter()
            .map(|column| cell_of(right.side.rows, right.row, *column)),
    )
}

/// The row an index row is.
pub(crate) fn indexed(row: IndexedRow) -> RowIndex {
    RowIndex(usize::try_from(row.0).unwrap_or(usize::MAX))
}

/// Keys every row of `side` by its cells in its key columns into `rows`: rows of equal cells share a key, compared from the
/// rows' own cells, each key's rows ascending.
///
/// # Errors
///
/// `NoSuchColumn` for a key column past a row.
pub(crate) fn cells(
    side: KeyedSide<'_>,
    rows: &mut KeyedRows,
) -> Result<(), PortableOperatorFault> {
    // Spike (ryo-jenl): a key that is one word is keyed by its word, by position when its words span few places.
    if crate::spike::dense_levers()[4] {
        census(side, rows);
        if words_any(side, rows)? {
            return Ok(());
        }
    }
    cells_by_cells(side, rows)
}

/// `cells`, every key hashed and compared cell by cell.
///
/// # Errors
///
/// `NoSuchColumn` for a key column past a row.
fn cells_by_cells(
    side: KeyedSide<'_>,
    rows: &mut KeyedRows,
) -> Result<(), PortableOperatorFault> {
    census(side, rows);
    let count = count_of(side.rows);
    cleared(rows, count);
    hashed_only(rows);
    for at in 0..count.0 {
        let probe = KeyedRowAt {
            side,
            row: RowIndex(at),
        };
        let hash = hash_of(probe)?;
        let row = IndexedRow(u32::try_from(at).unwrap_or(u32::MAX));
        insert(rows, hash, row, |run| {
            if run.hash == hash {
                same_key(
                    KeyedRowAt {
                        side,
                        row: indexed(run.first),
                    },
                    probe,
                )
            } else {
                PredicateOutcome::Fails
            }
        });
    }
    Ok(())
}

/// The key of `rows` whose text hashes to `hash`.
pub(crate) fn find_text(rows: &KeyedRows, hash: IndexKeyHash) -> Option<KeyedRun> {
    rows.keys
        .find(hash.0, |run| hashed(run, hash) == PredicateOutcome::Holds)
        .copied()
}

/// The key of `rows`, indexed from `indexed_side`, that `probe` holds in its key columns.
///
/// # Errors
///
/// `NoSuchColumn` for a probe key column past the row.
pub(crate) fn find_cells(
    rows: &KeyedRows,
    indexed_side: KeyedSide<'_>,
    probe: KeyedRowAt<'_>,
) -> Result<Option<KeyedRun>, PortableOperatorFault> {
    if crate::spike::keystats() {
        crate::spike::key_rows(1, usize::from(rows.census), 1);
    }
    if rows.words != KeyWords::Cells && probe.side.on.len() == indexed_side.on.len() {
        return word_found(rows, probe.side.on, probe.row, &|row, column| cell_of(probe.side.rows, row, column));
    }
    let hash = hash_of(probe)?;
    Ok(rows
        .keys
        .find(hash.0, |run| {
            run.hash == hash
                && same_key(
                    KeyedRowAt {
                        side: indexed_side,
                        row: indexed(run.first),
                    },
                    probe,
                ) == PredicateOutcome::Holds
        })
        .copied())
}

/// The key of `rows`, indexed from `indexed_side` on one column, whose cell is `key`.
///
/// # Errors
///
/// `ShapeMismatch` when the side is keyed on more or fewer columns than one.
pub(crate) fn find_value(
    rows: &KeyedRows,
    indexed_side: KeyedSide<'_>,
    key: Value,
) -> Result<Option<KeyedRun>, PortableOperatorFault> {
    let [column] = indexed_side.on else {
        return Err(PortableOperatorFault::ShapeMismatch);
    };
    if crate::spike::keystats() {
        crate::spike::key_rows(1, usize::from(rows.census), 1);
    }
    if rows.words != KeyWords::Cells {
        let mut words = rows.words;
        return Ok(words.word(key, None, false).and_then(|word| find_word(rows, word)));
    }
    let hash = hash_of_cells([key]);
    Ok(rows
        .keys
        .find(hash.0, |run| {
            run.hash == hash && cell_of(indexed_side.rows, indexed(run.first), *column) == Some(key)
        })
        .copied())
}

/// The row after `row` of the same key, or `NO_ROW` after its last.
#[expect(
    clippy::indexing_slicing,
    reason = "the chain is made together with the rows it indexes: a row past it is a bug, which panics"
)]
pub(crate) fn next_of(rows: &KeyedRows, row: IndexedRow) -> IndexedRow {
    rows.next[usize::try_from(row.0).unwrap_or(usize::MAX)]
}

/// Appends to `walk.reached` what the keyed edges lead to from `start`: the edges' far ends in edge order for a parent or a
/// child; every entity met breadth first, each once (the start not met before), for an ancestor or a descendant; the first
/// one `admitted` holds for the nearest ancestor.
pub(crate) fn keyed_walk(
    edges: KeyedSide<'_>,
    index: &KeyedRows,
    to: ColumnIndex,
    direction: Direction,
    start: Value,
    walk: &mut WalkScratch,
    admitted: impl Fn(Value) -> Result<SetMembership, PortableOperatorFault>,
) -> Result<(), PortableOperatorFault> {
    if matches!(direction, Direction::Parent | Direction::Child) {
        return follow(edges, index, to, start, &mut walk.reached);
    }
    let all = direction != Direction::NearestAncestor;
    walk.visited.reset();
    walk.entities.clear();
    follow(edges, index, to, start, &mut walk.entities)?;
    while !walk.entities.is_empty() {
        walk.next.clear();
        for place in 0..walk.entities.len() {
            let entity = walk.entities[place];
            if walk.visited.meet(entity) == SetMembership::In {
                continue;
            }
            if !all && admitted(entity)? == SetMembership::In {
                walk.reached.push(entity);
                return Ok(());
            }
            follow(edges, index, to, entity, &mut walk.next)?;
            if all {
                walk.reached.push(entity);
            }
        }
        std::mem::swap(&mut walk.entities, &mut walk.next);
    }
    Ok(())
}

/// Appends to `into` the far end of each edge keyed by `start`, in edge order.
fn follow(
    edges: KeyedSide<'_>,
    index: &KeyedRows,
    to: ColumnIndex,
    start: Value,
    into: &mut Vec<Value>,
) -> Result<(), PortableOperatorFault> {
    let mut row = find_value(index, edges, start)?.map_or(NO_ROW, |run| run.first);
    while row != NO_ROW {
        into.push(
            cell_of(edges.rows, indexed(row), to)
                .ok_or(PortableOperatorFault::NoSuchColumn { index: to })?,
        );
        row = next_of(index, row);
    }
    Ok(())
}

/// Spike (ryo-fmx3): the hash of a row's cells in key columns `on`, each read by `cell`: `hash_of`, with the reader
/// monomorphised.
///
/// # Errors
///
/// `NoSuchColumn` for the first key column past the row.
#[inline]
pub(crate) fn hash_read(
    on: &[ColumnIndex],
    row: RowIndex,
    cell: &impl Fn(RowIndex, ColumnIndex) -> Option<Value>,
) -> Result<IndexKeyHash, PortableOperatorFault> {
    let mut missing = None;
    let hash = hash_of_cells(on.iter().map_while(|column| {
        let read = cell(row, *column);
        if read.is_none() {
            missing = Some(*column);
        }
        read
    }));
    match missing {
        Some(index) => Err(PortableOperatorFault::NoSuchColumn { index }),
        None => Ok(hash),
    }
}

/// Spike (ryo-fmx3): whether two rows hold the same cells in their key columns: `same_key`, each side's reader
/// monomorphised.
#[inline]
fn same_read(
    one_on: &[ColumnIndex],
    one_row: RowIndex,
    one: &impl Fn(RowIndex, ColumnIndex) -> Option<Value>,
    other_on: &[ColumnIndex],
    other_row: RowIndex,
    other: &impl Fn(RowIndex, ColumnIndex) -> Option<Value>,
) -> bool {
    one_on.len() == other_on.len()
        && one_on.iter().zip(other_on).all(|(a, b)| {
            let cell = one(one_row, *a);
            cell.is_some() && cell == other(other_row, *b)
        })
}

/// Spike (ryo-fmx3): `cells`, each row's key read by `cell`, a reader of the side's own kind.
///
/// # Errors
///
/// `NoSuchColumn` for a key column past a row.
pub(crate) fn cells_read(
    count: RowCount,
    on: &[ColumnIndex],
    cell: &impl Fn(RowIndex, ColumnIndex) -> Option<Value>,
    rows: &mut KeyedRows,
) -> Result<(), PortableOperatorFault> {
    cleared(rows, count);
    hashed_only(rows);
    rows.keys.reserve(count.0.min(1 << 16), |run| run.hash.0);
    for at in 0..count.0 {
        let hash = hash_read(on, RowIndex(at), cell)?;
        let row = IndexedRow(u32::try_from(at).unwrap_or(u32::MAX));
        insert(rows, hash, row, |run| {
            if run.hash == hash && same_read(on, indexed(run.first), cell, on, RowIndex(at), cell) {
                PredicateOutcome::Holds
            } else {
                PredicateOutcome::Fails
            }
        });
    }
    Ok(())
}

/// Spike (ryo-fmx3): `find_cells`, the left row's key read by `left`, the index's rows by `right`.
///
/// # Errors
///
/// `NoSuchColumn` for a left key column past the row.
#[inline]
pub(crate) fn find_read(
    rows: &KeyedRows,
    right_on: &[ColumnIndex],
    right: &impl Fn(RowIndex, ColumnIndex) -> Option<Value>,
    left_on: &[ColumnIndex],
    at: RowIndex,
    left: &impl Fn(RowIndex, ColumnIndex) -> Option<Value>,
) -> Result<Option<KeyedRun>, PortableOperatorFault> {
    if crate::spike::keystats() {
        crate::spike::key_rows(1, usize::from(rows.census), 1);
    }
    if rows.words != KeyWords::Cells && left_on.len() == right_on.len() {
        return word_found(rows, left_on, at, left);
    }
    let hash = hash_read(left_on, at, left)?;
    Ok(rows
        .keys
        .find(hash.0, |run| {
            run.hash == hash && same_read(right_on, indexed(run.first), right, left_on, at, left)
        })
        .copied())
}

/// Spike (ryo-fmx3): `cells` over `side`, its rows read by their own kind's reader when the `fastkey` lever is on.
///
/// # Errors
///
/// `NoSuchColumn` for a key column past a row.
pub(crate) fn cells_typed(side: KeyedSide<'_>, rows: &mut KeyedRows) -> Result<(), PortableOperatorFault> {
    use super::super::scan_facts::item::{facts_cell, table_cell};
    if !crate::spike::probe_lever("fastkey") {
        return cells(side, rows);
    }
    if !matches!(side.rows, crate::domain::physical_plan::types::InputRows::Tree(_) | crate::domain::physical_plan::types::InputRows::Row(..)) {
        census(side, rows);
    }
    if words_any(side, rows)? {
        return Ok(());
    }
    match side.rows {
        crate::domain::physical_plan::types::InputRows::Table(table) => {
            cells_read(table.count, side.on, &|row, column| table_cell(table, row, column), rows)
        }
        crate::domain::physical_plan::types::InputRows::Facts(view) => {
            cells_read(rows_of(view), side.on, &|row, column| facts_cell(view, row, column), rows)
        }
        crate::domain::physical_plan::types::InputRows::Tree(_)
        | crate::domain::physical_plan::types::InputRows::Row(..) => cells_by_cells(side, rows),
    }
}

/// Spike (ryo-jenl): keys `side` by its one-word keys (`KeyWords`, from its key columns' declared kinds), each row read by
/// its own kind's reader; `false`, nothing keyed, for a key that is no word or a row whose cells are not.
///
/// # Errors
///
/// `NoSuchColumn` for a key column past a row.
fn words_any(side: KeyedSide<'_>, rows: &mut KeyedRows) -> Result<bool, PortableOperatorFault> {
    use super::super::scan_facts::item::{facts_cell, table_cell};
    let kinds = super::super::scan_facts::item::kinds_of(side.rows);
    let kind = |column: &ColumnIndex| kinds.get(usize::try_from(column.0).unwrap_or(usize::MAX)).copied();
    let words = if crate::spike::dense_lever("words") {
        match side.on {
            [one] => kind(one).map_or(KeyWords::Cells, |one| key_words([one].into_iter())),
            [one, two] => match (kind(one), kind(two)) {
                (Some(one), Some(two)) => key_words([one, two].into_iter()),
                _ => KeyWords::Cells,
            },
            _ => KeyWords::Cells,
        }
    } else {
        KeyWords::Cells
    };
    if words == KeyWords::Cells {
        return Ok(false);
    }
    match side.rows {
        crate::domain::physical_plan::types::InputRows::Table(table) => {
            words_read(table.count, side.on, &|row, column| table_cell(table, row, column), words, rows)
        }
        crate::domain::physical_plan::types::InputRows::Facts(view) => {
            words_read(rows_of(view), side.on, &|row, column| facts_cell(view, row, column), words, rows)
        }
        crate::domain::physical_plan::types::InputRows::Tree(_)
        | crate::domain::physical_plan::types::InputRows::Row(..) => {
            words_read(count_of(side.rows), side.on, &|row, column| cell_of(side.rows, row, column), words, rows)
        }
    }
}

/// Spike (ryo-jenl): `cells_read` for a key that is one word (`KeyWords`): each key hashed by its word's exact hash and
/// compared by that hash alone, no row read back. Answers `false`, its rows to be keyed cell by cell, when a row's cell is
/// not of its column's declared kind or a two-column key's node is in another file than the first key's.
///
/// # Errors
///
/// `NoSuchColumn` for a key column past a row.
fn words_read(
    count: RowCount,
    on: &[ColumnIndex],
    cell: &impl Fn(RowIndex, ColumnIndex) -> Option<Value>,
    words: KeyWords,
    rows: &mut KeyedRows,
) -> Result<bool, PortableOperatorFault> {
    cleared(rows, count);
    let mut words = words;
    let mut scratch = std::mem::take(&mut rows.scratch);
    scratch.clear();
    let (mut low, mut high) = (u64::MAX, 0u64);
    for at in 0..count.0 {
        let Some(word) = word_read(&mut words, on, RowIndex(at), cell, true)? else {
            rows.scratch = scratch;
            return Ok(false);
        };
        low = low.min(word);
        high = high.max(word);
        scratch.push(word);
    }
    let span = high.wrapping_sub(low);
    if count.0 > 0 && crate::spike::dense_levers()[3] && span < 4 * count.0 as u64 + 64 {
        // Position-addressed: each key's place in `runs` at its word less the smallest; the slots cleared are at most four
        // per row.
        if rows.keys.capacity() > 0 {
            rows.keys = hashbrown::HashTable::new();
        }
        rows.base = low;
        rows.slots.clear();
        rows.slots.resize(usize::try_from(span).unwrap_or(0) + 1, NO_SLOT);
        for (at, word) in scratch.iter().enumerate() {
            let row = IndexedRow(u32::try_from(at).unwrap_or(u32::MAX));
            let place = usize::try_from(word - low).unwrap_or(usize::MAX);
            let Some(slot) = rows.slots.get_mut(place) else {
                continue;
            };
            if *slot == NO_SLOT {
                *slot = u32::try_from(rows.runs.len()).unwrap_or(NO_SLOT);
                rows.runs.push(KeyedRun {
                    hash: IndexKeyHash(*word),
                    first: row,
                    last: row,
                    rows: RowCount(1),
                });
            } else if let Some(run) = rows.runs.get_mut(usize::try_from(*slot).unwrap_or(usize::MAX)) {
                if let Some(link) = rows.next.get_mut(usize::try_from(run.last.0).unwrap_or(usize::MAX)) {
                    *link = row;
                }
                run.last = row;
                run.rows = RowCount(run.rows.0.saturating_add(1));
            }
        }
    } else {
        hashed_only(rows);
        rows.keys.reserve(count.0.min(1 << 16), |run| run.hash.0);
        for (at, word) in scratch.iter().enumerate() {
            let hash = word_hash(*word);
            let row = IndexedRow(u32::try_from(at).unwrap_or(u32::MAX));
            insert(rows, hash, row, |run| hashed(run, hash));
        }
    }
    // The words are read once per build: a build past the kept room gives its scratch back rather than hold it.
    if scratch.capacity() <= crate::domain::kept_room::constants::KEPT_ROOM {
        rows.scratch = scratch;
    }
    rows.words = words;
    if crate::spike::keystats() {
        crate::spike::key_rows(6, usize::from(rows.census), count.0 as u64);
    }
    Ok(true)
}

/// Spike (ryo-jenl): the word of row `at`'s key in columns `on`; none when a cell is of another kind or file.
///
/// # Errors
///
/// `NoSuchColumn` for the first key column past the row.
#[inline]
fn word_read(
    words: &mut KeyWords,
    on: &[ColumnIndex],
    at: RowIndex,
    cell: &impl Fn(RowIndex, ColumnIndex) -> Option<Value>,
    fix: bool,
) -> Result<Option<u64>, PortableOperatorFault> {
    let (first, second) = match on {
        [first] => (*first, None),
        [first, second] => (*first, Some(*second)),
        _ => return Ok(None),
    };
    let one = cell(at, first).ok_or(PortableOperatorFault::NoSuchColumn { index: first })?;
    let two = match second {
        Some(column) => Some(cell(at, column).ok_or(PortableOperatorFault::NoSuchColumn { index: column })?),
        None => None,
    };
    Ok(words.word(one, two, fix))
}

/// Spike (ryo-jenl): the key of word-keyed `rows` that row `at`, read by `cell`, holds in columns `on`.
///
/// # Errors
///
/// `NoSuchColumn` for the first key column past the row.
#[inline]
fn word_found(
    rows: &KeyedRows,
    on: &[ColumnIndex],
    at: RowIndex,
    cell: &impl Fn(RowIndex, ColumnIndex) -> Option<Value>,
) -> Result<Option<KeyedRun>, PortableOperatorFault> {
    let mut words = rows.words;
    Ok(word_read(&mut words, on, at, cell, false)?.and_then(|word| find_word(rows, word)))
}

/// Spike (ryo-jenl): the key of word-keyed `rows` whose word is `word`.
#[inline]
fn find_word(rows: &KeyedRows, word: u64) -> Option<KeyedRun> {
    if crate::spike::keystats() {
        crate::spike::key_rows(7, usize::from(rows.census), 1);
    }
    rows.word_run(word)
}
