//! The `band_join` item of the `kernels` collection: over the rows of the partition the runner hands it, the right rows ordered
//! once as a permutation by their key and the band's first column, then each left row, in order, meets the right rows of its key
//! inside the band, every one or the nearest (ties kept), ascending; one gather writes the answer's columns.

#[cfg(test)]
use super::super::interface::{KernelsItem, Reach};
use super::super::scan_facts::item::{cell_of, count_of, kinds_of};
use crate::domain::comparing::collections::comparisons::interface::ComparisonsItem as _;
use crate::domain::comparing::collections::comparisons::ordering::item::OrderingItem;
use crate::domain::contract::types::{PortableOperatorFault, PredicateOutcome};
use crate::domain::equality_join::service::test_of;
use crate::domain::equality_join::types::BandParts;
use crate::domain::growth::types::{Growth, Operator};
use crate::domain::physical_plan::types::InputRows;
use crate::domain::physical_plan::types::lowering::value::BandJoinNode;
#[cfg(test)]
use crate::domain::physical_plan::types::lowering::value::Lowering;
use crate::domain::table_texts::service::datum;
use crate::domain::table_texts::types::{Datum, TextView, Texts};
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::band_joins::types::{BandAnswer, BandTest};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Side, WholeNumber};
use ryo_values::domain::rows::types::{RowCount, RowIndex, Table};
use std::cmp::Ordering;

/// `band_join`, one item of the `kernels` collection.
#[cfg(test)]
pub(crate) struct BandJoinItem;

#[cfg(test)]
impl KernelsItem for BandJoinItem {
    fn reached_by(&self, lowering: &Lowering) -> Reach {
        if matches!(lowering, Lowering::BandJoin(_)) {
            Reach::Runs
        } else {
            Reach::Skips
        }
    }
}

impl Operator for BandJoinNode {
    fn growth(&self) -> Growth {
        Growth::Banded(crate::domain::equality_join::service::band_fan(&self.band))
    }
}

/// Where a cell stands in a band's order: numbers by value, then texts byte by byte; anything else orders with nothing, as
/// the language's order comparisons say.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Rank<'a> {
    /// A number.
    Number(WholeNumber),
    /// A text.
    Text(TextView<'a>),
    /// Neither: never less or greater than anything.
    Unordered,
}

/// Which class of rank a cell has: two cells compare only within one class.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum RankClass {
    /// Numbers.
    Number,
    /// Texts.
    Text,
    /// Neither.
    Unordered,
}

/// One pair the band join answers: a left row and a right row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Paired {
    /// The left row.
    left: RowIndex,
    /// The right row.
    right: RowIndex,
}

/// A right row in the band's order: its index and the rank of its first-edge cell.
#[derive(Clone, Copy, Debug)]
struct Placed<'a> {
    /// The row.
    row: RowIndex,
    /// The rank of its cell in the band's first edge.
    rank: Rank<'a>,
}

/// Which side of a split the right rows whose rank equals the left one's fall on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ties {
    /// Below the split: counted among the rows below.
    Below,
    /// Above the split.
    Above,
}

/// The cell of `rows` at `row` and `column`: the kernel's one accessor.
fn cell(
    rows: InputRows<'_>,
    row: RowIndex,
    column: ColumnIndex,
) -> Result<Value, PortableOperatorFault> {
    cell_of(rows, row, column).ok_or(PortableOperatorFault::NoSuchColumn { index: column })
}

/// The rank of a cell.
fn rank<'a>(value: Value, texts: &Texts<'a>) -> Rank<'a> {
    match datum(value, texts) {
        Datum::Number(number) => Rank::Number(number),
        Datum::Text(text) => Rank::Text(text),
        Datum::Unlocated | Datum::Flag(_) | Datum::Numbers(_) | Datum::Texts(_) => Rank::Unordered,
    }
}

/// The class of a rank.
const fn class(rank: Rank<'_>) -> RankClass {
    match rank {
        Rank::Number(_) => RankClass::Number,
        Rank::Text(_) => RankClass::Text,
        Rank::Unordered => RankClass::Unordered,
    }
}

/// The key cells of `row` of `rows` in `on`, into `into`.
fn key_of(
    rows: InputRows<'_>,
    row: RowIndex,
    on: &[ColumnIndex],
    into: &mut Vec<Value>,
) -> Result<(), PortableOperatorFault> {
    into.clear();
    for column in on {
        into.push(cell(rows, row, *column)?);
    }
    Ok(())
}

