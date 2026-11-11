//! Boundary values for the actual borrowed graph preparation and traversal APIs.
use super::fixtures::{Graph, Observed, retained, walk};
use crate::domain::operating::errors::OperatingError;
use crate::domain::table_reach::types::OrderedVisit;
use ryo_derived::domain::fault::types::DerivedFault;
use ryo_relations::domain::relations::types::Integer;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::rows::types::RowIndex;
use std::borrow::Cow;

/// A row's input: the graph prepared and walked.
type Input = Graph;

/// What preparing and walking the graph observed.
type Value = Observed;

/// The derived preparation's refusal.
type Failure = DerivedFault;

/// A visit of an edge row at an ordinal.
const fn visit(edge: usize, ordinal: i64) -> OrderedVisit {
    OrderedVisit {
        edge: RowIndex(edge),
        ordinal: Integer(ordinal),
    }
}

/// The visits from `a`.
const FROM_A: &[OrderedVisit] = &[visit(0, 0), visit(1, 1), visit(2, 2)];

/// The visits from `b`.
const FROM_B: &[OrderedVisit] = &[visit(1, 0), visit(2, 1), visit(0, 2)];

/// The rows: one per graph.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("ordinal_overflow_follows_the_last_representable_visit"),
        word: Word::Succeeds,
        input: || Graph::LastOrdinal,
        expect: Expect::Value(Observed::LastOrdinal {
            first: Ok(Some(visit(0, i64::MAX))),
            destination_expanded_at_yield: false,
            next: Err(OperatingError::DfsOrdinalOverflow),
        }),
    },
    Case {
        name: CaseName("nonnumeric_order_cell_is_refused_by_actual_derived_preparation"),
        word: Word::Refuses,
        input: || Graph::FlagOrder,
        expect: Expect::Error(DerivedFault::ShapeMismatch),
    },
    Case {
        name: CaseName("equal_retained_hashes_use_exact_text_and_preserve_original_cells"),
        word: Word::Succeeds,
        input: || Graph::RetainedNames,
        expect: Expect::Value(Observed::RetainedNames {
            same_text_shares_key: true,
            equal_hashes_share_key: false,
            roots_preserved: true,
            edges_preserved: true,
            edge_names_kept: true,
            third_root: retained(1),
            from_a: Cow::Borrowed(FROM_A),
            from_b: Cow::Borrowed(FROM_B),
            expanded: 6,
            examined: 6,
        }),
    },
];

/// Prepares and walks one row's graph.
fn subject(graph: Input) -> Result<Value, Failure> {
    walk(graph)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
