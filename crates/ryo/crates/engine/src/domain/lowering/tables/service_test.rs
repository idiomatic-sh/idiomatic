//! The cases of the `tables` part: a rule is admitted or refused step by step, naming the step that cannot be evaluated or
//! the count that cannot be written; the kinds of operation the lowering evaluates; and the flattening of a plan visits
//! each step once (a counted case at a depth and at twice it). Every expected text is what a run printed.

use super::arms::service::rule_flattened;
use super::service::admitted;
use super::types::RuleFlattening;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::lowering::definitions::types::Drafts;
use crate::domain::lowering::fixtures::growth::growth_plan;
use crate::domain::lowering::fixtures::{digests_of, named};
use crate::domain::lowering::nodes::service::stored;
use crate::domain::lowering::nodes::types::AcceptedPlace;
use crate::domain::lowering::types::{LoadRefusal, PastWidth, PlanNode, SelectedRule};
use ryo_values::domain::descriptions::types::{KindVocabulary, LanguageName};
use ryo_values::domain::plan_parts::types::{Input as PlanInput, Side, StepId, StepScope};
use ryo_values::domain::plans::types::{Operation, PlanStep};
use ryo_values::domain::query_bindings::types::RuleBindings;
use ryo_values::domain::query_operators::types::OperationKind;
use ryo_values::domain::query_plans::types::{NodeDigests, Plan};
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
use std::borrow::Cow;

/// What a row asks of the tables part.
#[derive(Debug)]
enum Asked {
    /// A plan admitted: the columns of its emitted step.
    Admitted(Plan),
    /// The kinds of operation whose growth plan is admitted: the ones the lowering evaluates.
    Kinds,
    /// A chain of self-joins of this depth and of twice it, flattened: the work of each.
    Chain(u32),
}

type Input = Asked;
/// What was asked, said.
type Value = Cow<'static, str>;
type Failure = LoadRefusal;

const fn refused(
    name: &'static str,
    input: fn() -> Input,
    refusal: LoadRefusal,
) -> Case<Input, Value, Failure> {
    Case {
        name: CaseName(name),
        word: Word::Refuses,
        input,
        expect: Expect::Error(refusal),
    }
}

const fn said(
    name: &'static str,
    input: fn() -> Input,
    value: &'static str,
) -> Case<Input, Value, Failure> {
    Case {
        name: CaseName(name),
        word: Word::Succeeds,
        input,
        expect: Expect::Value(Cow::Borrowed(value)),
    }
}

/// The one tree rule, its first step moved after the files.
fn after_the_files() -> Plan {
    let mut plan = named("one tree rule");
    if let Some(step) = plan.steps.first_mut() {
        step.scope = StepScope::CrossFile;
    }
    plan
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    said(
        "a_rule_of_the_lowered_operations_is_admitted_with_its_emitted_columns",
        || Asked::Admitted(named("as-of latest before")),
        "3 columns",
    ),
    said(
        "the_kinds_of_operation_the_lowering_evaluates_are_these",
        || Asked::Kinds,
        "[Replace, OrderedMatch, AsOf, Select, Project, Extend, Join, Union, Antijoin, GroupCount, GroupBound, GroupOrdinal, Capture, Axis, Closure, Follow, Rows, SpanBetween, SourceMatches, FitsRendered, SpanBounds]",
    ),
    said(
        "a_plan_walk_visits_each_step_once_a_chain_of_self_joins_twice_as_deep_is_twice_the_work",
        || Asked::Chain(40),
        "linear: work Some(207) at depth 40, Some(407) at twice it",
    ),
    refused(
        "a_rule_over_the_hosts_rows_is_refused_at_that_step",
        || Asked::Admitted(named("over the host's rows")),
        LoadRefusal::Unlowerable(PlanNode(0)),
    ),
    refused(
        "a_step_of_65_536_columns_is_refused_naming_the_columns_of_a_table",
        || Asked::Admitted(named("a step of 65,536 columns")),
        LoadRefusal::Budget(PastWidth::TableColumns),
    ),
    refused(
        "an_operation_not_lowered_yet_is_refused_at_its_step",
        || Asked::Admitted(growth_plan(OperationKind::Cycles)),
        LoadRefusal::Unlowerable(PlanNode(2)),
    ),
    said(
        "a_rule_whose_emit_reaches_per_file_steps_alone_is_admitted_though_its_plan_hands_rows_over",
        || {
            Asked::Admitted(named(
                "a rule over the tree whose words another rule observes",
            ))
        },
        "5 columns",
    ),
    refused(
        "a_rule_whose_emit_reaches_a_step_after_the_files_is_refused_at_that_step",
        || Asked::Admitted(named("a rule that emits after the files")),
        LoadRefusal::Unlowerable(PlanNode(1)),
    ),
    refused(
        "a_step_that_runs_after_the_files_is_refused",
        || Asked::Admitted(after_the_files()),
        LoadRefusal::Unlowerable(PlanNode(0)),
    ),
];

