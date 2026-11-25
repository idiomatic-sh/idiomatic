//! The authored rows and expectations of the kinds this engine never runs (`EXPANDED`): what each means, stated over small
//! inputs. This engine registers the kinds and is prepared from their expansion; the proofs that run a fixture skip them, and
//! this file's own cases hold each fixture's rows equal to what the kind's one meaning answers.
use super::expectations::{Expected, cells};
use super::values::{FROM, RIGHT, matrix, n};
use ryo_values::domain::plan_orders::types::{GroupEnd, OrderBound};
use ryo_values::domain::plan_parts::types::{ColumnIndex, JoinColumn, Side};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::rows::types::Table;

/// An as-of by key and order: the left row at 3 pairs with the right row latest before it, at 2; the one at 1 has none.
#[must_use]
pub fn as_of() -> (Operation, Vec<Table>, Vec<Vec<Expected>>) {
    (
        Operation::AsOf {
            left: FROM,
            right: RIGHT,
            left_on: vec![ColumnIndex(0)],
            right_on: vec![ColumnIndex(0)],
            left_order: ColumnIndex(1),
            right_order: ColumnIndex(1),
            bound: OrderBound::Before,
            output: vec![
                JoinColumn {
                    side: Side::Left,
                    column: ColumnIndex(1),
                },
                JoinColumn {
                    side: Side::Right,
                    column: ColumnIndex(1),
                },
            ],
        },
        vec![matrix(&[&[1, 3], &[1, 1]]), matrix(&[&[1, 4], &[1, 2]])],
        cells(vec![vec![n(3), n(2)]]),
    )
}

/// A group bound: of each key's rows, the one of the greatest order.
#[must_use]
pub fn group_bound() -> (Operation, Vec<Table>, Vec<Vec<Expected>>) {
    (
        Operation::GroupBound {
            from: FROM,
            keys: vec![ColumnIndex(0)],
            order: ColumnIndex(1),
            end: GroupEnd::Greatest,
        },
        vec![matrix(&[&[1, 3], &[1, 5], &[2, 4]])],
        cells(vec![vec![n(1), n(5)], vec![n(2), n(4)]]),
    )
}

/// A group ordinal: each row of a key with its place by order, from 0.
#[must_use]
pub fn group_ordinal() -> (Operation, Vec<Table>, Vec<Vec<Expected>>) {
    (
        Operation::GroupOrdinal {
            from: FROM,
            keys: vec![ColumnIndex(0)],
            order: ColumnIndex(1),
        },
        vec![matrix(&[&[1, 3], &[1, 5]])],
        cells(vec![vec![n(1), n(3), n(0)], vec![n(1), n(5), n(1)]]),
    )
}

/// A follow: 1 points to 2, which points to itself; 3 points to 4, which points nowhere.
#[must_use]
pub fn follow() -> (Operation, Vec<Table>, Vec<Vec<Expected>>) {
    (
        Operation::Follow {
            edges: FROM,
            keys: vec![],
            source: ColumnIndex(0),
            destination: ColumnIndex(1),
        },
        vec![matrix(&[&[1, 2], &[2, 2], &[3, 4]])],
        cells(vec![vec![n(1), n(2)], vec![n(2), n(2)]]),
    )
}

