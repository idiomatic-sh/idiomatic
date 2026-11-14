//! The `hash_index` kernel, built into the worker's one keyed-row table with no allocation per row, and the `lookup` kernel,
//! which reads a constant key's rows from it in source row order.
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::service::filter::service::{filter, keep, kept_kinds};
use crate::domain::physical_plan::types::file_buffers::value::{
    FileBuffers, IndexedRow, KeyedRows, KeyedRun, NO_ROW,
};
use crate::domain::physical_plan::types::physical_plan::value::PhysicalPlan;
use crate::domain::physical_plan::types::{
    FactRows, IndexPlace, KernelRun, Lowering, PhysicalNode, PlanInputs, PlanNodeId,
};
use crate::domain::table_texts::types::Texts;
use ryo_language::domain::symbols::service::hash_in;
use ryo_relations::domain::relations::service::from_bits;
use ryo_relations::domain::relations::types::{Bits, CellKind, ColumnsView, Relation, Value};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::types::{RowCount, RowIndex, Table};

/// Builds the shared index `index` from one read of its relation and answers every lookup of it: each the rows whose key is
/// its constant, in source row order, kept by all its comparisons.
///
/// # Errors
///
/// A refused relation; `NoSuchColumn` for a kept column past the relation's rows.
pub(crate) fn lookups(
    plan: &PhysicalPlan,
    run: &mut KernelRun<'_>,
    index: IndexPlace,
    buffers: &mut FileBuffers,
) -> Result<(), PortableOperatorFault> {
    let shared = plan
        .indexes
        .get(index.0)
        .ok_or(PortableOperatorFault::WrongOperation)?;
    let view = facts(run.inputs, shared.relation)?;
    let texts = run.resources.texts();
    build(&view, shared.column, &texts, &mut buffers.index);
    for lookup in &shared.lookups {
        let place = usize::try_from(lookup.0).unwrap_or(usize::MAX);
        let (
            Some(PhysicalNode {
                lowering: Lowering::Lookup { key, select, .. },
                ..
            }),
            Some(decisions),
        ) = (plan.nodes.get(place), buffers.decisions.get_mut(place))
        else {
            continue;
        };
        let mut output = Table {
            kinds: kept_kinds(view.kinds, &select.kept)?,
            ..Table::default()
        };
        let kernel = filter(select, view.kinds, texts, decisions);
        let mut fault = None;
        let mut row = buffers
            .index
            .keys
            .find(key.0, |run| run.hash == *key)
            .map_or(NO_ROW, |run| run.first);
        while row != NO_ROW {
            let at = usize::try_from(row.0).unwrap_or(usize::MAX);
            decode(&view, RowIndex(at), &mut buffers.row);
            if let Err(error) = keep(
                kernel,
                run.matcher,
                &buffers.row,
                decisions,
                &mut buffers.written,
                &mut output,
            ) {
                fault = Some(error);
                break;
            }
            row = buffers.index.next.get(at).copied().unwrap_or(NO_ROW);
        }
        hold(plan, buffers, *lookup, output, fault);
    }
    Ok(())
}

/// Keys the relation's rows by the hash of `column`'s text into `rows`: each key's first and last row, and each row's next
/// of the same key, ascending. A cell with no text is no key.
fn build(view: &ColumnsView<'_>, column: ColumnIndex, texts: &Texts<'_>, rows: &mut KeyedRows) {
    rows.keys.clear();
    rows.next.clear();
    rows.next.resize(rows_of(view).0, NO_ROW);
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
        match rows.keys.find_mut(hash.0, |run| run.hash == hash) {
            Some(run) => {
                if let Some(link) = rows
                    .next
                    .get_mut(usize::try_from(run.last.0).unwrap_or(usize::MAX))
                {
                    *link = row;
                }
                run.last = row;
            }
            None => {
                let _: hashbrown::hash_table::OccupiedEntry<'_, KeyedRun> =
                    rows.keys.insert_unique(
                        hash.0,
                        KeyedRun {
                            hash,
                            first: row,
                            last: row,
                        },
                        |run| run.hash.0,
                    );
            }
        }
    }
}

/// A relation's rows as the facts lend them.
///
/// # Errors
///
/// `NotRun` for a relation the facts refused: a fault, never empty rows.
pub(crate) fn facts(
    inputs: &dyn PlanInputs,
    relation: Relation,
) -> Result<ColumnsView<'_>, PortableOperatorFault> {
    match inputs.facts(relation) {
        FactRows::Lent(view) => Ok(view),
        FactRows::Refused => Err(PortableOperatorFault::NotRun),
    }
}

/// How many rows a relation's columns hold.
pub(crate) fn rows_of(view: &ColumnsView<'_>) -> RowCount {
    RowCount(view.columns.first().map_or(0, |column| column.0.len()))
}

/// Row `row` of the columns, decoded into `into` one cell per column: the facts are read in place, never into a table.
pub(crate) fn decode(view: &ColumnsView<'_>, row: RowIndex, into: &mut Vec<Value>) {
    into.clear();
    // A column shorter than the row is a row of another width, which the filter refuses: no cell is made up for it.
    into.extend(
        view.kinds
            .iter()
            .zip(view.columns)
            .filter_map(|(kind, column)| {
                column
                    .0
                    .get(row.0)
                    .map(|bits| from_bits(*kind, Bits(*bits)))
            }),
    );
}

/// Holds a node's rows and pending fault, with all its readers still to come.
pub(crate) fn hold(
    plan: &PhysicalPlan,
    buffers: &mut FileBuffers,
    node: PlanNodeId,
    rows: Table,
    pending: Option<PortableOperatorFault>,
) {
    let place = usize::try_from(node.0).unwrap_or(usize::MAX);
    if let Some(held) = buffers.nodes.get_mut(place) {
        *held = Some(rows);
    }
    if let Some(slot) = buffers.pending.get_mut(place) {
        *slot = pending;
    }
    if let (Some(left), Some(physical)) = (buffers.remaining.get_mut(place), plan.nodes.get(place))
    {
        *left = physical.readers;
    }
}
