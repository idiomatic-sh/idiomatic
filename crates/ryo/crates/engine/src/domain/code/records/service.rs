//! The services of the `records` part: the one place a pool record and a need record are written and read. A reader answers
//! none for a place the slots do not reach and for a tag no record has.

use super::types::{
    BodyRecord, BoundFormal, CollectedRead, EdgeColumns, NeedRecord, PooledOperand, ReadBody,
    VisitColumns, WorklistColumns,
};
use crate::domain::code::constants::NO_SLOT;
use crate::domain::code::types::{
    CodeEntry, ConstantNumber, IndexSite, MemoSite, NeedSite, PoolAt, RelationNumber, WordSlot,
};
use crate::domain::steps::types::{MadeColumn, MadeTableId, Register};

/// How many slots an operand takes.
pub(crate) const OPERAND_SLOTS: WordSlot = WordSlot(2);

/// The place the next slot written to `slots` takes; past the width of a place it saturates, which the encoding refuses when
/// it counts its slots.
#[must_use]
pub(crate) fn next_place(slots: &[WordSlot]) -> PoolAt {
    PoolAt(u32::try_from(slots.len()).unwrap_or(u32::MAX))
}

/// The slot `ahead` places after `at`, when the slots reach it.
fn slot_at(slots: &[WordSlot], at: WordSlot, ahead: WordSlot) -> Option<WordSlot> {
    let place = at.0.checked_add(ahead.0)?;
    slots
        .get(usize::try_from(place).unwrap_or(usize::MAX))
        .copied()
}

/// How many items a record holds, as the slot that says so.
fn count_of<T>(items: &[T]) -> WordSlot {
    WordSlot(u32::try_from(items.len()).unwrap_or(u32::MAX))
}

/// Writes a list: its length, then its items.
pub(crate) fn push_list(pool: &mut Vec<WordSlot>, items: &[WordSlot]) -> PoolAt {
    let at = next_place(pool);
    pool.push(count_of(items));
    pool.extend_from_slice(items);
    at
}

/// Writes an operand: its tag, then its value.
pub(crate) fn push_operand(pool: &mut Vec<WordSlot>, operand: PooledOperand) {
    let (tag, value) = match operand {
        PooledOperand::Register(register) => (0, register.0),
        PooledOperand::Constant(constant) => (1, constant.0),
        PooledOperand::Constants(list) => (2, list.0),
        PooledOperand::File => (3, 0),
    };
    pool.extend_from_slice(&[WordSlot(tag), WordSlot(value)]);
}

/// The operand at a place of the pool.
#[must_use]
pub(crate) fn operand_at(pool: &[WordSlot], at: PoolAt) -> Option<PooledOperand> {
    let value = slot_at(pool, WordSlot(at.0), WordSlot(1))?.0;
    Some(match slot_at(pool, WordSlot(at.0), WordSlot(0))?.0 {
        0 => PooledOperand::Register(Register(value)),
        1 => PooledOperand::Constant(ConstantNumber(value)),
        2 => PooledOperand::Constants(PoolAt(value)),
        3 => PooledOperand::File,
        4..=u32::MAX => return None,
    })
}

/// Writes the record of an inline body.
pub(crate) fn push_inline_body(pool: &mut Vec<WordSlot>, entry: CodeEntry) -> PoolAt {
    let at = next_place(pool);
    pool.extend_from_slice(&[WordSlot(0), WordSlot(entry.0)]);
    at
}

/// Writes the record of a use of a pattern in call form. Its entry and its unit are written again once the pattern's program
/// has its place and its unit (`call_entry_place`, `call_unit_place`): until then the unit is `NO_SLOT`. The need site of
/// the pattern's definition is known when the record is written (the site is numbered before its formula is), so it has
/// no place function.
pub(crate) fn push_call_body(
    pool: &mut Vec<WordSlot>,
    entry: CodeEntry,
    memo: MemoSite,
    need: NeedSite,
    bound: &[BoundFormal],
) -> PoolAt {
    let at = next_place(pool);
    let count = count_of(bound);
    pool.extend_from_slice(&[
        WordSlot(1),
        WordSlot(entry.0),
        WordSlot(memo.0),
        NO_SLOT,
        WordSlot(need.0),
        count,
    ]);
    for each in bound {
        pool.push(WordSlot(each.formal.0));
        push_operand(pool, each.value);
    }
    at
}

/// The place of the entry in the record of a use of a pattern in call form that starts at `record`: what the encoding writes
/// again once the pattern's program has its place.
#[must_use]
pub(crate) const fn call_entry_place(record: PoolAt) -> PoolAt {
    PoolAt(record.0.saturating_add(1))
}

