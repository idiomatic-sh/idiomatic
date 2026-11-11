//! A production route cannot bypass an injected semantic refusal, and fused programs refuse alike;
//! over operators that answer, the same route publishes.
use super::super::fixtures::chain::{Chain, execute_chain, execute_fixture};
use super::super::fixtures::describe;
use super::fixtures::RefusingSemantics;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_values::domain::query_operators::types::OperationKind;
use std::sync::Arc;
/// What is run against operators that refuse every call.
#[derive(Clone, Copy, Debug)]
enum Asked {
    /// The kind's fixture through the production route.
    Route(OperationKind),
    /// The kind's fixture through the production route, over operators that answer.
    Answered(OperationKind),
    /// The kind's fixture as a fused and as a held chain.
    Chains(OperationKind),
}

/// What a run answered when it did not refuse: the rows it published, or that the fused and
/// held chains disagreed.
#[derive(Debug, PartialEq)]
enum Answered {
    Published(usize),
    Disagreed { fused: String, held: String },
}

type Input = Asked;
type Value = Answered;
type Failure = OperatingError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_production_route_publishes_over_operators_that_answer"),
        word: Word::Succeeds,
        input: || Asked::Answered(OperationKind::Project),
        expect: Expect::Value(Answered::Published(2)),
    },
    Case {
        name: CaseName("production_route_cannot_bypass_an_injected_semantic_refusal"),
        word: Word::Refuses,
        input: || Asked::Route(OperationKind::Project),
        expect: Expect::Error(OperatingError::WrongOperation),
    },
    Case {
        name: CaseName("all26_fused_and_single_instruction_programs_refuse_with_the_same_error"),
        word: Word::Refuses,
        input: || Asked::Chains(OperationKind::Replace),
        expect: Expect::Error(OperatingError::WrongOperation),
    },
    Case {
        name: CaseName("a_fused_and_a_held_ordered_match_refuse_with_the_same_error"),
        word: Word::Refuses,
        input: || Asked::Chains(OperationKind::OrderedMatch),
        expect: Expect::Error(OperatingError::WrongOperation),
    },
    Case {
        name: CaseName("a_fused_and_a_held_select_refuse_with_the_same_error"),
        word: Word::Refuses,
        input: || Asked::Chains(OperationKind::Select),
        expect: Expect::Error(OperatingError::WrongOperation),
    },
    Case {
        name: CaseName("a_fused_and_a_held_project_refuse_with_the_same_error"),
        word: Word::Refuses,
        input: || Asked::Chains(OperationKind::Project),
        expect: Expect::Error(OperatingError::WrongOperation),
    },
    Case {
        name: CaseName("a_fused_and_a_held_extend_refuse_with_the_same_error"),
        word: Word::Refuses,
        input: || Asked::Chains(OperationKind::Extend),
        expect: Expect::Error(OperatingError::WrongOperation),
    },
    Case {
        name: CaseName("a_fused_and_a_held_join_refuse_with_the_same_error"),
        word: Word::Refuses,
        input: || Asked::Chains(OperationKind::Join),
        expect: Expect::Error(OperatingError::WrongOperation),
    },
    Case {
        name: CaseName("a_fused_and_a_held_union_refuse_with_the_same_error"),
        word: Word::Refuses,
        input: || Asked::Chains(OperationKind::Union),
        expect: Expect::Error(OperatingError::WrongOperation),
    },
    Case {
        name: CaseName("a_fused_and_a_held_intersection_refuse_with_the_same_error"),
        word: Word::Refuses,
        input: || Asked::Chains(OperationKind::Intersection),
        expect: Expect::Error(OperatingError::WrongOperation),
    },
    Case {
        name: CaseName("a_fused_and_a_held_difference_refuse_with_the_same_error"),
        word: Word::Refuses,
        input: || Asked::Chains(OperationKind::Difference),
        expect: Expect::Error(OperatingError::WrongOperation),
    },
    Case {
        name: CaseName("a_fused_and_a_held_antijoin_refuse_with_the_same_error"),
        word: Word::Refuses,
        input: || Asked::Chains(OperationKind::Antijoin),
        expect: Expect::Error(OperatingError::WrongOperation),
    },
    Case {
        name: CaseName("a_fused_and_a_held_group_count_refuse_with_the_same_error"),
        word: Word::Refuses,
        input: || Asked::Chains(OperationKind::GroupCount),
        expect: Expect::Error(OperatingError::WrongOperation),
    },
    Case {
        name: CaseName("a_fused_and_a_held_group_digest_refuse_with_the_same_error"),
        word: Word::Refuses,
        input: || Asked::Chains(OperationKind::GroupDigest),
        expect: Expect::Error(OperatingError::WrongOperation),
    },
    Case {
        name: CaseName("a_fused_and_a_held_capture_refuse_with_the_same_error"),
        word: Word::Refuses,
        input: || Asked::Chains(OperationKind::Capture),
        expect: Expect::Error(OperatingError::WrongOperation),
    },
    Case {
        name: CaseName("a_fused_and_a_held_rank_refuse_with_the_same_error"),
        word: Word::Refuses,
        input: || Asked::Chains(OperationKind::Rank),
        expect: Expect::Error(OperatingError::WrongOperation),
    },
    Case {
        name: CaseName("a_fused_and_a_held_words_refuse_with_the_same_error"),
        word: Word::Refuses,
        input: || Asked::Chains(OperationKind::Words),
        expect: Expect::Error(OperatingError::WrongOperation),
    },
    Case {
        name: CaseName("a_fused_and_a_held_axis_refuse_with_the_same_error"),
        word: Word::Refuses,
        input: || Asked::Chains(OperationKind::Axis),
        expect: Expect::Error(OperatingError::WrongOperation),
    },
    Case {
        name: CaseName("a_fused_and_a_held_closure_refuse_with_the_same_error"),
        word: Word::Refuses,
        input: || Asked::Chains(OperationKind::Closure),
        expect: Expect::Error(OperatingError::WrongOperation),
    },
    Case {
        name: CaseName("a_fused_and_a_held_cycles_refuse_with_the_same_error"),
        word: Word::Refuses,
        input: || Asked::Chains(OperationKind::Cycles),
        expect: Expect::Error(OperatingError::WrongOperation),
    },
    Case {
        name: CaseName("a_fused_and_a_held_rows_refuse_with_the_same_error"),
        word: Word::Refuses,
        input: || Asked::Chains(OperationKind::Rows),
        expect: Expect::Error(OperatingError::WrongOperation),
    },
    Case {
        name: CaseName("a_fused_and_a_held_hosted_refuse_with_the_same_error"),
        word: Word::Refuses,
        input: || Asked::Chains(OperationKind::Hosted),
        expect: Expect::Error(OperatingError::WrongOperation),
    },
    Case {
        name: CaseName("a_fused_and_a_held_span_between_refuse_with_the_same_error"),
        word: Word::Refuses,
        input: || Asked::Chains(OperationKind::SpanBetween),
        expect: Expect::Error(OperatingError::WrongOperation),
    },
    Case {
        name: CaseName("a_fused_and_a_held_source_matches_refuse_with_the_same_error"),
        word: Word::Refuses,
        input: || Asked::Chains(OperationKind::SourceMatches),
        expect: Expect::Error(OperatingError::WrongOperation),
    },
    Case {
        name: CaseName("a_fused_and_a_held_render_syntax_refuse_with_the_same_error"),
        word: Word::Refuses,
        input: || Asked::Chains(OperationKind::RenderSyntax),
        expect: Expect::Error(OperatingError::WrongOperation),
    },
    Case {
        name: CaseName("a_fused_and_a_held_fits_rendered_refuse_with_the_same_error"),
        word: Word::Refuses,
        input: || Asked::Chains(OperationKind::FitsRendered),
        expect: Expect::Error(OperatingError::WrongOperation),
    },
    Case {
        name: CaseName("a_fused_and_a_held_span_bounds_refuse_with_the_same_error"),
        word: Word::Refuses,
        input: || Asked::Chains(OperationKind::SpanBounds),
        expect: Expect::Error(OperatingError::WrongOperation),
    },
    Case {
        name: CaseName("a_fused_and_a_held_ordered_dfs_refuse_with_the_same_error"),
        word: Word::Refuses,
        input: || Asked::Chains(OperationKind::OrderedDfs),
        expect: Expect::Error(OperatingError::WrongOperation),
    },
];

/// Runs against operators that refuse every call (or, as a control, answer); chains refuse with
/// their shared error.
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Asked::Route(kind) => execute_fixture(kind, false, Arc::new(RefusingSemantics))
            .map(|(table, _)| Answered::Published(table.count.0)),
        Asked::Answered(kind) => execute_fixture(
            kind,
            false,
            Arc::new(crate::application::file_dag::fixtures::operators()),
        )
        .map(|(table, _)| Answered::Published(table.count.0)),
        Asked::Chains(kind) => {
            let fused = execute_chain(kind, Chain::Fused, Arc::new(RefusingSemantics));
            let single = execute_chain(kind, Chain::Held, Arc::new(RefusingSemantics));
            if describe(&fused) != describe(&single) {
                return Ok(Answered::Disagreed {
                    fused: describe(&fused),
                    held: describe(&single),
                });
            }
            fused
                .outcome
                .map(|(table, _)| Answered::Published(table.count.0))
        }
    }
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
