//! Differential traversal behavior over an actual prepared equality index.
use super::fixtures::{Observed, Traversal, traverse};
use ryo_relations::domain::relations::types::Integer;
use ryo_relations::domain::relations::types::Value::Number;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use std::borrow::Cow;

/// A row's input: the edges and the walk asked of them.
type Input = Traversal;

/// What the walk observed.
type Value = Observed;

/// A walk's own errors are part of what it observed.
type Failure = std::convert::Infallible;

/// The edge tables the differential row walks: a chain, branches, ties, duplicates with a cycle, a self loop,
/// and no edges.
const DIFFERENTIAL_TABLES: &[&[[i64; 2]]] = &[
    &[[1, 2], [2, 3], [3, 4]],
    &[[1, 2], [1, 3], [2, 4], [3, 5]],
    &[[1, 3], [1, 2], [3, 4], [2, 4]],
    &[[1, 2], [1, 2], [2, 3], [3, 1]],
    &[[1, 1], [1, 2]],
    &[],
];

/// The rows: one per walk.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "nearest_matches_first_admitted_breadth_vertex_for_branches_duplicates_cycles_and_ties",
        ),
        word: Word::Succeeds,
        input: || Traversal::NearestAgainstBreadth(DIFFERENTIAL_TABLES),
        expect: Expect::Value(Observed::Agreement {
            combinations: 90,
            agreeing: 90,
        }),
    },
    Case {
        name: CaseName("reused_walk_nearest_hit_then_no_path_then_full_walk_clears_previous_work"),
        word: Word::Succeeds,
        input: || Traversal::ReusedWalk(&[[1, 2], [1, 3], [2, 4], [3, 5]]),
        expect: Expect::Value(Observed::Reused {
            hit: Cow::Borrowed(&[Number(Integer(3))]),
            hit_error: None,
            visited_at_hit: 2,
            queued_at_hit: true,
            no_path: Cow::Borrowed(&[]),
            no_path_error: None,
            full: Cow::Borrowed(&[
                Number(Integer(2)),
                Number(Integer(3)),
                Number(Integer(4)),
                Number(Integer(5)),
            ]),
        }),
    },
    Case {
        name: CaseName(
            "admission_fault_is_deferred_until_first_reached_vertex_and_stops_further_admission",
        ),
        word: Word::Succeeds,
        input: || Traversal::FailingAdmission(&[[1, 2], [2, 3]]),
        expect: Expect::Value(Observed::Fault {
            error: Some("wrong membership view"),
            asked: Cow::Borrowed(&[Number(Integer(2))]),
            reached: Cow::Borrowed(&[]),
        }),
    },
    Case {
        name: CaseName("missing_destination_cells_are_skipped_as_in_full_traversal"),
        word: Word::Succeeds,
        input: || Traversal::MissingDestination(&[[1, 2], [2, 3]]),
        expect: Expect::Value(Observed::Ragged {
            full: Cow::Borrowed(&[Number(Integer(2))]),
            nearest: Cow::Borrowed(&[]),
            error: None,
        }),
    },
];

/// Runs one row's walk.
fn subject(traversal: Input) -> Result<Value, Failure> {
    Ok(traverse(traversal))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
