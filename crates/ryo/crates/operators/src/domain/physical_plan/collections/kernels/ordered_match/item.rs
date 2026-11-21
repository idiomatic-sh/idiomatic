//! The `ordered_match` item of the `kernels` collection: a blocking kernel that orders both inputs by key then order and
//! pairs each left row with the earliest right row after it in an equal-key group, first in, first out. A relation's rows
//! are copied into the kernel's own table first: a blocking kernel may materialize what it reads.

#[cfg(test)]
use super::super::interface::{KernelsItem, Reach};
use super::super::distinct::item::counted;
use super::super::scan_facts::item::{count_of, kinds_of, row_of};
use crate::domain::ordered_match::types::Band;
use crate::domain::physical_plan::types::CellStart;
use crate::domain::physical_plan::types::row_set::value::RowSet;
use crate::domain::table_texts::service::datum;
use crate::domain::table_texts::types::Datum;
use ryo_derived::domain::fault::types::DerivedFault;
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::plan_parts::types::{ColumnIndex, RepeatedRows};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::ordered_groups::types::GroupOrders;
use crate::domain::ordered_match::service::paired;
use crate::domain::ordered_match::types::OrderedParts;
use crate::domain::physical_plan::types::InputRows;
#[cfg(test)]
use crate::domain::physical_plan::types::lowering::value::Lowering;
use crate::domain::table_texts::types::Texts;
use ryo_relations::domain::relations::types::{Integer, Value};
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

/// Orders `left` and `right` by `ordered`'s keys and orders, and writes into `output` each pair's output columns, first in,
/// first out within each key; a band match (`ordered.band`) writes each left row with a right row in its band instead.
/// `same` says both sides are one source read alike, so one ordering serves both.
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
    same: bool,
    repeats: &mut RowSet,
    output: &mut Table,
) -> Result<(), PortableOperatorFault> {
    let left_kinds = kinds_of(left);
    let right_kinds = kinds_of(right);
    output.kinds = ordered
        .output
        .iter()
        .map(|column| {
            let side = match column.side {
                Side::Left => left_kinds,
                Side::Right => right_kinds,
            };
            side.get(usize::try_from(column.column.0).unwrap_or(usize::MAX))
                .copied()
                .ok_or(PortableOperatorFault::NoSuchColumn {
                    index: column.column,
                })
        })
        .collect::<Result<_, _>>()?;
    if let Some(band) = &ordered.band {
        return banded(ordered, band, left, right, repeats, output);
    }
    // Spike (ryo-fmx3): the ordering as `ryo_derived::domain::ordered::service::build` makes it, with its checks and faults,
    // without an owned key per row: keys are borrowed texts or cells, flat, and rows are sorted by index.
    let derived = |fault| PortableOperatorFault::Derived(fault);
    let kind = |kinds: &[CellKind], column: ColumnIndex| kinds.get(usize::from(column)).copied();
    if ordered.left_on.len() != ordered.right_on.len()
        || kind(left_kinds, ordered.left_order) != Some(CellKind::Number)
        || kind(right_kinds, ordered.right_order) != Some(CellKind::Number)
    {
        return Err(derived(DerivedFault::ShapeMismatch));
    }
    for (columns, kinds) in [(&ordered.left_on, left_kinds), (&ordered.right_on, right_kinds)] {
        if let Some(column) = columns.iter().find(|column| usize::from(**column) >= kinds.len()) {
            return Err(derived(DerivedFault::NoSuchColumn { column: *column }));
        }
    }
    if ordered
        .left_on
        .iter()
        .zip(&ordered.right_on)
        .any(|(one, other)| kind(left_kinds, *one) != kind(right_kinds, *other))
    {
        return Err(derived(DerivedFault::ShapeMismatch));
    }
    let mut row = Vec::new();
    let left_side = sorted(left, &ordered.left_on, ordered.left_order, &texts, &mut row).map_err(derived)?;
    let right_owned;
    let right_side = if same && ordered.left_on == ordered.right_on && ordered.left_order == ordered.right_order {
        &left_side
    } else {
        right_owned =
            sorted(right, &ordered.right_on, ordered.right_order, &texts, &mut row).map_err(derived)?;
        &right_owned
    };
    let (mut left_row, mut right_row) = (Vec::new(), Vec::new());
    // Each side's orders in its sorted rows' order: a key's rows are then one stretch of each, and the pairing within the
    // key is the operation's own meaning.
    let in_order = |side: &Sorted<'_>| -> Vec<Integer> { side.rows.iter().map(|at| Integer(side.orders[*at])).collect() };
    let (left_orders, right_orders) = (in_order(&left_side), in_order(right_side));
    let mut pairs = Vec::new();
    let (mut left_from, mut right_from) = (0, 0);
    while left_from < left_side.rows.len() {
        let keys = left_side.keys(left_side.rows[left_from]);
        let left_to = left_from + left_side.rows[left_from..].partition_point(|at| left_side.keys(*at) == keys);
        right_from += right_side.rows[right_from..].partition_point(|at| right_side.keys(*at) < keys);
        let right_to = right_from + right_side.rows[right_from..].partition_point(|at| right_side.keys(*at) == keys);
        pairs.clear();
        paired(
            GroupOrders(&left_orders[left_from..left_to]),
            GroupOrders(&right_orders[right_from..right_to]),
            &mut pairs,
        );
        for pair in &pairs {
            let left_at = left_side.rows[left_from + usize::try_from(pair.left.0).unwrap_or(usize::MAX)];
            let right_at = right_side.rows[right_from + usize::try_from(pair.right.0).unwrap_or(usize::MAX)];
            let left_cells = row_of(left, RowIndex(left_at), &mut left_row);
            let right_cells = row_of(right, RowIndex(right_at), &mut right_row);
            for column in &ordered.output {
                let side = match column.side {
                    Side::Left => left_cells,
                    Side::Right => right_cells,
                };
                output.cells.push(
                    *side
                        .get(usize::try_from(column.column.0).unwrap_or(usize::MAX))
                        .ok_or(PortableOperatorFault::NoSuchColumn {
                            index: column.column,
                        })?,
                );
            }
            output.count = RowCount(output.count.0 + 1);
        }
        left_from = left_to;
        right_from = right_to;
    }
    Ok(())
}

