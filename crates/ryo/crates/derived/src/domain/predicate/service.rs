//! Ordered predicate positions and collision-checked distinct representatives.
use crate::domain::contract::types::{
    DerivedFault, LiteralPostings, PredicateRows, RowCells, SharedPositions,
};
use ryo_language::domain::symbols::{
    service::text_of,
    types::{ByteSlice, Symbols},
};
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::{ColumnIndex, LiteralText};
use std::sync::Arc;

pub(crate) fn matches(
    rows: &mut dyn PredicateRows,
    candidates: Option<&[u32]>,
) -> Result<SharedPositions, DerivedFault> {
    let count = rows.count();
    let mut output = Vec::new();
    let mut decide = |at: usize| -> Result<(), DerivedFault> {
        if rows.holds(at)? {
            output.push(u32::try_from(at).map_err(|_| DerivedFault::ShapeMismatch)?);
        }
        Ok(())
    };
    if let Some(candidates) = candidates {
        let mut previous = None;
        for &at in candidates {
            if previous.is_some_and(|before| before >= at) {
                return Err(DerivedFault::ShapeMismatch);
            }
            previous = Some(at);
            decide(at as usize)?;
        }
    } else {
        for at in 0..count {
            decide(at)?;
        }
    }
    Ok(SharedPositions(Arc::new(output)))
}
pub(crate) fn distinct(
    rows: &dyn RowCells,
    columns: &[ColumnIndex],
    positions: &[u32],
) -> Result<SharedPositions, DerivedFault> {
    let mut seen = std::collections::HashMap::<u64, Vec<u32>>::new();
    let mut output = Vec::new();
    for &at in positions {
        let hash = columns.iter().try_fold(0_u64, |hash, column| {
            let value = rows
                .cell(at as usize, *column)
                .ok_or(DerivedFault::ShapeMismatch)?;
            Ok::<_, DerivedFault>(
                (hash.rotate_left(5) ^ crate::domain::contract::key::word_of(&value).0)
                    .wrapping_mul(0x517c_c1b7_2722_0a95),
            )
        })?;
        let bucket = seen.entry(hash).or_default();
        if !bucket.iter().any(|&prior| {
            columns
                .iter()
                .all(|column| rows.cell(prior as usize, *column) == rows.cell(at as usize, *column))
        }) {
            bucket.push(at);
            output.push(at);
        }
    }
    Ok(SharedPositions(Arc::new(output)))
}
pub(crate) fn postings(
    rows: &dyn RowCells,
    column: ColumnIndex,
    allowed: &[LiteralText],
    active: &[LiteralText],
    symbols: &Symbols,
    source: ByteSlice<'_>,
) -> Result<Vec<LiteralPostings>, DerivedFault> {
    for (at, text) in active.iter().enumerate() {
        if (at > 0 && active[at - 1].0 >= text.0)
            || allowed
                .binary_search_by(|entry| entry.0.cmp(&text.0))
                .is_err()
        {
            return Err(DerivedFault::WrongOperation);
        }
    }
    let mut output = active
        .iter()
        .map(|text| LiteralPostings {
            text: text.0.to_string(),
            rows: SharedPositions(Arc::new(Vec::new())),
        })
        .collect::<Vec<_>>();
    for at in 0..rows.count() {
        let Some(Value::Symbol(symbol)) = rows.cell(at, column) else {
            return Err(DerivedFault::ShapeMismatch);
        };
        let Some(text) = text_of(symbols, source, symbol) else {
            continue;
        };
        if let Ok(place) = output.binary_search_by(|entry| entry.text.as_str().cmp(text.0)) {
            Arc::get_mut(&mut output[place].rows.0)
                .ok_or(DerivedFault::NotRun)?
                .push(u32::try_from(at).map_err(|_| DerivedFault::ShapeMismatch)?);
        }
    }
    Ok(output)
}
