//! Ordered predicate positions and collision-checked distinct representatives.
use super::types::LiteralPostings;
use crate::domain::fault::types::DerivedFault;
use crate::domain::key::service::{mix, word_of};
use crate::domain::key::types::KeyCellsHash;
use crate::domain::positions::types::RowPosition;
use crate::domain::rows::types::{PredicateRows, RowCells, Verdict};
use crate::domain::sharing::types::shared_positions::value::SharedPositions;
use ryo_language::domain::symbols::{
    service::text_of,
    types::{ByteSlice, Symbols},
};
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::{ColumnIndex, LiteralText};
use ryo_values::domain::rows::types::RowIndex;

/// The positions of the rows that satisfy the predicate, in ascending order.
pub(crate) fn matches(
    rows: &mut dyn PredicateRows,
    candidates: Option<&[RowPosition]>,
) -> Result<SharedPositions, DerivedFault> {
    let count = rows.count();
    let mut output = Vec::new();
    let mut decide = |at: RowIndex| -> Result<(), DerivedFault> {
        if rows.holds(at)? == Verdict::Holds {
            output.push(RowPosition(
                u32::try_from(at.0).map_err(|_| DerivedFault::ShapeMismatch)?,
            ));
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
            decide(RowIndex(at.0 as usize))?;
        }
    } else {
        for at in 0..count.0 {
            decide(RowIndex(at))?;
        }
    }
    Ok(SharedPositions::new(output))
}

/// The first position of each distinct key among the given positions.
pub(crate) fn distinct(
    rows: &dyn RowCells,
    columns: &[ColumnIndex],
    positions: &[RowPosition],
) -> Result<SharedPositions, DerivedFault> {
    let mut seen = std::collections::HashMap::<KeyCellsHash, Vec<RowPosition>>::new();
    let mut output = Vec::new();
    for &at in positions {
        let hash = columns.iter().try_fold(KeyCellsHash(0), |hash, column| {
            let value = rows
                .cell(RowIndex(at.0 as usize), *column)
                .ok_or(DerivedFault::ShapeMismatch)?;
            Ok::<_, DerivedFault>(mix(hash, word_of(&value)))
        })?;
        let bucket = seen.entry(hash).or_default();
        if !bucket.iter().any(|&prior| {
            columns.iter().all(|column| {
                rows.cell(RowIndex(prior.0 as usize), *column)
                    == rows.cell(RowIndex(at.0 as usize), *column)
            })
        }) {
            bucket.push(at);
            output.push(at);
        }
    }
    Ok(SharedPositions::new(output))
}

/// The positions of the rows whose symbol stands for each active literal.
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
    let mut collected = vec![Vec::new(); active.len()];
    for at in 0..rows.count().0 {
        let Some(Value::Symbol(symbol)) = rows.cell(RowIndex(at), column) else {
            return Err(DerivedFault::ShapeMismatch);
        };
        let Some(text) = text_of(symbols, source, symbol) else {
            continue;
        };
        if let Ok(place) = active.binary_search_by(|entry| (*entry.0).cmp(text.0)) {
            collected
                .get_mut(place)
                .ok_or(DerivedFault::NotRun)?
                .push(RowPosition(
                    u32::try_from(at).map_err(|_| DerivedFault::ShapeMismatch)?,
                ));
        }
    }
    Ok(active
        .iter()
        .cloned()
        .zip(collected)
        .map(|(text, rows)| LiteralPostings {
            text,
            rows: SharedPositions::new(rows),
        })
        .collect())
}