/// The pairs of `parts` over `left` and `right`, in left row order, each left row's right rows ascending; then their answer
/// columns into `output`, gathered once.
///
/// # Errors
///
/// `NoSuchColumn` for a key, band or output column past its side.
pub(crate) fn banded(
    parts: &BandParts,
    left: InputRows<'_>,
    right: InputRows<'_>,
    texts: Texts<'_>,
    output: &mut Table,
) -> Result<(), PortableOperatorFault> {
    output.kinds = parts
        .output
        .iter()
        .map(|column| {
            let side = match column.side {
                Side::Left => left,
                Side::Right => right,
            };
            kinds_of(side)
                .get(usize::try_from(column.column.0).unwrap_or(usize::MAX))
                .copied()
                .ok_or(PortableOperatorFault::NoSuchColumn {
                    index: column.column,
                })
        })
        .collect::<Result<_, _>>()?;
    let Some(first) = parts.band.first() else {
        return Err(PortableOperatorFault::WrongOperation);
    };
    // The right rows' keys and first-edge ranks, read once; the order is a permutation of their indexes.
    let count = count_of(right).0;
    let mut keys = Vec::with_capacity(count);
    let mut order = Vec::with_capacity(count);
    let mut key = Vec::new();
    for at in 0..count {
        key_of(right, RowIndex(at), &parts.right_on, &mut key)?;
        keys.push(key.clone());
        order.push(Placed {
            row: RowIndex(at),
            rank: rank(cell(right, RowIndex(at), first.right)?, &texts),
        });
    }
    order.sort_by(|a, b| {
        keys.get(a.row.0)
            .cmp(&keys.get(b.row.0))
            .then_with(|| a.rank.cmp(&b.rank))
            .then_with(|| a.row.cmp(&b.row))
    });
    let mut pairs = Vec::new();
    let mut found = Vec::new();
    for row in 0..count_of(left).0 {
        let row = RowIndex(row);
        key_of(left, row, &parts.left_on, &mut key)?;
        let wanted = rank(cell(left, row, first.left)?, &texts);
        if class(wanted) == RankClass::Unordered {
            continue;
        }
        // The right rows of the key, and of them those whose rank is of the left one's class.
        let start = order.partition_point(|each| keys.get(each.row.0).is_some_and(|k| *k < key));
        let end = order.partition_point(|each| keys.get(each.row.0).is_some_and(|k| *k <= key));
        let run = order.get(start..end).unwrap_or_default();
        let low = run.partition_point(|each| class(each.rank) < class(wanted));
        let high = run.partition_point(|each| class(each.rank) <= class(wanted));
        let segment = run.get(low..high).unwrap_or_default();
        let below = |ties: Ties| {
            segment.partition_point(|each| match each.rank.cmp(&wanted) {
                Ordering::Less => true,
                Ordering::Equal => ties == Ties::Below,
                Ordering::Greater => false,
            })
        };
        let band = match first.test {
            BandTest::Below => segment.get(..below(Ties::Above)),
            BandTest::AtOrBelow => segment.get(..below(Ties::Below)),
            BandTest::Above => segment.get(below(Ties::Below)..),
            BandTest::AtOrAbove => segment.get(below(Ties::Above)..),
        }
        .unwrap_or_default();
        let kept = match (parts.answer, first.test) {
            (BandAnswer::Every, _) => band,
            (BandAnswer::Nearest, BandTest::Below | BandTest::AtOrBelow) => {
                let nearest = band.last().map(|each| each.rank);
                band.get(band.partition_point(|each| Some(each.rank) < nearest)..)
                    .unwrap_or_default()
            }
            (BandAnswer::Nearest, BandTest::Above | BandTest::AtOrAbove) => {
                let nearest = band.first().map(|each| each.rank);
                band.get(..band.partition_point(|each| Some(each.rank) <= nearest))
                    .unwrap_or_default()
            }
        };
        found.clear();
        for each in kept {
            let other = each.row;
            let mut inside = true;
            for edge in parts.band.get(1..).unwrap_or_default() {
                let theirs = datum(cell(right, other, edge.right)?, &texts);
                let ours = datum(cell(left, row, edge.left)?, &texts);
                let held = OrderingItem
                    .holds(test_of(edge.test), &theirs, &ours)
                    .map_err(|_wrong| PortableOperatorFault::WrongTest)?;
                inside &= held == PredicateOutcome::Holds;
            }
            if inside {
                found.push(each.row);
            }
        }
        found.sort_unstable();
        pairs.extend(found.iter().map(|each| Paired {
            left: row,
            right: *each,
        }));
    }
    for pair in &pairs {
        for column in &parts.output {
            output.cells.push(match column.side {
                Side::Left => cell(left, pair.left, column.column)?,
                Side::Right => cell(right, pair.right, column.column)?,
            });
        }
    }
    output.count = RowCount(pairs.len());
    Ok(())
}
