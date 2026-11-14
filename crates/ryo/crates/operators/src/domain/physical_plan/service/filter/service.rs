//! The `filter` kernel over the moved comparisons, unchanged, and the `project` kernel over the projection's one meaning.
use crate::domain::comparing::service::holds_bound;
use crate::domain::comparing::types::{DecisionWorkspace, NameCount, RowShape};
use crate::domain::contract::types::{PortableOperatorFault, PredicateOutcome};
use crate::domain::patterns::types::PatternMatcher;
use crate::domain::physical_plan::types::file_buffers::value::NodeDecisions;
use crate::domain::physical_plan::types::{FilterKernel, SelectParts};
use crate::domain::table_scratch::types::Memo;
use crate::domain::table_texts::types::Texts;
use ryo_relations::domain::relations::types::{CellKind, Value};
use ryo_values::domain::plan_parts::types::{ColumnIndex, RepeatedRows};
use ryo_values::domain::rows::types::{RowCount, Table};

/// The filter of `select` over rows of `kinds`, its memo of each name's decisions readied for the open file.
pub(crate) fn filter<'f>(
    select: &'f SelectParts,
    kinds: &'f [CellKind],
    texts: Texts<'f>,
    decisions: &mut NodeDecisions,
) -> FilterKernel<'f> {
    let names = NameCount(ryo_language::domain::names::service::name_count(texts.symbols).0);
    decisions.memo.clear();
    decisions
        .memo
        .resize(select.comparisons.len() * names.0, Memo::Unknown);
    FilterKernel {
        select,
        kinds,
        texts,
        names,
    }
}

/// Appends `row`, with the kept columns, to `output` when every comparison holds for it.
///
/// # Errors
///
/// `ShapeMismatch` for a row of another width; the first comparison's fault the row raises; `NoSuchColumn` for a kept column
/// past the row.
pub(crate) fn keep(
    filter: FilterKernel<'_>,
    matcher: PatternMatcher<'_>,
    row: &[Value],
    decisions: &mut NodeDecisions,
    written: &mut Vec<Value>,
    output: &mut Table,
) -> Result<(), PortableOperatorFault> {
    let comparisons = &filter.select.comparisons;
    if !comparisons.is_empty() && row.len() != filter.kinds.len() {
        return Err(PortableOperatorFault::ShapeMismatch);
    }
    let holds = holds_bound(
        comparisons,
        row,
        RowShape {
            kinds: filter.kinds,
            names: filter.names,
        },
        &filter.texts,
        &filter.select.bindings,
        None,
        matcher,
        DecisionWorkspace {
            select: &mut decisions.select,
            memo: &mut decisions.memo,
        },
    )
    .map_err(PortableOperatorFault::from)?;
    if holds == PredicateOutcome::Fails {
        return Ok(());
    }
    if filter.select.kept.is_empty() {
        output.cells.extend_from_slice(row);
    } else {
        written.clear();
        crate::domain::project::service::project(row, &filter.select.kept, written)?;
        output.cells.extend_from_slice(written);
    }
    output.count = RowCount(output.count.0 + 1);
    Ok(())
}

/// The kinds of the kept columns of a row of `kinds`; none kept keeps every column.
///
/// # Errors
///
/// `NoSuchColumn` for a kept column past the row.
pub(crate) fn kept_kinds(
    kinds: &[CellKind],
    kept: &[ColumnIndex],
) -> Result<Vec<CellKind>, PortableOperatorFault> {
    if kept.is_empty() {
        return Ok(kinds.to_vec());
    }
    kept.iter()
        .map(|column| {
            kinds
                .get(usize::try_from(column.0).unwrap_or(usize::MAX))
                .copied()
                .ok_or(PortableOperatorFault::NoSuchColumn { index: *column })
        })
        .collect()
}

/// Appends a row's projection to `output`, the first time only when repeated rows are dropped. Repeats are found through
/// `seen`, which holds the place of each kept row of `output` and copies no row.
///
/// # Errors
///
/// `NoSuchColumn` for a column past the row.
pub(crate) fn projected(
    row: &[Value],
    columns: &[ColumnIndex],
    distinct: RepeatedRows,
    seen: &mut hashbrown::HashTable<RowCount>,
    output: &mut Table,
) -> Result<(), PortableOperatorFault> {
    let start = output.cells.len();
    crate::domain::project::service::project(row, columns, &mut output.cells)?;
    if distinct == RepeatedRows::Dropped {
        let width = columns.len();
        let hasher = rustc_hash::FxBuildHasher;
        let cells = &output.cells;
        let row_at = |at: RowCount| {
            cells
                .get(at.0 * width..at.0 * width + width)
                .unwrap_or_default()
        };
        let new = cells.get(start..).unwrap_or_default();
        let hash = std::hash::BuildHasher::hash_one(&hasher, new);
        if seen.find(hash, |earlier| row_at(*earlier) == new).is_some() {
            output.cells.truncate(start);
            return Ok(());
        }
        let _: hashbrown::hash_table::OccupiedEntry<'_, RowCount> =
            seen.insert_unique(hash, output.count, |earlier| {
                std::hash::BuildHasher::hash_one(&hasher, row_at(*earlier))
            });
    }
    output.count = RowCount(output.count.0 + 1);
    Ok(())
}