/// Spike (ryo-fmx3): one key cell of an ordered row, ordered as `OwnedOrderedKey` orders it: a text before any cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum KeyCell<'t> {
    /// A symbol's or name's text.
    Text(&'t str),
    /// Any other cell.
    Cell(Value),
}

/// Spike (ryo-fmx3): one side's rows ordered by key, then order, then row.
struct Sorted<'t> {
    /// The key cells of every row, flat, `width` a row.
    keys: Vec<KeyCell<'t>>,
    /// The key width.
    width: usize,
    /// The order of every row.
    orders: Vec<i64>,
    /// The rows, sorted.
    rows: Vec<usize>,
}

impl<'t> Sorted<'t> {
    /// The key cells of row `at`.
    fn keys(&self, at: usize) -> &[KeyCell<'t>] {
        self.keys.get(at * self.width..(at + 1) * self.width).unwrap_or_default()
    }
}

/// Spike (ryo-fmx3): `rows` sorted by `keys` then `order` then row, as the derived ordering sorts them.
fn sorted<'t>(
    rows: InputRows<'_>,
    keys: &[ColumnIndex],
    order: ColumnIndex,
    texts: &Texts<'t>,
    row: &mut Vec<Value>,
) -> Result<Sorted<'t>, DerivedFault> {
    let count = count_of(rows).0;
    let mut side = Sorted {
        keys: Vec::with_capacity(count * keys.len()),
        width: keys.len(),
        orders: Vec::with_capacity(count),
        rows: (0..count).collect(),
    };
    for at in 0..count {
        let cells = row_of(rows, RowIndex(at), row);
        let Some(Value::Number(number)) = cells.get(usize::from(order)).copied() else {
            return Err(DerivedFault::ShapeMismatch);
        };
        side.orders.push(number.0);
        for column in keys {
            let value = cells
                .get(usize::from(*column))
                .copied()
                .ok_or(DerivedFault::NoSuchColumn { column: *column })?;
            side.keys.push(match value {
                Value::Symbol(_) | Value::Name(_) => match datum(value, texts) {
                    Datum::Text(text) => KeyCell::Text(text.0),
                    _ => return Err(DerivedFault::ShapeMismatch),
                },
                Value::Node(_) | Value::File(_) | Value::Span(_) | Value::Number(_) | Value::Flag(_) => {
                    KeyCell::Cell(value)
                }
            });
        }
    }
    let mut ordered = std::mem::take(&mut side.rows);
    ordered.sort_unstable_by(|one, other| {
        side.keys(*one)
            .cmp(side.keys(*other))
            .then(side.orders[*one].cmp(&side.orders[*other]))
            .then(one.cmp(other))
    });
    if ordered
        .windows(2)
        .any(|pair| side.keys(pair[0]) == side.keys(pair[1]) && side.orders[pair[0]] == side.orders[pair[1]])
    {
        return Err(DerivedFault::DuplicateOrder);
    }
    side.rows = ordered;
    Ok(side)
}

