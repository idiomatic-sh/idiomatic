//! Existing ordered matching, retaining exact duplicate-order error semantics.
use super::types::{OrderedMatching, OrderedText, OwnedOrderedKey, OwnedOrderedRow};
use crate::domain::fault::types::DerivedFault;
use crate::domain::row_cells::types::{RowCells, ValueTexts};
use ryo_relations::domain::relations::types::{CellKind, Value};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::types::RowIndex;
fn cell(row: &[Value], column: ColumnIndex) -> Result<Value, DerivedFault> {
    row.get(usize::from(column))
        .copied()
        .ok_or(DerivedFault::NoSuchColumn { column })
}
fn check_columns(kinds: &[CellKind], columns: &[ColumnIndex]) -> Result<(), DerivedFault> {
    for column in columns {
        if usize::try_from(column.0).map_or(true, |at| at >= kinds.len()) {
            return Err(DerivedFault::NoSuchColumn { column: *column });
        }
    }
    Ok(())
}
fn ordered_rows(
    input: &dyn RowCells,
    keys: &[ColumnIndex],
    order: ColumnIndex,
    texts: &dyn ValueTexts,
    buffer: &mut Vec<Value>,
) -> Result<Vec<OwnedOrderedRow>, DerivedFault> {
    let mut rows = Vec::with_capacity(input.count().0);
    for at in 0..input.count().0 {
        let Some(row) = input.read_row(RowIndex(at), buffer) else {
            continue;
        };
        let Value::Number(order) = cell(row, order)? else {
            return Err(DerivedFault::ShapeMismatch);
        };
        let group = keys
            .iter()
            .map(|column| {
                let value = cell(row, *column)?;
                Ok(match value {
                    Value::Symbol(_) | Value::Name(_) => {
                        let Some(text) = texts.text(value) else {
                            return Err(DerivedFault::ShapeMismatch);
                        };
                        OwnedOrderedKey::Text(OrderedText(text.0.to_owned()))
                    }
                    Value::Node(_)
                    | Value::File(_)
                    | Value::Span(_)
                    | Value::Number(_)
                    | Value::Flag(_) => OwnedOrderedKey::Value(value),
                })
            })
            .collect::<Result<Vec<_>, DerivedFault>>()?;
        rows.push(OwnedOrderedRow {
            keys: group,
            order,
            row: RowIndex(at),
        });
    }
    rows.sort_unstable();
    if rows.windows(2).any(|pair| matches!(pair, [first, second] if first.keys == second.keys && first.order == second.order)) {
        return Err(DerivedFault::DuplicateOrder);
    }
    Ok(rows)
}

/// Both sides ordered by key then order, each row's keys read as text where a key is a name.
///
/// # Errors
///
/// `ShapeMismatch` for keys of unequal count or kind, an order that is no number, or a name key with no text;
/// `NoSuchColumn` for a key column past its side; `DuplicateOrder` for two rows of one key with one order.
///
/// # Examples
///
/// ```
/// use ryo_derived::domain::ordered::service::build;
/// use ryo_derived::domain::row_cells::types::ValueTexts;
/// use ryo_relations::domain::relations::types::{CellKind, Integer, Value};
/// use ryo_values::domain::plan_parts::types::ColumnIndex;
/// use ryo_values::domain::row_texts::types::CellText;
/// use ryo_values::domain::rows::types::{RowCount, Table};
/// struct NoTexts;
/// impl ValueTexts for NoTexts {
///     fn text(&self, _value: Value) -> Option<CellText<'_>> {
///         None
///     }
/// }
/// let rows = Table {
///     kinds: vec![CellKind::Number],
///     cells: vec![Value::Number(Integer(2)), Value::Number(Integer(1))],
///     count: RowCount(2),
/// };
/// let order = ColumnIndex(0);
/// let ordered = build(&rows, &rows, &[], &[], order, order, &NoTexts, &mut Vec::new());
/// assert!(ordered.is_ok_and(|ordered| ordered.left[0].row.0 == 1));
/// ```
pub fn build(
    left: &dyn RowCells,
    right: &dyn RowCells,
    left_on: &[ColumnIndex],
    right_on: &[ColumnIndex],
    left_order: ColumnIndex,
    right_order: ColumnIndex,
    texts: &dyn ValueTexts,
    buffer: &mut Vec<Value>,
) -> Result<OrderedMatching, DerivedFault> {
    let kind =
        |input: &dyn RowCells, column: ColumnIndex| input.kinds().get(usize::from(column)).copied();
    if left_on.len() != right_on.len()
        || kind(left, left_order) != Some(CellKind::Number)
        || kind(right, right_order) != Some(CellKind::Number)
    {
        return Err(DerivedFault::ShapeMismatch);
    }
    check_columns(left.kinds(), left_on)?;
    check_columns(right.kinds(), right_on)?;
    for (left_key, right_key) in left_on.iter().zip(right_on) {
        if kind(left, *left_key) != kind(right, *right_key) {
            return Err(DerivedFault::ShapeMismatch);
        }
    }
    Ok(OrderedMatching {
        left: ordered_rows(left, left_on, left_order, texts, buffer)?,
        right: ordered_rows(right, right_on, right_order, texts, buffer)?,
    })
}