/// The place of the unit in the record of a use of a pattern in call form that starts at `record`.
#[must_use]
pub(crate) const fn call_unit_place(record: PoolAt) -> PoolAt {
    PoolAt(record.0.saturating_add(3))
}

/// Writes the record of a use of a pattern in table form.
pub(crate) fn push_table_body(
    pool: &mut Vec<WordSlot>,
    index: IndexSite,
    values: &[PooledOperand],
    reads: &[CollectedRead],
) -> PoolAt {
    let at = next_place(pool);
    pool.extend_from_slice(&[WordSlot(2), WordSlot(index.0), count_of(values)]);
    for value in values {
        push_operand(pool, *value);
    }
    pool.push(count_of(reads));
    for read in reads {
        pool.extend_from_slice(&[WordSlot(u32::from(read.column.0)), WordSlot(read.out.0)]);
    }
    at
}

/// The body record at a place of the pool, and the place after it.
#[must_use]
pub(crate) fn body_at(pool: &[WordSlot], at: PoolAt) -> Option<ReadBody> {
    let slot = |ahead| slot_at(pool, WordSlot(at.0), WordSlot(ahead));
    let (record, slots) = match slot(0)?.0 {
        0 => (
            BodyRecord::Inline {
                entry: CodeEntry(slot(1)?.0),
            },
            2,
        ),
        1 => {
            let bound = slot(5)?;
            let record = BodyRecord::Call {
                entry: CodeEntry(slot(1)?.0),
                memo: MemoSite(slot(2)?.0),
                unit: slot(3)?,
                need: NeedSite(slot(4)?.0),
                bound,
                formals: PoolAt(at.0.checked_add(6)?),
            };
            (
                record,
                bound
                    .0
                    .checked_mul(OPERAND_SLOTS.0.checked_add(1)?)?
                    .checked_add(6)?,
            )
        }
        2 => {
            let bound = slot(2)?;
            let values = bound.0.checked_mul(OPERAND_SLOTS.0)?;
            let collected = slot(values.checked_add(3)?)?;
            let record = BodyRecord::Table {
                index: IndexSite(slot(1)?.0),
                bound,
                values: PoolAt(at.0.checked_add(3)?),
                collected,
                reads: PoolAt(at.0.checked_add(values)?.checked_add(4)?),
            };
            (
                record,
                collected
                    .0
                    .checked_mul(2)?
                    .checked_add(values)?
                    .checked_add(4)?,
            )
        }
        3..=u32::MAX => return None,
    };
    Some(ReadBody {
        record,
        after: PoolAt(at.0.checked_add(slots)?),
    })
}

/// Writes a need record, answering its place among the records.
pub(crate) fn push_need(needs: &mut Vec<WordSlot>, record: NeedRecord) -> WordSlot {
    let at = WordSlot(next_place(needs).0);
    match record {
        NeedRecord::Unknown => needs.push(WordSlot(0)),
        NeedRecord::Kinds(kinds) => needs.extend_from_slice(&[WordSlot(1), WordSlot(kinds.0)]),
        NeedRecord::Row {
            relation,
            column,
            constant,
        } => {
            needs.extend_from_slice(&[WordSlot(2), WordSlot(relation.0), column, constant]);
        }
        NeedRecord::Site(site) => needs.extend_from_slice(&[WordSlot(3), WordSlot(site.0)]),
        NeedRecord::Table(table) => needs.extend_from_slice(&[WordSlot(4), WordSlot(table.0)]),
        NeedRecord::Path {
            comparison,
            regex,
            constant,
        } => needs.extend_from_slice(&[WordSlot(7), comparison, regex, constant]),
        NeedRecord::Bytes { regex, constant } => needs.extend_from_slice(&[WordSlot(8), regex, constant]),
        NeedRecord::RowTest {
            relation,
            column,
            comparison,
            regex,
            constant,
        } => needs.extend_from_slice(&[WordSlot(9), WordSlot(relation.0), column, comparison, regex, constant]),
        // The offsets of the parts follow the count: the caller writes them, each the place of a part's record.
        NeedRecord::All { parts, .. } => needs.extend_from_slice(&[WordSlot(5), parts]),
        NeedRecord::Any { parts, .. } => needs.extend_from_slice(&[WordSlot(6), parts]),
    }
    at
}

