//! The services of the `keying` part: the indexes a relation's table is made with, each row's key in each of them, the rows
//! put in the first key's order, and each index's presence bits, ranks and stretches.

use crate::domain::fact_tables::types::{
    BitAt, FactKey, FactRow, FactTable, IndexOrder, KeyBits, KeyCode, KeyColumnIndex, KeyRank,
    KeySpace, KeyedRow, SortRoom, TableBody, TableLayout,
};
use crate::domain::facts::types::CellWord;
use ryo_relations::domain::catalog::service::{fact_form, key_columns, scope_of};
use ryo_relations::domain::catalog::types::RowScope;
use ryo_relations::domain::relations::service::{ALL, kind, node_position};
use ryo_relations::domain::relations::types::{
    CellKind, Column, Count, FactForm, FilledKey, NodeId, Relation,
};

/// A relation's place in the catalog's list.
#[inline]
pub(crate) fn place_of(relation: Relation) -> Count {
    match ALL.binary_search(&relation) {
        Ok(place) => Count(place),
        Err(_) => Count(
            ALL.iter()
                .position(|each| *each == relation)
                .map_or(ALL.len(), |place| place),
        ),
    }
}

/// The table no relation answered: no row, no column.
pub(crate) static EMPTY: FactTable = FactTable {
    body: TableBody {
        named: Vec::new(),
        indexes: Vec::new(),
        columns: Vec::new(),
        rows: Count(0),
        sorting: SortRoom {
            pairs: Vec::new(),
            cells: Vec::new(),
            keys: Vec::new(),
        },
    },
};

/// A row number as a table numbers its rows.
#[inline]
pub(crate) fn row_number(at: Count) -> FactRow {
    FactRow(u32::try_from(at.0).unwrap_or(u32::MAX))
}

/// The index of one key column of `relation` at `place` among its columns `named`.
fn index_of(relation: Relation, named: &[Column], column: Column) -> Option<KeyColumnIndex> {
    let place = named.iter().position(|each| *each == column)?;
    let space = match kind(column) {
        CellKind::Node => KeySpace::Nodes,
        CellKind::Symbol | CellKind::File | CellKind::Span | CellKind::Number | CellKind::Flag => {
            match scope_of(relation) {
                RowScope::Project => KeySpace::Entities,
                RowScope::File => KeySpace::Words,
            }
        }
    };
    Some(KeyColumnIndex {
        place: Count(place),
        column,
        space,
        order: IndexOrder::Other,
        layout: TableLayout::PerKey,
        present: Vec::new(),
        rank: Vec::new(),
        offsets: Vec::new(),
        listed: Vec::new(),
        keys: Vec::new(),
    })
}

/// The indexes of `relation`'s table: the column its catalog item says it is filled by (`Unique` or `Many`), whose order the
/// rows are put in; then each other column its item declares among its keys. None for a relation filled whole.
pub(crate) fn indexes_of(relation: Relation, named: &[Column]) -> Vec<KeyColumnIndex> {
    let first = match fact_form(relation) {
        FactForm::Filled(FilledKey::Unique(column) | FilledKey::Many(column)) => Some(column),
        FactForm::Filled(FilledKey::Whole) | FactForm::NodeTable => None,
    };
    let mut indexes: Vec<KeyColumnIndex> = first
        .into_iter()
        .chain(
            key_columns(relation)
                .iter()
                .copied()
                .filter(|column| Some(*column) != first),
        )
        .filter_map(|column| index_of(relation, named, column))
        .collect();
    if let Some(index) = indexes.first_mut().filter(|_| first.is_some()) {
        index.order = IndexOrder::Keyed;
    }
    indexes
}

/// Empties an index, its buffers kept.
pub(crate) fn emptied(index: &mut KeyColumnIndex) {
    index.present.clear();
    index.rank.clear();
    index.offsets.clear();
    index.listed.clear();
    index.keys.clear();
    index.layout = TableLayout::PerKey;
}

/// The code a cell is as a key of `space`.
#[inline]
fn cell_code(space: KeySpace, cell: CellWord) -> KeyCode {
    match space {
        KeySpace::Nodes => KeyCode(node_position(NodeId(cell.0)).0),
        KeySpace::Words | KeySpace::Entities => KeyCode(u32::try_from(cell.0).unwrap_or(u32::MAX)),
    }
}

/// The code a key is in an index of `space`: none for a key of another space.
#[inline]
pub(crate) fn key_code(space: KeySpace, key: FactKey) -> Option<KeyCode> {
    match (space, key) {
        (KeySpace::Nodes, FactKey::Node(place)) => Some(KeyCode(place.0)),
        (KeySpace::Words, FactKey::Word(word)) => {
            u32::try_from(word.0).map_or(None, |code| Some(KeyCode(code)))
        }
        (KeySpace::Entities, FactKey::Entity(id)) => Some(KeyCode(id.0)),
        (KeySpace::Nodes | KeySpace::Words | KeySpace::Entities, _) => None,
    }
}

