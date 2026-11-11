//! Typed populations, dense bag order, partial provenance and fused reach semantics.
use super::fixtures::{Observed, Population, Reach, Refusal, exercise};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_relations::domain::relations::types::{
    FileId, Integer, NodeId, Offset, Span, Value as Cell,
};
use std::borrow::Cow;

type Input = Population;
type Value = Observed;
type Failure = Refusal;

/// A function's span, from `start` to `end`.
const fn stretch(start: u32, end: u32) -> Cell {
    Cell::Span(Span {
        start: Offset(start),
        end: Offset(end),
    })
}

/// A number cell.
const fn number(value: i64) -> Cell {
    Cell::Number(Integer(value))
}

/// The composite join's answer on every route: 96 rows, the first place meeting node 1 and the
/// third place meeting node 3 twice, every place in order.
const COMPOSITE: Observed = Observed::Bag {
    rows: 96,
    first: Cow::Borrowed(&[
        stretch(0, 1),
        number(10),
        Cell::Node(NodeId(1)),
        stretch(2, 3),
        number(30),
        Cell::Node(NodeId(3)),
        stretch(2, 3),
        number(31),
        Cell::Node(NodeId(3)),
    ]),
    places_in_order: true,
    affected: Cow::Borrowed(&[]),
};

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "composite_typed_keys_keep_duplicate_bag_order_under_stored_and_zero_budget_routes",
        ),
        word: Word::Succeeds,
        input: || Population::Composite {
            stored: true,
            optional_limit: Some(0),
        },
        expect: Expect::Value(COMPOSITE),
    },
    Case {
        name: CaseName("composite_typed_keys_keep_bag_order_computed"),
        word: Word::Succeeds,
        input: || Population::Composite {
            stored: false,
            optional_limit: None,
        },
        expect: Expect::Value(COMPOSITE),
    },
    Case {
        name: CaseName("composite_typed_keys_keep_bag_order_computed_under_a_zero_budget"),
        word: Word::Succeeds,
        input: || Population::Composite {
            stored: false,
            optional_limit: Some(0),
        },
        expect: Expect::Value(COMPOSITE),
    },
    Case {
        name: CaseName("composite_typed_keys_keep_bag_order_stored"),
        word: Word::Succeeds,
        input: || Population::Composite {
            stored: true,
            optional_limit: None,
        },
        expect: Expect::Value(COMPOSITE),
    },
    Case {
        name: CaseName(
            "dense_pairs_keep_all_65552_rows_and_first_last_order_without_optional_retention",
        ),
        word: Word::Succeeds,
        input: || Population::Dense,
        expect: Expect::Value(Observed::Dense {
            rows: 65552,
            first: Cow::Borrowed(&[stretch(0, 1), number(0)]),
            last: Cow::Borrowed(&[stretch(4096, 4097), number(15)]),
            places_in_order: true,
        }),
    },
    Case {
        name: CaseName("partial_empty_population_keeps_exact_empty_rows_and_partial_provenance"),
        word: Word::Succeeds,
        input: || Population::PartialEmpty,
        expect: Expect::Value(Observed::Bag {
            rows: 0,
            first: Cow::Borrowed(&[]),
            places_in_order: true,
            affected: Cow::Borrowed(&[Some(FileId(0))]),
        }),
    },
    Case {
        name: CaseName(
            "reach_join_keeps_lazy_acquisition_exact_cycle_order_duplicates_and_refusals",
        ),
        word: Word::Succeeds,
        input: || Population::Reach(Reach::Reached),
        expect: Expect::Value(Observed::Reached {
            delivered: Cow::Borrowed(&[
                number(1),
                number(2),
                number(3),
                number(9),
                number(1),
                number(2),
                number(3),
                number(9),
            ]),
            rows: 8,
            builds: 1,
            derived_builds: 2,
            right_admitted: true,
            queue: 0,
        }),
    },
    Case {
        name: CaseName(
            "a_reach_join_over_an_empty_primary_builds_nothing_but_admits_the_right_source",
        ),
        word: Word::Succeeds,
        input: || Population::Reach(Reach::EmptyPrimary),
        expect: Expect::Value(Observed::Reached {
            delivered: Cow::Borrowed(&[]),
            rows: 0,
            builds: 0,
            derived_builds: 0,
            right_admitted: true,
            queue: 0,
        }),
    },
    Case {
        name: CaseName("a_reach_join_keeps_the_right_source_refusal"),
        word: Word::Refuses,
        input: || Population::Reach(Reach::RightRefused),
        expect: Expect::Error(Refusal {
            error: OperatingError::NotRead,
            delivered: 0,
        }),
    },
    Case {
        name: CaseName("a_reach_join_keeps_the_left_source_refusal"),
        word: Word::Refuses,
        input: || Population::Reach(Reach::LeftRefused),
        expect: Expect::Error(Refusal {
            error: OperatingError::NotRun,
            delivered: 0,
        }),
    },
    Case {
        name: CaseName("a_reach_join_keeps_the_sink_refusal"),
        word: Word::Refuses,
        input: || Population::Reach(Reach::SinkRefused),
        expect: Expect::Error(Refusal {
            error: OperatingError::NotRun,
            delivered: 0,
        }),
    },
];

/// Joins the population.
fn subject(population: Input) -> Result<Value, Failure> {
    exercise(population)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
