//! Count/digest semantics over borrowed engine-owned populations, indexes, and reduction scratch.
use crate::domain::contract::types::PortableOperatorFault as Fault;
use crate::domain::reduction::types::{
    AggregationCheckOrder, AggregationHash, AggregationPlan, DigestDatum, DigestValues,
    GroupColumns, GroupOutput, StreamingReduction,
};
use ryo_relations::domain::relations::types::{CellKind, Value};
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, FlagState},
    rows::types::{RowCount, RowIndex, Table},
};
use std::hash::Hasher as _;

fn row(table: &Table, at: RowIndex) -> &[Value] {
    let width = table.kinds.len();
    table
        .cells
        .get(at.0 * width..(at.0 + 1) * width)
        .filter(|_| at.0 < table.count.0)
        .unwrap_or_default()
}
fn cell(row: &[Value], column: ColumnIndex) -> Result<Value, Fault> {
    row.get(usize::try_from(column.0).unwrap_or(usize::MAX))
        .copied()
        .ok_or(Fault::NoSuchColumn { index: column.0 })
}
fn check(kinds: &[CellKind], column: ColumnIndex) -> Result<CellKind, Fault> {
    kinds
        .get(usize::try_from(column.0).unwrap_or(usize::MAX))
        .copied()
        .ok_or(Fault::NoSuchColumn { index: column.0 })
}
fn same(table: &Table, on: GroupColumns<'_>, left: RowIndex, right: RowIndex) -> bool {
    let (left, right) = (row(table, left), row(table, right));
    match on {
        GroupColumns::Whole => left == right,
        GroupColumns::Columns(columns) => columns.iter().all(|column| {
            let at = usize::try_from(column.0).unwrap_or(usize::MAX);
            left.get(at) == right.get(at)
        }),
    }
}
pub(crate) fn count_groups(
    table: &Table,
    on: GroupColumns<'_>,
    distinct: Option<ColumnIndex>,
    order: &[RowIndex],
    values: &mut Vec<Value>,
    groups: &mut dyn GroupOutput,
) -> Result<(), Fault> {
    if let Some(column) = distinct {
        check(&table.kinds, column)?;
    }
    if let GroupColumns::Columns(keys) = on {
        for column in keys {
            check(&table.kinds, *column)?;
        }
    }
    groups.clear();
    for run in order.chunk_by(|left, right| same(table, on, *left, *right)) {
        let first = run.iter().copied().min().unwrap_or(RowIndex(0));
        let counted = if let Some(column) = distinct {
            values.clear();
            values.extend(
                run.iter()
                    .filter_map(|at| cell(row(table, *at), column).ok()),
            );
            values.sort_unstable();
            values.dedup();
            values.len()
        } else {
            run.len()
        };
        groups.push(first, RowCount(counted), 0);
    }
    groups.sort_first();
    Ok(())
}
pub(crate) fn digest_groups(
    table: &Table,
    keys: &[ColumnIndex],
    over: ColumnIndex,
    order: &[RowIndex],
    hashes: &mut Vec<u64>,
    hasher: &dyn DigestValues,
    groups: &mut dyn GroupOutput,
) -> Result<(), Fault> {
    check(&table.kinds, over)?;
    for column in keys {
        check(&table.kinds, *column)?;
    }
    groups.clear();
    for run in order.chunk_by(|left, right| same(table, GroupColumns::Columns(keys), *left, *right))
    {
        let first = run.iter().copied().min().unwrap_or(RowIndex(0));
        hashes.clear();
        for value in run.iter().filter_map(|at| cell(row(table, *at), over).ok()) {
            hashes.push(hash_of(hasher.datum(value)));
        }
        hashes.sort_unstable();
        hashes.dedup();
        let mut folded = rustc_hash::FxHasher::default();
        for hash in hashes.iter() {
            folded.write_u64(*hash);
        }
        groups.push(first, RowCount(hashes.len()), folded.finish().cast_signed());
    }
    groups.sort_first();
    Ok(())
}
pub(crate) fn hash_of(datum: DigestDatum<'_>) -> u64 {
    let mut hasher = rustc_hash::FxHasher::default();
    match datum {
        DigestDatum::Unlocated => {
            hasher.write(ryo_relations::domain::relations::constants::UNLOCATED.as_bytes())
        }
        DigestDatum::Text(text) => hasher.write(text.as_bytes()),
        DigestDatum::Number(number) => hasher.write_i64(number.0),
        DigestDatum::Flag(flag) => hasher.write_u8(u8::from(flag == FlagState::Raised)),
        DigestDatum::Numbers(numbers) => {
            numbers.iter().for_each(|number| hasher.write_i64(number.0))
        }
        DigestDatum::Texts(items) => items
            .iter()
            .for_each(|text| hasher.write(text.0.as_bytes())),
    }
    hasher.finish()
}
pub(crate) fn check_program(
    program: AggregationPlan<'_>,
    kinds: &[CellKind],
    order: AggregationCheckOrder,
    target: &mut Vec<CellKind>,
) -> Result<(), Fault> {
    let (keys, over, digest) = match program {
        AggregationPlan::Count { keys, distinct } => (keys, distinct, false),
        AggregationPlan::Digest { keys, over } => (keys, Some(over), true),
    };
    if digest || matches!(order, AggregationCheckOrder::Collected) {
        if let Some(over) = over {
            check(kinds, over)?;
        }
    }
    target.clear();
    for key in keys {
        target.push(check(kinds, *key)?);
    }
    if !digest && matches!(order, AggregationCheckOrder::Streaming) {
        if let Some(over) = over {
            check(kinds, over)?;
        }
    }
    Ok(())
}
pub(crate) fn push(
    program: AggregationPlan<'_>,
    row: &[Value],
    key: &mut Vec<Value>,
    driver: &mut dyn StreamingReduction,
) -> Result<(), Fault> {
    let keys = match program {
        AggregationPlan::Count { keys, .. } | AggregationPlan::Digest { keys, .. } => keys,
    };
    key.clear();
    for column in keys {
        key.push(cell(row, *column)?);
    }
    let group = driver.group(key, row).map_err(|_| Fault::Interrupted)?;
    match program {
        AggregationPlan::Count { distinct, .. } => {
            let counted = match distinct {
                None => true,
                Some(column) => driver.insert_value(group, cell(row, column)?),
            };
            if counted {
                *driver.count(group).ok_or(Fault::ShapeMismatch)? += 1;
            }
        }
        AggregationPlan::Digest { over, .. } => {
            let hash = hash_of(driver.datum(cell(row, over)?));
            driver.append_hash(AggregationHash {
                group: u32::try_from(group.0).unwrap_or(u32::MAX),
                hash,
            });
        }
    }
    Ok(())
}
pub(crate) fn finish(
    counts: &[usize],
    hashes: &mut Vec<AggregationHash>,
    digest: bool,
    groups: &mut dyn GroupOutput,
) {
    groups.clear();
    if !digest {
        for (group, count) in counts.iter().copied().enumerate() {
            groups.push(RowIndex(group), RowCount(count), 0);
        }
    } else {
        hashes.sort_unstable();
        hashes.dedup();
        let mut at = 0;
        for group in 0..counts.len() {
            let mut hasher = rustc_hash::FxHasher::default();
            let mut counted = 0;
            while let Some(each) = hashes.get(at)
                && usize::try_from(each.group).unwrap_or(usize::MAX) == group
            {
                hasher.write_u64(each.hash);
                counted += 1;
                at += 1;
            }
            groups.push(
                RowIndex(group),
                RowCount(counted),
                hasher.finish().cast_signed(),
            );
        }
    }
}
