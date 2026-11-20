//! The authored fixtures of the kinds this engine never runs state what the one meaning of each kind answers: no engine runs
//! them here, so each fixture's rows are held against `ryo_operators`' `ordered_groups` and `follow` over the fixture's inputs.

use super::super::expectations::Expected;
use super::{as_of, follow, group_bound, group_ordinal};
use ryo_operators::domain::follow::service::fixed_points;
use ryo_operators::domain::follow::types::{Entity, GroupPointers, Pointer};
use ryo_operators::domain::ordered_groups::service::{extreme, nearest, places};
use ryo_operators::domain::ordered_groups::types::GroupOrders;
use ryo_relations::domain::relations::types::{Integer, Value as Cell};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Side};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::rows::service::row_in;
use ryo_values::domain::rows::types::{RowIndex, Table};
use std::borrow::Cow;
use std::collections::BTreeMap;

/// A fixture: its operation, its inputs and the rows it states.
type Input = fn() -> (Operation, Vec<Table>, Vec<Vec<Expected>>);
/// The rows the meaning answers over the fixture's inputs, sorted, as text.
type Value = Cow<'static, str>;
/// The rows the fixture states, when they are not the rows the meaning answers.
type Failure = Cow<'static, str>;
/// A row of numbers.
type Row = Vec<i64>;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_as_of_fixture_states_the_nearest_right_row_before_each_left_row"),
        word: Word::Succeeds,
        input: || as_of,
        expect: Expect::Value(Cow::Borrowed("[[3, 2]]")),
    },
    Case {
        name: CaseName("the_group_bound_fixture_states_the_greatest_row_of_each_key"),
        word: Word::Succeeds,
        input: || group_bound,
        expect: Expect::Value(Cow::Borrowed("[[1, 5], [2, 4]]")),
    },
    Case {
        name: CaseName("the_group_ordinal_fixture_states_each_rows_place_from_zero"),
        word: Word::Succeeds,
        input: || group_ordinal,
        expect: Expect::Value(Cow::Borrowed("[[1, 3, 0], [1, 5, 1]]")),
    },
    Case {
        name: CaseName("the_follow_fixture_states_the_fixed_points_and_no_dead_end"),
        word: Word::Succeeds,
        input: || follow,
        expect: Expect::Value(Cow::Borrowed("[[1, 2], [2, 2]]")),
    },
];

/// The rows of a table of numbers.
fn rows_of(table: Option<&Table>) -> Vec<Row> {
    let Some(table) = table else {
        return Vec::new();
    };
    (0..table.count.0)
        .filter_map(|at| row_in(table, RowIndex(at)))
        .map(|row| {
            row.iter()
                .filter_map(|cell| {
                    if let Cell::Number(Integer(number)) = cell {
                        Some(*number)
                    } else {
                        None
                    }
                })
                .collect()
        })
        .collect()
}

/// The cells of `row` at `columns`.
fn picked(row: &[i64], columns: &[ColumnIndex]) -> Row {
    columns
        .iter()
        .filter_map(|column| {
            row.get(usize::try_from(column.0).unwrap_or(usize::MAX))
                .copied()
        })
        .collect()
}

/// The rows of `rows` by the cells of their `keys`, each group ascending by the cell at `order`.
fn grouped(rows: Vec<Row>, keys: &[ColumnIndex], order: ColumnIndex) -> BTreeMap<Row, Vec<Row>> {
    let mut groups: BTreeMap<Row, Vec<Row>> = BTreeMap::new();
    for row in rows {
        groups.entry(picked(&row, keys)).or_default().push(row);
    }
    for group in groups.values_mut() {
        group.sort_by_key(|row| picked(row, &[order]));
    }
    groups
}

/// The orders of a group's rows.
fn orders_of(group: &[Row], order: ColumnIndex) -> Vec<Integer> {
    group
        .iter()
        .flat_map(|row| picked(row, &[order]))
        .map(Integer)
        .collect()
}

/// The rows the meaning of `operation` answers over `inputs`.
fn meant(operation: &Operation, inputs: &[Table]) -> Vec<Row> {
    let first = rows_of(inputs.first());
    let mut answer: Vec<Row> = Vec::new();
    if let Operation::AsOf {
        left_on,
        right_on,
        left_order,
        right_order,
        bound,
        output,
        ..
    } = operation
    {
        {
            let events = grouped(rows_of(inputs.get(1)), right_on, *right_order);
            for probe in first {
                // A probe whose key no event has pairs with none.
                let Some(group) = events.get(&picked(&probe, left_on)) else {
                    continue;
                };
                let orders = orders_of(group, *right_order);
                let at = picked(&probe, &[*left_order]).first().copied().map(Integer);
                let run = at.map_or(0..0, |at| nearest(GroupOrders(&orders), at, *bound).0);
                for event in group.get(run).into_iter().flatten() {
                    answer.push(
                        output
                            .iter()
                            .flat_map(|named| match named.side {
                                Side::Left => picked(&probe, &[named.column]),
                                Side::Right => picked(event, &[named.column]),
                            })
                            .collect(),
                    );
                }
            }
        }
    } else if let Operation::GroupBound {
        keys, order, end, ..
    } = operation
    {
        {
            for group in grouped(first, keys, *order).into_values() {
                let run = extreme(GroupOrders(&orders_of(&group, *order)), *end).0;
                answer.extend(group.get(run).into_iter().flatten().cloned());
            }
        }
    } else if let Operation::GroupOrdinal { keys, order, .. } = operation {
        {
            let mut distinct = first;
            distinct.sort();
            distinct.dedup();
            for group in grouped(distinct, keys, *order).into_values() {
                let mut placed = Vec::new();
                places(GroupOrders(&orders_of(&group, *order)), &mut placed);
                answer.extend(group.into_iter().zip(placed).map(|(mut row, place)| {
                    row.push(i64::from(place.0));
                    row
                }));
            }
        }
    } else if let Operation::Follow {
        keys,
        source,
        destination,
        ..
    } = operation
    {
        {
            for (key, group) in grouped(first, keys, *source) {
                let entity = |row: &Row, column| {
                    picked(row, &[column])
                        .first()
                        .and_then(|cell| u32::try_from(*cell).ok())
                        .map(Entity)
                };
                let edges: Vec<Pointer> = group
                    .iter()
                    .filter_map(|row| {
                        Some(Pointer {
                            source: entity(row, *source)?,
                            destination: entity(row, *destination)?,
                        })
                    })
                    .collect();
                let mut ends = Vec::new();
                fixed_points(GroupPointers(&edges), &mut ends);
                answer.extend(ends.into_iter().map(|end| {
                    let mut row = key.clone();
                    row.extend([i64::from(end.source.0), i64::from(end.destination.0)]);
                    row
                }));
            }
        }
    }
    answer.sort();
    answer
}

fn subject(fixture: Input) -> Result<Value, Failure> {
    let (operation, inputs, expected) = fixture();
    let mut stated: Vec<Row> = expected
        .iter()
        .map(|row| {
            row.iter()
                .filter_map(|cell| {
                    if let Expected::Cell(Cell::Number(Integer(number))) = cell {
                        Some(*number)
                    } else {
                        None
                    }
                })
                .collect()
        })
        .collect();
    stated.sort();
    let meant = meant(&operation, &inputs);
    if stated == meant {
        Ok(Cow::Owned(format!("{meant:?}")))
    } else {
        Err(Cow::Owned(format!(
            "the fixture states {stated:?}, the meaning answers {meant:?}"
        )))
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