/// The need record at a place among the needs.
#[must_use]
pub(crate) fn need_at(needs: &[WordSlot], at: WordSlot) -> Option<NeedRecord> {
    let slot = |ahead| slot_at(needs, at, WordSlot(ahead));
    Some(match slot(0)?.0 {
        0 => NeedRecord::Unknown,
        1 => NeedRecord::Kinds(PoolAt(slot(1)?.0)),
        2 => NeedRecord::Row {
            relation: RelationNumber(slot(1)?.0),
            column: slot(2)?,
            constant: slot(3)?,
        },
        3 => NeedRecord::Site(NeedSite(slot(1)?.0)),
        4 => NeedRecord::Table(MadeTableId(slot(1)?.0)),
        5 => NeedRecord::All {
            parts: slot(1)?,
            offsets: WordSlot(at.0.checked_add(2)?),
        },
        6 => NeedRecord::Any {
            parts: slot(1)?,
            offsets: WordSlot(at.0.checked_add(2)?),
        },
        7 => NeedRecord::Path {
            comparison: slot(1)?,
            regex: slot(2)?,
            constant: slot(3)?,
        },
        8 => NeedRecord::Bytes {
            regex: slot(1)?,
            constant: slot(2)?,
        },
        9 => NeedRecord::RowTest {
            relation: RelationNumber(slot(1)?.0),
            column: slot(2)?,
            comparison: slot(3)?,
            regex: slot(4)?,
            constant: slot(5)?,
        },
        10..=u32::MAX => return None,
    })
}

/// The column a slot of a graph fill's record names, `ahead` places after `at`; none for a number no column has.
fn column_at(pool: &[WordSlot], at: PoolAt, ahead: WordSlot) -> Option<MadeColumn> {
    match u16::try_from(slot_at(pool, WordSlot(at.0), ahead)?.0) {
        Ok(column) => Some(MadeColumn(column)),
        Err(_) => None,
    }
}

/// The slot that names a column of a graph fill's record.
fn column_slot(column: MadeColumn) -> WordSlot {
    WordSlot(u32::from(column.0))
}

/// Writes the record of the edge columns of a `follow` or a `cycles` fill: `source, destination`. The place it answers is
/// the one parameter of the table's fill record.
pub(crate) fn push_edge_columns(pool: &mut Vec<WordSlot>, columns: EdgeColumns) -> PoolAt {
    let at = next_place(pool);
    pool.extend_from_slice(&[
        column_slot(columns.source),
        column_slot(columns.destination),
    ]);
    at
}

/// The edge columns of a `follow` or a `cycles` fill, at the place its table's one parameter names.
#[must_use]
pub(crate) fn edge_columns_at(pool: &[WordSlot], at: PoolAt) -> Option<EdgeColumns> {
    Some(EdgeColumns {
        source: column_at(pool, at, WordSlot(0))?,
        destination: column_at(pool, at, WordSlot(1))?,
    })
}

/// Writes the record of the columns of an `ordered-dfs` fill: `root, source, destination, edge`. The place it answers is
/// the one parameter of the table's fill record.
pub(crate) fn push_visit_columns(pool: &mut Vec<WordSlot>, columns: VisitColumns) -> PoolAt {
    let at = next_place(pool);
    pool.extend_from_slice(&[
        column_slot(columns.root),
        column_slot(columns.source),
        column_slot(columns.destination),
        column_slot(columns.edge),
    ]);
    at
}

/// The columns of an `ordered-dfs` fill, at the place its table's one parameter names.
#[must_use]
pub(crate) fn visit_columns_at(pool: &[WordSlot], at: PoolAt) -> Option<VisitColumns> {
    Some(VisitColumns {
        root: column_at(pool, at, WordSlot(0))?,
        source: column_at(pool, at, WordSlot(1))?,
        destination: column_at(pool, at, WordSlot(2))?,
        edge: column_at(pool, at, WordSlot(3))?,
    })
}

/// Writes the record of the columns of a `worklist` fill: `root, state, source, destination, before, after, lower,
/// upper`. The place it answers is the one parameter of the table's fill record.
pub(crate) fn push_worklist_columns(pool: &mut Vec<WordSlot>, columns: WorklistColumns) -> PoolAt {
    let at = next_place(pool);
    pool.extend_from_slice(&[
        column_slot(columns.root),
        column_slot(columns.state),
        column_slot(columns.source),
        column_slot(columns.destination),
        column_slot(columns.before),
        column_slot(columns.after),
        column_slot(columns.lower),
        column_slot(columns.upper),
    ]);
    at
}

/// The columns of a `worklist` fill, at the place its table's one parameter names.
#[must_use]
pub(crate) fn worklist_columns_at(pool: &[WordSlot], at: PoolAt) -> Option<WorklistColumns> {
    Some(WorklistColumns {
        root: column_at(pool, at, WordSlot(0))?,
        state: column_at(pool, at, WordSlot(1))?,
        source: column_at(pool, at, WordSlot(2))?,
        destination: column_at(pool, at, WordSlot(3))?,
        before: column_at(pool, at, WordSlot(4))?,
        after: column_at(pool, at, WordSlot(5))?,
        lower: column_at(pool, at, WordSlot(6))?,
        upper: column_at(pool, at, WordSlot(7))?,
    })
}
