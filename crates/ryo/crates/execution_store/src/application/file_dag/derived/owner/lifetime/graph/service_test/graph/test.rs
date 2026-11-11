//! Real owner handles remain valid/accounted across clear and cannot poison refill.
use super::fixtures::{Completed, Declined, Held, Lifetime, Observed, Shared, observe};
use crate::application::file_dag::derived::owner::graphs::types::handles::types::GraphSharing;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::stored_graph::types::Orientation;
use ryo_relations::domain::relations::types::{Integer, Value as Cell};
use std::borrow::Cow;

type Input = Lifetime;

type Value = Observed;

type Failure = String;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "zero_optional_budget_shares_live_graph_and_rebuilds_only_after_last_opaque_consumer",
        ),
        word: Word::Succeeds,
        input: || Lifetime::NoOptionalBudget,
        expect: Expect::Value(Observed::Shared(Shared {
            sharing: GraphSharing::Local,
            live: Held {
                backings: 1,
                builds: 1,
                optional_bytes: 0,
            },
            one_backing: true,
            reverse: Orientation::Reverse,
            unique_bytes_unchanged: true,
            shared: Held {
                backings: 1,
                builds: 1,
                optional_bytes: 0,
            },
            reverse_answered: false,
            reverse_reached: Cow::Borrowed(&[Cell::Number(Integer(1)), Cell::Number(Integer(2))]),
            after_first_dropped: 1,
            expired: Held {
                backings: 0,
                builds: 1,
                optional_bytes: 0,
            },
            refilled: Held {
                backings: 1,
                builds: 2,
                optional_bytes: 0,
            },
            after_refill_dropped: 0,
        })),
    },
    Case {
        name: CaseName("old_completed_answer_owns_exact_context_through_clear_and_changed_facts"),
        word: Word::Succeeds,
        input: || Lifetime::CompletedAnswer,
        expect: Expect::Value(Observed::Completed(Completed {
            nearest: Cell::Number(Integer(3)),
            answer: Cow::Borrowed(&[Cell::Number(Integer(2)), Cell::Number(Integer(3))]),
            alias_same_data: true,
            live_backings: 1,
            unique_is_required_plus_optional_less_overlap: true,
            cleared_optional_bytes: 0,
            cleared_backings: 1,
            fresh: Cow::Borrowed(&[Cell::Number(Integer(90))]),
            changed_builds: 1,
            changed_backings: 2,
            answer_after_change: Cow::Borrowed(&[
                Cell::Number(Integer(2)),
                Cell::Number(Integer(3)),
            ]),
            old_domain_refused: Some(OperatingError::NotRead),
            alone: Cow::Borrowed(&[Cell::Number(Integer(2)), Cell::Number(Integer(3))]),
            alone_backings: 1,
            alive_before_drop: true,
            alive_after_drop: false,
            dropped_backings: 0,
            dropped_unique_bytes: 0,
        })),
    },
    Case {
        name: CaseName("unavailable_canonical_source_declines_without_retaining_duplicate_graphs"),
        word: Word::Succeeds,
        input: || Lifetime::WithoutCanonicalIdentity([(false, 1024), (true, 0)]),
        expect: Expect::Value(Observed::Declined([
            Declined {
                acquired: [false, false],
                graphs: 0,
                held: Held {
                    backings: 0,
                    builds: 0,
                    optional_bytes: 0,
                },
            },
            Declined {
                acquired: [false, false],
                graphs: 0,
                held: Held {
                    backings: 0,
                    builds: 0,
                    optional_bytes: 0,
                },
            },
        ])),
    },
];

/// Runs one row through its fixtures.
fn subject(input: Input) -> Result<Value, Failure> {
    observe(input)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
