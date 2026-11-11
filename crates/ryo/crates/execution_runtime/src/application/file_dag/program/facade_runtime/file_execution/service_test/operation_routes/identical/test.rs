//! Fused and single-instruction programs publish identical rows in identical order.
use super::super::super::super::service::*;
use super::super::super::super::*;
use super::super::fixtures::chain::{Chain, execute_chain};
use super::super::fixtures::describe;
use super::fixtures::engine;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::executable_program::fusion::types::{Barrier, Fusibility};
use ryo_values::domain::query_operators::types::OperationKind;
/// What a fused and a held chain over one kind decided and published.
#[derive(Debug, PartialEq)]
struct Chains {
    /// Whether the fused chain inserted a projection to continue.
    continues: bool,
    /// Whether every fused projection fuses into its sole-reader producer.
    fused: bool,
    /// Whether a second reader keeps every held producer's publication.
    held: bool,
    /// The rows the fused chain published.
    rows: usize,
    /// Whether the held chain published the same rows in the same order.
    identical: bool,
}

/// Both chains publish `rows` rows alike.
const fn identical(rows: usize) -> Chains {
    Chains {
        continues: true,
        fused: true,
        held: true,
        rows,
        identical: true,
    }
}

type Input = OperationKind;
type Value = Chains;
type Failure = String;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "all26_fused_and_single_instruction_programs_publish_identical_rows_in_identical_order",
        ),
        word: Word::Succeeds,
        input: || OperationKind::Replace,
        expect: Expect::Value(identical(0)),
    },
    Case {
        name: CaseName("a_fused_and_a_held_ordered_match_publish_identical_rows"),
        word: Word::Succeeds,
        input: || OperationKind::OrderedMatch,
        expect: Expect::Value(identical(0)),
    },
    Case {
        name: CaseName("a_fused_and_a_held_select_publish_identical_rows"),
        word: Word::Succeeds,
        input: || OperationKind::Select,
        expect: Expect::Value(identical(0)),
    },
    Case {
        name: CaseName("a_fused_and_a_held_project_publish_identical_rows"),
        word: Word::Succeeds,
        input: || OperationKind::Project,
        expect: Expect::Value(identical(0)),
    },
    Case {
        name: CaseName("a_fused_and_a_held_extend_publish_identical_rows"),
        word: Word::Succeeds,
        input: || OperationKind::Extend,
        expect: Expect::Value(identical(0)),
    },
    Case {
        name: CaseName("a_fused_and_a_held_join_publish_identical_rows"),
        word: Word::Succeeds,
        input: || OperationKind::Join,
        expect: Expect::Value(identical(0)),
    },
    Case {
        name: CaseName("a_fused_and_a_held_union_publish_identical_rows"),
        word: Word::Succeeds,
        input: || OperationKind::Union,
        expect: Expect::Value(identical(0)),
    },
    Case {
        name: CaseName("a_fused_and_a_held_intersection_publish_identical_rows"),
        word: Word::Succeeds,
        input: || OperationKind::Intersection,
        expect: Expect::Value(identical(0)),
    },
    Case {
        name: CaseName("a_fused_and_a_held_difference_publish_identical_rows"),
        word: Word::Succeeds,
        input: || OperationKind::Difference,
        expect: Expect::Value(identical(0)),
    },
    Case {
        name: CaseName("a_fused_and_a_held_antijoin_publish_identical_rows"),
        word: Word::Succeeds,
        input: || OperationKind::Antijoin,
        expect: Expect::Value(identical(0)),
    },
    Case {
        name: CaseName("a_fused_and_a_held_group_count_publish_identical_rows"),
        word: Word::Succeeds,
        input: || OperationKind::GroupCount,
        expect: Expect::Value(identical(0)),
    },
    Case {
        name: CaseName("a_fused_and_a_held_group_digest_publish_identical_rows"),
        word: Word::Succeeds,
        input: || OperationKind::GroupDigest,
        expect: Expect::Value(identical(0)),
    },
    Case {
        name: CaseName("a_fused_and_a_held_capture_publish_identical_rows"),
        word: Word::Succeeds,
        input: || OperationKind::Capture,
        expect: Expect::Value(identical(0)),
    },
    Case {
        name: CaseName("a_fused_and_a_held_rank_publish_identical_rows"),
        word: Word::Succeeds,
        input: || OperationKind::Rank,
        expect: Expect::Value(identical(0)),
    },
    Case {
        name: CaseName("a_fused_and_a_held_words_publish_identical_rows"),
        word: Word::Succeeds,
        input: || OperationKind::Words,
        expect: Expect::Value(identical(0)),
    },
    Case {
        name: CaseName("a_fused_and_a_held_axis_publish_identical_rows"),
        word: Word::Succeeds,
        input: || OperationKind::Axis,
        expect: Expect::Value(identical(0)),
    },
    Case {
        name: CaseName("a_fused_and_a_held_closure_publish_identical_rows"),
        word: Word::Succeeds,
        input: || OperationKind::Closure,
        expect: Expect::Value(identical(0)),
    },
    Case {
        name: CaseName("a_fused_and_a_held_cycles_publish_identical_rows"),
        word: Word::Succeeds,
        input: || OperationKind::Cycles,
        expect: Expect::Value(identical(0)),
    },
    Case {
        name: CaseName("a_fused_and_a_held_rows_publish_identical_rows"),
        word: Word::Succeeds,
        input: || OperationKind::Rows,
        expect: Expect::Value(identical(0)),
    },
    Case {
        name: CaseName("a_fused_and_a_held_hosted_publish_identical_rows"),
        word: Word::Succeeds,
        input: || OperationKind::Hosted,
        expect: Expect::Value(identical(0)),
    },
    Case {
        name: CaseName("a_fused_and_a_held_span_between_publish_identical_rows"),
        word: Word::Succeeds,
        input: || OperationKind::SpanBetween,
        expect: Expect::Value(identical(0)),
    },
    Case {
        name: CaseName("a_fused_and_a_held_source_matches_publish_identical_rows"),
        word: Word::Succeeds,
        input: || OperationKind::SourceMatches,
        expect: Expect::Value(identical(0)),
    },
    Case {
        name: CaseName("a_fused_and_a_held_render_syntax_publish_identical_rows"),
        word: Word::Succeeds,
        input: || OperationKind::RenderSyntax,
        expect: Expect::Value(identical(0)),
    },
    Case {
        name: CaseName("a_fused_and_a_held_fits_rendered_publish_identical_rows"),
        word: Word::Succeeds,
        input: || OperationKind::FitsRendered,
        expect: Expect::Value(identical(0)),
    },
    Case {
        name: CaseName("a_fused_and_a_held_span_bounds_publish_identical_rows"),
        word: Word::Succeeds,
        input: || OperationKind::SpanBounds,
        expect: Expect::Value(identical(0)),
    },
    Case {
        name: CaseName("a_fused_and_a_held_ordered_dfs_publish_identical_rows"),
        word: Word::Succeeds,
        input: || OperationKind::OrderedDfs,
        expect: Expect::Value(identical(0)),
    },
];

/// Runs the kind's fixture as a fused and as a held chain.
fn subject(kind: Input) -> Result<Value, Failure> {
    let fused = execute_chain(kind, Chain::Fused, engine());
    let single = execute_chain(kind, Chain::Held, engine());
    let rows = match &fused.outcome {
        Ok((table, _)) => table.count.0,
        Err(_) => return Err(describe(&fused)),
    };
    Ok(Chains {
        continues: !fused.continuations.is_empty(),
        fused: fused
            .continuations
            .iter()
            .all(|class| *class == Some(Fusibility::Fusable)),
        held: single
            .continuations
            .iter()
            .all(|class| *class == Some(Fusibility::Barrier(Barrier::SharedConsumer))),
        rows,
        identical: describe(&fused) == describe(&single),
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