/// Spike (ryo-fmx3): the band semi match: each left row, in order, whose equal-keyed right rows hold one whose order passes
/// every bound, its output (left) columns once.
fn banded(
    ordered: &OrderedParts,
    band: &Band,
    left: InputRows<'_>,
    right: InputRows<'_>,
    repeats: &mut RowSet,
    output: &mut Table,
) -> Result<(), PortableOperatorFault> {
    let number = |cells: &[Value], column: ColumnIndex| match cells.get(usize::from(column)) {
        Some(Value::Number(number)) => Ok(number.0),
        Some(_) => Err(PortableOperatorFault::ShapeMismatch),
        None => Err(PortableOperatorFault::NoSuchColumn { index: column }),
    };
    let width = ordered.right_on.len();
    let count = count_of(right).0;
    let (mut keys, mut orders, mut row) = (Vec::with_capacity(count * width), Vec::with_capacity(count), Vec::new());
    for at in 0..count {
        let cells = row_of(right, RowIndex(at), &mut row);
        for column in &ordered.right_on {
            keys.push(
                *cells
                    .get(usize::from(*column))
                    .ok_or(PortableOperatorFault::NoSuchColumn { index: *column })?,
            );
        }
        orders.push(number(cells, ordered.right_order)?);
    }
    let key_of = |at: usize| keys.get(at * width..(at + 1) * width).unwrap_or_default();
    let mut rows: Vec<usize> = (0..count).collect();
    rows.sort_unstable_by(|one, other| key_of(*one).cmp(key_of(*other)).then(orders[*one].cmp(&orders[*other])));
    let mut wanted: Vec<Value> = Vec::with_capacity(width);
    for at in 0..count_of(left).0 {
        let cells = row_of(left, RowIndex(at), &mut row);
        wanted.clear();
        for column in &ordered.left_on {
            wanted.push(
                *cells
                    .get(usize::from(*column))
                    .ok_or(PortableOperatorFault::NoSuchColumn { index: *column })?,
            );
        }
        // The tightest lower bound: the greatest, an exclusive one over an inclusive one of the same value.
        let mut lower: Option<(i64, bool)> = None;
        for bound in band.bounds.iter().filter(|bound| bound.above) {
            let value = (number(cells, bound.column)?, bound.inclusive);
            lower = Some(match lower {
                Some(held) if held.0 > value.0 || held.0 == value.0 && !held.1 => held,
                _ => value,
            });
        }
        let passes_lower = |order: i64| match lower {
            None => true,
            Some((value, true)) => order >= value,
            Some((value, false)) => order > value,
        };
        let found = rows.partition_point(|&right_at| match key_of(right_at).cmp(&wanted) {
            std::cmp::Ordering::Less => true,
            std::cmp::Ordering::Greater => false,
            std::cmp::Ordering::Equal => !passes_lower(orders[right_at]),
        });
        let Some(&right_at) = rows.get(found) else {
            continue;
        };
        if key_of(right_at) != wanted.as_slice() {
            continue;
        }
        let order = orders[right_at];
        let mut within = true;
        for bound in band.bounds.iter().filter(|bound| !bound.above) {
            let value = number(cells, bound.column)?;
            within &= if bound.inclusive { order <= value } else { order < value };
        }
        if !within {
            continue;
        }
        let start = CellStart(output.cells.len());
        for column in &ordered.output {
            output.cells.push(
                *cells
                    .get(usize::try_from(column.column.0).unwrap_or(usize::MAX))
                    .ok_or(PortableOperatorFault::NoSuchColumn {
                        index: column.column,
                    })?,
            );
        }
        counted(output, start, RepeatedRows::Dropped, repeats);
    }
    Ok(())
}
