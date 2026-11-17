//! What the plan reads of each operation kind's contract: one operation of each kind, its inputs fact relations, gives the
//! rows its `presence` declares and the inputs its `failure` declares fallible, so a changed arm is seen.
use super::super::super::fixtures::drafts;
use super::super::super::service::plan_of;
use super::super::super::types::NodeRequirement;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operation_schema::fixtures::operations;
use ryo_operators::domain::physical_plan::types::PlanInput;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::StepScope;
use ryo_values::domain::plans::service::{inputs_of, kind_of, named};
use ryo_values::domain::query_operators::types::OperationKind;
use std::borrow::Cow;

/// The fact relation each input reads.
const FACT: Relation = Relation::Function;

type Input = OperationKind;
/// The node's requirement: its rows and the bits of its fallible inputs, or `Live`.
type Value = Cow<'static, str>;
type Failure = OperatingError;

/// What the plan reads of one operation of `kind`, each of its inputs a fact relation.
fn subject(kind: Input) -> Result<Value, Failure> {
    let operation = operations()
        .into_iter()
        .find(|operation| kind_of(operation) == kind)
        .ok_or(OperatingError::NotRun)?;
    let reads = named(&inputs_of(&operation)).len();
    let nodes = vec![(
        operation,
        vec![PlanInput::Relation(FACT); reads],
        StepScope::PerFile,
    )];
    let lists: Vec<Vec<PlanInput>> = nodes.iter().map(|node| node.1.clone()).collect();
    let plan = plan_of(&drafts(&nodes), &lists, &Default::default());
    Ok(Cow::Owned(match plan.nodes.first() {
        Some(NodeRequirement::Inputs { rows, fallible, .. }) => {
            format!("{rows:?} fallible {}", fallible.0)
        }
        Some(requirement) => format!("{requirement:?}"),
        None => String::new(),
    }))
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_select_declares_its_contract"),
        word: Word::Succeeds,
        input: || OperationKind::Select,
        expect: Expect::Value(Cow::Borrowed("AllOf(InputMask(1)) fallible 0")),
    },
    Case {
        name: CaseName("a_project_declares_its_contract"),
        word: Word::Succeeds,
        input: || OperationKind::Project,
        expect: Expect::Value(Cow::Borrowed("AllOf(InputMask(1)) fallible 0")),
    },
    Case {
        name: CaseName("an_extend_declares_its_contract"),
        word: Word::Succeeds,
        input: || OperationKind::Extend,
        expect: Expect::Value(Cow::Borrowed("AllOf(InputMask(1)) fallible 0")),
    },
    Case {
        name: CaseName("a_join_declares_its_contract"),
        word: Word::Succeeds,
        input: || OperationKind::Join,
        expect: Expect::Value(Cow::Borrowed("AllOf(InputMask(3)) fallible 0")),
    },
    Case {
        name: CaseName("a_union_declares_its_contract"),
        word: Word::Succeeds,
        input: || OperationKind::Union,
        expect: Expect::Value(Cow::Borrowed("AnyOf(InputMask(3)) fallible 0")),
    },
    Case {
        name: CaseName("an_intersection_declares_its_contract"),
        word: Word::Succeeds,
        input: || OperationKind::Intersection,
        expect: Expect::Value(Cow::Borrowed("AllOf(InputMask(3)) fallible 0")),
    },
    Case {
        name: CaseName("a_difference_declares_its_contract"),
        word: Word::Succeeds,
        input: || OperationKind::Difference,
        expect: Expect::Value(Cow::Borrowed("AllOf(InputMask(1)) fallible 0")),
    },
    Case {
        name: CaseName("an_antijoin_declares_its_contract"),
        word: Word::Succeeds,
        input: || OperationKind::Antijoin,
        expect: Expect::Value(Cow::Borrowed("AllOf(InputMask(1)) fallible 3")),
    },
    Case {
        name: CaseName("a_group_count_declares_its_contract"),
        word: Word::Succeeds,
        input: || OperationKind::GroupCount,
        expect: Expect::Value(Cow::Borrowed("AllOf(InputMask(1)) fallible 0")),
    },
    Case {
        name: CaseName("a_group_digest_declares_its_contract"),
        word: Word::Succeeds,
        input: || OperationKind::GroupDigest,
        expect: Expect::Value(Cow::Borrowed("AllOf(InputMask(1)) fallible 0")),
    },
    Case {
        name: CaseName("a_capture_declares_its_contract"),
        word: Word::Succeeds,
        input: || OperationKind::Capture,
        expect: Expect::Value(Cow::Borrowed("AllOf(InputMask(1)) fallible 0")),
    },
    Case {
        name: CaseName("a_rank_declares_its_contract"),
        word: Word::Succeeds,
        input: || OperationKind::Rank,
        expect: Expect::Value(Cow::Borrowed("AllOf(InputMask(1)) fallible 1")),
    },
    Case {
        name: CaseName("a_replace_declares_its_contract"),
        word: Word::Succeeds,
        input: || OperationKind::Replace,
        expect: Expect::Value(Cow::Borrowed("AllOf(InputMask(1)) fallible 1")),
    },
    Case {
        name: CaseName("an_ordered_match_declares_its_contract"),
        word: Word::Succeeds,
        input: || OperationKind::OrderedMatch,
        expect: Expect::Value(Cow::Borrowed("AllOf(InputMask(3)) fallible 3")),
    },
    Case {
        name: CaseName("a_words_declares_its_contract"),
        word: Word::Succeeds,
        input: || OperationKind::Words,
        expect: Expect::Value(Cow::Borrowed("AllOf(InputMask(1)) fallible 0")),
    },
    Case {
        name: CaseName("an_axis_declares_its_contract"),
        word: Word::Succeeds,
        input: || OperationKind::Axis,
        expect: Expect::Value(Cow::Borrowed("AllOf(InputMask(1)) fallible 1")),
    },
    Case {
        name: CaseName("a_closure_declares_its_contract"),
        word: Word::Succeeds,
        input: || OperationKind::Closure,
        expect: Expect::Value(Cow::Borrowed("AllOf(InputMask(1)) fallible 0")),
    },
    Case {
        name: CaseName("an_ordered_dfs_declares_its_contract"),
        word: Word::Succeeds,
        input: || OperationKind::OrderedDfs,
        expect: Expect::Value(Cow::Borrowed("AllOf(InputMask(1)) fallible 3")),
    },
    Case {
        name: CaseName("a_worklist_declares_its_contract"),
        word: Word::Succeeds,
        input: || OperationKind::Worklist,
        expect: Expect::Value(Cow::Borrowed("AllOf(InputMask(1)) fallible 7")),
    },
    Case {
        name: CaseName("a_cycles_declares_its_contract"),
        word: Word::Succeeds,
        input: || OperationKind::Cycles,
        expect: Expect::Value(Cow::Borrowed("AllOf(InputMask(1)) fallible 0")),
    },
    Case {
        name: CaseName("a_rows_declares_its_contract"),
        word: Word::Succeeds,
        input: || OperationKind::Rows,
        expect: Expect::Value(Cow::Borrowed("Always fallible 0")),
    },
    Case {
        name: CaseName("a_hosted_declares_its_contract"),
        word: Word::Succeeds,
        input: || OperationKind::Hosted,
        expect: Expect::Value(Cow::Borrowed("Live")),
    },
    Case {
        name: CaseName("a_span_between_declares_its_contract"),
        word: Word::Succeeds,
        input: || OperationKind::SpanBetween,
        expect: Expect::Value(Cow::Borrowed("AllOf(InputMask(1)) fallible 1")),
    },
    Case {
        name: CaseName("a_source_matches_declares_its_contract"),
        word: Word::Succeeds,
        input: || OperationKind::SourceMatches,
        expect: Expect::Value(Cow::Borrowed("AllOf(InputMask(1)) fallible 1")),
    },
    Case {
        name: CaseName("a_render_syntax_declares_its_contract"),
        word: Word::Succeeds,
        input: || OperationKind::RenderSyntax,
        expect: Expect::Value(Cow::Borrowed("AllOf(InputMask(1)) fallible 1")),
    },
    Case {
        name: CaseName("a_fits_rendered_declares_its_contract"),
        word: Word::Succeeds,
        input: || OperationKind::FitsRendered,
        expect: Expect::Value(Cow::Borrowed("AllOf(InputMask(1)) fallible 1")),
    },
    Case {
        name: CaseName("a_span_bounds_declares_its_contract"),
        word: Word::Succeeds,
        input: || OperationKind::SpanBounds,
        expect: Expect::Value(Cow::Borrowed("AllOf(InputMask(1)) fallible 1")),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