/// Where a key's bit is.
#[inline]
fn bit_of(code: KeyCode) -> BitAt {
    BitAt {
        word: Count(usize::try_from(code.0 / 64).unwrap_or(usize::MAX)),
        bit: KeyBits(1_u64 << (code.0 % 64)),
    }
}

/// Keeps one written row's key in every index: its bit set, and its code for the order and the stretches.
pub(crate) fn key_written(indexes: &mut [KeyColumnIndex], cells: &[CellWord]) {
    for index in indexes {
        debug_assert!(index.place.0 < cells.len(), "a row's cells hold its key");
        let Some(cell) = cells.get(index.place.0) else {
            continue;
        };
        let code = cell_code(index.space, *cell);
        let at = bit_of(code);
        if index.present.len() <= at.word.0 {
            index
                .present
                .resize(at.word.0.saturating_add(1), KeyBits::default());
        }
        if let Some(bits) = index.present.get_mut(at.word.0) {
            bits.0 |= at.bit.0;
        }
        index.keys.push(code);
    }
}

/// The rank of a present key among the present keys of a finished index; none when the key has no row.
#[inline]
pub(crate) fn present_at(index: &KeyColumnIndex, code: KeyCode) -> Option<KeyRank> {
    let at = bit_of(code);
    let bits = index.present.get(at.word.0)?;
    if bits.0 & at.bit.0 == 0 {
        return None;
    }
    let before = index.rank.get(at.word.0)?;
    Some(KeyRank(
        before
            .0
            .saturating_add((bits.0 & (at.bit.0 - 1)).count_ones()),
    ))
}

/// The (key, row) words of one index's keys, sorted: the rows in that key's order, each key's in written order.
fn sorted_pairs(pairs: &mut Vec<KeyedRow>, keys: &[KeyCode]) {
    pairs.clear();
    pairs.extend(
        keys.iter().enumerate().map(|(row, key)| {
            KeyedRow(u64::from(key.0) << 32 | u64::from(row_number(Count(row)).0))
        }),
    );
    pairs.sort_unstable();
}

/// The row a sorted pair names.
#[inline]
fn pair_row(pair: KeyedRow) -> Count {
    Count(usize::try_from(pair.0 & 0xffff_ffff).unwrap_or(usize::MAX))
}

/// Puts the rows in the first index's key order when they are not, moving every column and every other index's keys once.
fn in_first_order(body: &mut TableBody) {
    let Some(first) = body
        .indexes
        .first()
        .filter(|index| index.order == IndexOrder::Keyed)
    else {
        return;
    };
    if first.keys.is_sorted() {
        return;
    }
    let room = &mut body.sorting;
    sorted_pairs(&mut room.pairs, &first.keys);
    for column in &mut body.columns {
        room.cells.clear();
        room.cells.extend(
            room.pairs
                .iter()
                .filter_map(|pair| column.get(pair_row(*pair).0).copied()),
        );
        std::mem::swap(column, &mut room.cells);
    }
    for index in &mut body.indexes {
        room.keys.clear();
        room.keys.extend(
            room.pairs
                .iter()
                .filter_map(|pair| index.keys.get(pair_row(*pair).0).copied()),
        );
        std::mem::swap(&mut index.keys, &mut room.keys);
    }
}

/// Builds one index over keys in its own order: its ranks, and its stretches when a key has more than one row.
fn stretched(index: &mut KeyColumnIndex, keys: &[KeyCode]) {
    index.rank.clear();
    let mut before = KeyRank(0);
    for bits in &index.present {
        index.rank.push(before);
        before = KeyRank(before.0.saturating_add(bits.0.count_ones()));
    }
    index.offsets.clear();
    let listed = keys.windows(2).any(|pair| pair.first() == pair.get(1));
    index.layout = if listed {
        TableLayout::Listed
    } else {
        TableLayout::PerKey
    };
    if listed {
        let mut last = None;
        for (at, key) in keys.iter().enumerate() {
            if last != Some(*key) {
                index.offsets.push(row_number(Count(at)));
                last = Some(*key);
            }
        }
        index.offsets.push(row_number(Count(keys.len())));
    }
}

/// Finishes a table's indexes: the rows put in the first index's order, then each index built, an index in another order
/// with its rows listed in its own.
pub(crate) fn keyed(body: &mut TableBody) {
    in_first_order(body);
    let room = &mut body.sorting;
    for index in &mut body.indexes {
        let keys = std::mem::take(&mut index.keys);
        match index.order {
            IndexOrder::Keyed => stretched(index, &keys),
            IndexOrder::Other => {
                sorted_pairs(&mut room.pairs, &keys);
                index.listed.clear();
                index
                    .listed
                    .extend(room.pairs.iter().map(|pair| row_number(pair_row(*pair))));
                room.keys.clear();
                room.keys.extend(
                    room.pairs
                        .iter()
                        .map(|pair| KeyCode(u32::try_from(pair.0 >> 32).unwrap_or(u32::MAX))),
                );
                let sorted = std::mem::take(&mut room.keys);
                stretched(index, &sorted);
                room.keys = sorted;
            }
        }
        index.keys = keys;
        index.keys.clear();
    }
}