fn rule<'a>(plan: &'a Plan, digests: &'a NodeDigests) -> SelectedRule<'a> {
    SelectedRule {
        rule: SelectedRuleOrdinal(0),
        plan,
        bindings: RuleBindings(&[]),
        digests,
    }
}

/// A plan admitted, with its bindings none and its digests the fixture's.
fn admitted_plan(plan: &Plan) -> Result<usize, LoadRefusal> {
    let digests = digests_of(plan, RuleBindings(&[]));
    let nodes = admitted(rule(plan, &digests))?;
    let emitted = nodes.shapes.get(usize::from(plan.emit.step));
    Ok(emitted
        .and_then(Option::as_ref)
        .map_or(0, |shape| shape.0.len()))
}

fn subject(asked: Input) -> Result<Value, Failure> {
    Ok(Cow::Owned(match asked {
        Asked::Admitted(plan) => format!("{} columns", admitted_plan(&plan)?),
        Asked::Kinds => {
            let lowered = OperationKind::ALL.iter().copied();
            let kinds: Vec<OperationKind> = lowered
                .filter_map(|kind| match admitted_plan(&growth_plan(kind)) {
                    Ok(columns) => (columns > 0).then_some(kind),
                    Err(LoadRefusal::Unlowerable(_) | LoadRefusal::Budget(_)) => None,
                })
                .collect();
            format!("{kinds:?}")
        }
        Asked::Chain(depth) => {
            let (short, long) = (work(depth), work(depth.saturating_mul(2)));
            let linear = match (short, long) {
                (Some(short), Some(long)) if long <= short.saturating_mul(2).saturating_add(16) => {
                    "linear"
                }
                (Some(_) | None, Some(_) | None) => "NOT linear",
            };
            format!("{linear}: work {short:?} at depth {depth}, {long:?} at twice it")
        }
    }))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

/// A chain of `depth` joins, each of the step before it with itself on its first column: the shape whose widths the old
/// expansion recomputed at every use.
fn chain(depth: u32) -> Plan {
    let mut plan = named("one tree rule");
    let of = |side| ryo_values::domain::plan_parts::types::JoinColumn {
        side,
        column: ryo_values::domain::plan_parts::types::ColumnIndex(0),
    };
    for step in 0..depth {
        let read = PlanInput::Step(StepId(step));
        plan.steps.push(PlanStep {
            operation: Operation::Join {
                left: read,
                right: read,
                left_on: vec![ryo_values::domain::plan_parts::types::ColumnIndex(0)],
                right_on: vec![ryo_values::domain::plan_parts::types::ColumnIndex(0)],
                output: vec![of(Side::Left), of(Side::Right)],
                filter: Vec::new(),
                view: Vec::new(),
                reach: None,
            },
            scope: StepScope::PerFile,
        });
    }
    plan.emit.step = StepId(depth);
    plan
}

/// The work of flattening a chain: the variables and the definitions it makes.
fn work(depth: u32) -> Option<u64> {
    let plan = chain(depth);
    let digests = digests_of(&plan, RuleBindings(&[]));
    let rules = [admitted(rule(&plan, &digests)).ok()?];
    let store = stored(&rules);
    let kinds = KindVocabulary {
        language: LanguageName("walking".into()),
        kinds: Vec::new(),
        fields: Vec::new(),
        fixed: Vec::new(),
    };
    let mut drafts = Drafts::default();
    let mut state = RuleFlattening {
        rules: &rules,
        store: &store,
        kinds: &kinds,
        place: AcceptedPlace(0),
        flat: Vec::new(),
    };
    let flattened = rule_flattened(&mut state, &mut drafts).ok()?;
    assert_eq!(
        flattened.branches.len(),
        1,
        "a chain of joins of one branch is one branch"
    );
    let made = u64::try_from(drafts.definitions.len()).ok()?;
    Some(u64::from(drafts.variables.0) + made)
}
