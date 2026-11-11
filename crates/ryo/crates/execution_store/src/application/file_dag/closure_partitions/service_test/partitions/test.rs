//! The proofs of the partitioned closure cursor.
use super::super::super::fixtures::working_capacity;
use super::fixtures::{number, ordering_bytes, scope, working_cells};
use crate::cases::{Case, CaseName, Expect, Word, run};
#[cfg(test)]
use ryo_execution_domain::domain::table_model::types::RepresentationInput;
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::service::push_row;
use crate::application::file_dag::closure_partitions::types::PartitionAdvance;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_relations::domain::relations::types::Value as Cell;
use ryo_values::domain::rows::types::Table;
use std::borrow::Cow;

/// The edges `(from, to, key)` a closure is partitioned over by key.
#[derive(Clone, Copy, Debug)]
enum Edges {
    /// The edges 1 -> 2, 2 -> 3 and 1 -> 4 in each of this many keys, interleaved.
    Interleaved(i64),
    /// These edges.
    Keyed(&'static [(i64, i64, i64)]),
    /// The edges 1 -> 2 in key 5 and 3 -> 4 in key 7; the second partition is taken out.
    Taken,
    /// No edges; the cursor's prepared order is then handed a wrong plan.
    Empty,
}

/// What walking interleaved partitions observed.
#[derive(Debug, PartialEq)]
struct Interleaved {
    shapes: usize,
    sources_one_two: usize,
    seen: usize,
    live_cells_bounded: bool,
    reserved_bounded: bool,
    ordering_bounded: bool,
    last: PartitionAdvance,
}

/// What taking the second of two partitions observed.
#[derive(Debug, PartialEq)]
struct Taken {
    input_charged: bool,
    first: PartitionAdvance,
    partition_charged: bool,
    second: PartitionAdvance,
    first_released: bool,
    charge_unchanged: bool,
    taken_key: Cow<'static, [Cell]>,
    cursor_holds_none: bool,
    taken_kept: bool,
    taken_charged: bool,
    taken_released: bool,
    input_charge_left: bool,
    charge_after_input: usize,
}

/// What an empty cursor answered.
#[derive(Debug, PartialEq)]
struct Empty {
    current_before: Option<OperatingError>,
    advance: PartitionAdvance,
    current_after: Option<OperatingError>,
    take: Option<OperatingError>,
    wrong_plan: Option<OperatingError>,
}

/// What the partitions observed.
#[derive(Debug, PartialEq)]
enum Observed {
    Interleaved(Interleaved),
    /// Per partition, in order: its key and its edge cells.
    Partitions(Cow<'static, [(Cow<'static, [Cell]>, Cow<'static, [Cell]>)]>),
    Taken(Taken),
    Empty(Empty),
}

type Input = Edges;
type Value = Observed;
type Failure = OperatingError;

const fn n(value: i64) -> Cell {
    Cell::Number(ryo_relations::domain::relations::types::Integer(value))
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("interleaved_partitions_keep_one_reused_adjacency"),
        word: Word::Succeeds,
        input: || Edges::Interleaved(2048),
        expect: Expect::Value(Observed::Interleaved(Interleaved {
            shapes: 2048,
            sources_one_two: 2048,
            seen: 2048,
            live_cells_bounded: true,
            reserved_bounded: true,
            ordering_bounded: true,
            last: PartitionAdvance::Exhausted,
        })),
    },
    Case {
        name: CaseName("separate_keys_never_share_edges"),
        word: Word::Succeeds,
        input: || Edges::Keyed(&[(1, 2, 5), (9, 10, 7), (2, 3, 5), (10, 11, 7)]),
        expect: Expect::Value(Observed::Partitions(Cow::Borrowed(&[
            (
                Cow::Borrowed(&[n(5)]),
                Cow::Borrowed(&[n(1), n(2), n(2), n(3)]),
            ),
            (
                Cow::Borrowed(&[n(7)]),
                Cow::Borrowed(&[n(9), n(10), n(10), n(11)]),
            ),
        ]))),
    },
    Case {
        name: CaseName("advancing_releases_only_the_previous_partition_lease"),
        word: Word::Succeeds,
        input: || Edges::Taken,
        expect: Expect::Value(Observed::Taken(Taken {
            input_charged: true,
            first: PartitionAdvance::Advanced,
            partition_charged: true,
            second: PartitionAdvance::Advanced,
            first_released: true,
            charge_unchanged: true,
            taken_key: Cow::Borrowed(&[n(7)]),
            cursor_holds_none: true,
            taken_kept: true,
            taken_charged: true,
            taken_released: true,
            input_charge_left: true,
            charge_after_input: 0,
        })),
    },
    Case {
        name: CaseName("empty_order_and_wrong_prepared_order_are_refused_precisely"),
        word: Word::Succeeds,
        input: || Edges::Empty,
        expect: Expect::Value(Observed::Empty(Empty {
            current_before: Some(OperatingError::NotRun),
            advance: PartitionAdvance::Exhausted,
            current_after: Some(OperatingError::NotRun),
            take: Some(OperatingError::NotRun),
            wrong_plan: Some(OperatingError::WrongOperation),
        })),
    },
];

/// Publishes the edges, prepares the partitioned closure over key column 2 and walks it.
fn subject(input: Input) -> Result<Value, Failure> {
    let mut table = Table {
        kinds: vec![CellKind::Number; 3],
        ..Table::default()
    };
    match input {
        Edges::Interleaved(partitions) => {
            for (from, to) in [(1, 2), (2, 3), (1, 4)] {
                for partition in 0..partitions {
                    push_row(&mut table, &[number(from), number(to), number(partition)]);
                }
            }
        }
        Edges::Keyed(edges) => {
            for &(from, to, key) in edges {
                push_row(&mut table, &[number(from), number(to), number(key)]);
            }
        }
        Edges::Taken => {
            for (from, to, key) in [(1, 2, 5), (3, 4, 7)] {
                push_row(&mut table, &[number(from), number(to), number(key)]);
            }
        }
        Edges::Empty => {}
    }
    let scope = scope();
    let publication = scope.computed(table.clone())?;
    let rows = scope.resolve(publication.as_ref(), &|_| Err(OperatingError::NotRead))?;
    let input_charge = scope.derived().publication_ownership_bytes().0;
    let mut cursor = super::super::super::service::prepare(
        &crate::application::file_dag::relations::handle::service::Publication::of(
            publication.clone(),
        ),
        RepresentationInput::from(&rows),
        &[ColumnIndex(2)],
        ColumnIndex(0),
        ColumnIndex(1),
        std::rc::Rc::clone(&scope),
    )?;
    Ok(match input {
        Edges::Interleaved(_) => {
            let mut walked = Interleaved {
                shapes: 0,
                sources_one_two: 0,
                seen: 0,
                live_cells_bounded: true,
                reserved_bounded: true,
                ordering_bounded: true,
                last: PartitionAdvance::Advanced,
            };
            let mut maximum_live = 0;
            let mut maximum_reserved = 0;
            while cursor.advance()? == PartitionAdvance::Advanced {
                let part = cursor.current()?;
                walked.shapes +=
                    usize::from(part.table.count.0 == 3 && part.table.cells.len() == 6 && part.key.len() == 1);
                walked.sources_one_two += usize::from(part.sources == [number(1), number(2)]);
                walked.seen += 1;
                maximum_live = maximum_live.max(working_cells(&cursor));
                maximum_reserved = maximum_reserved.max(working_capacity(&cursor));
            }
            // All owned Value payload, including key and sources.
            walked.live_cells_bounded = maximum_live <= 16;
            walked.reserved_bounded = maximum_reserved < 64;
            // Ordering metadata scales with row IDs, not the number of partition graphs.
            walked.ordering_bounded = ordering_bytes(&cursor) < table.count.0 * 64;
            walked.last = cursor.advance()?;
            Observed::Interleaved(walked)
        }
        Edges::Keyed(_) => {
            let mut observed = Vec::new();
            while cursor.advance()? == PartitionAdvance::Advanced {
                let part = cursor.current()?;
                observed.push((
                    Cow::Owned(part.key.to_vec()),
                    Cow::Owned(part.table.cells.clone()),
                ));
            }
            observed.sort_by_key(|(key, _)| format!("{key:?}"));
            Observed::Partitions(Cow::Owned(observed))
        }
        Edges::Taken => {
            let first = cursor.advance()?;
            let owner = std::rc::Rc::clone(&scope);
            let charge = || owner.derived().publication_ownership_bytes().0;
            let first_backing =
                crate::application::file_dag::derived::owner::fixtures::resources::weak_storage(
                    cursor.current.as_ref().expect("a current partition"),
                );
            let first_charge = charge();
            let second = cursor.advance()?;
            let first_released = first_backing.upgrade().is_none();
            let second_backing =
                crate::application::file_dag::derived::owner::fixtures::resources::weak_storage(
                    cursor.current.as_ref().expect("a current partition"),
                );
            let charge_unchanged = charge() == first_charge;
            let transferred = cursor.take_current()?;
            let ryo_derived::domain::contract::types::DerivedView::ClosureAdjacency(part) =
                transferred.view().expect("a closure view")
            else {
                panic!("closure partition")
            };
            let taken_key = Cow::Owned(part.key.to_vec());
            let cursor_holds_none = cursor.current.is_none();
            drop(cursor);
            let taken_kept = second_backing.upgrade().is_some();
            let taken_charged = charge() > 0;
            drop(transferred);
            let taken_released = second_backing.upgrade().is_none();
            let input_charge_left = charge() == input_charge;
            drop(rows);
            drop(publication);
            Observed::Taken(Taken {
                input_charged: input_charge > 0,
                first,
                partition_charged: first_charge > input_charge,
                second,
                first_released,
                charge_unchanged,
                taken_key,
                cursor_holds_none,
                taken_kept,
                taken_charged,
                taken_released,
                input_charge_left,
                charge_after_input: charge(),
            })
        }
        Edges::Empty => {
            let current_before = cursor.current().err();
            let advance = cursor.advance()?;
            let current_after = cursor.current().err();
            let take = cursor.take_current().err();
            let wrong_plan = std::sync::Arc::new((*cursor.partition_plan).clone());
            Observed::Empty(Empty {
                current_before,
                advance,
                current_after,
                take,
                wrong_plan: super::super::super::service::from_prepared(
                    cursor.proof.clone(),
                    wrong_plan,
                    std::sync::Arc::clone(&cursor.closure_plan),
                    cursor.ordered.clone(),
                    std::rc::Rc::clone(&cursor.owner),
                )
                .err(),
            })
        }
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
