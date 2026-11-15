//! The `ordered_match` item of the `kernels` collection: a blocking kernel that orders both inputs by key then order and
//! pairs each left row with the earliest right row after it in an equal-key group, first in, first out. A relation's rows
//! are copied into the kernel's own table first: a blocking kernel may materialize what it reads.

#[cfg(test)]
use super::super::interface::{KernelsItem, Reach};
use super::super::scan_facts::item::{count_of, kinds_of, row_of};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::ordered_match::types::OrderedParts;
use crate::domain::physical_plan::types::InputRows;
#[cfg(test)]
use crate::domain::physical_plan::types::Lowering;
use crate::domain::table_texts::types::Texts;
use crate::domain::table_texts::types::derived_texts::value::DerivedTexts;
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::Side;
use ryo_values::domain::rows::types::{RowCount, RowIndex, Table};

/// `ordered_match`, one item of the `kernels` collection.
#[cfg(test)]
pub(crate) struct OrderedMatchItem;

#[cfg(test)]
impl KernelsItem for OrderedMatchItem {
    fn reached_by(&self, lowering: &Lowering) -> Reach {
        if matches!(lowering, Lowering::OrderedMatch { .. }) {
            Reach::Runs
        } else {
            Reach::Skips
        }
    }
}

/// `rows` as a table: a table as it is, a relation's rows copied into `copy`.
fn table_of<'a>(rows: InputRows<'a>, copy: &'a mut Table, row: &mut Vec<Value>) -> &'a Table {
    match rows {
        InputRows::Table(table) => table,
        InputRows::Facts(_) | InputRows::Tree(_) => {
            copy.kinds = kinds_of(rows).to_vec();
            copy.cells.clear();
            for at in 0..count_of(rows).0 {
                copy.cells
                    .extend_from_slice(row_of(rows, RowIndex(at), row));
            }
            copy.count = count_of(rows);
            copy
        }
    }
}

/// Orders `left` and `right` by `ordered`'s keys and orders, and writes into `output` each pair's output columns, first in,
/// first out within each key.
///
/// # Errors
///
/// The ordering's fault (`Derived`): keys of unequal kinds, an order that is no number, or a duplicate order;
/// `NoSuchColumn` for an output column past its side.
pub(crate) fn matched(
    ordered: &OrderedParts,
    left: InputRows<'_>,
    right: InputRows<'_>,
    texts: Texts<'_>,
    output: &mut Table,
) -> Result<(), PortableOperatorFault> {
    let (mut left_copy, mut right_copy, mut row) = (Table::default(), Table::default(), Vec::new());
    let left = table_of(left, &mut left_copy, &mut row);
    let right = table_of(right, &mut right_copy, &mut row);
    output.kinds = ordered
        .output
        .iter()
        .map(|column| {
            let side = match column.side {
                Side::Left => left,
                Side::Right => right,
            };
            side.kinds
                .get(usize::try_from(column.column.0).unwrap_or(usize::MAX))
                .copied()
                .ok_or(PortableOperatorFault::NoSuchColumn {
                    index: column.column,
                })
        })
        .collect::<Result<_, _>>()?;
    let ordering = ryo_derived::domain::ordered::service::build(
        left,
        right,
        &ordered.left_on,
        &ordered.right_on,
        ordered.left_order,
        ordered.right_order,
        &DerivedTexts(texts),
        &mut row,
    )
    .map_err(PortableOperatorFault::Derived)?;
    crate::domain::ordered_match::service::paired(
        &ordered.output,
        &ordering,
        left,
        right,
        &mut |pair| {
            output.cells.extend_from_slice(pair);
            output.count = RowCount(output.count.0 + 1);
            Ok(())
        },
    )
}
