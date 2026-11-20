//! The cases of the `tables` part: a rule is admitted or refused step by step, naming the step that cannot be evaluated or
//! the count that cannot be written; and the flattening of a plan visits each step once.

use super::arms::service::rule_flattened;
use super::service::admitted;
use super::types::RuleFlattening;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::lowering::definitions::types::Drafts;
use crate::domain::lowering::fixtures::growth::growth_plan;
use crate::domain::lowering::fixtures::named;
use crate::domain::lowering::nodes::service::stored;
use crate::domain::lowering::types::{LoadRefusal, PastWidth, PlanNode, SelectedRule};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::descriptions::types::{KindVocabulary, LanguageName};
use ryo_values::domain::plan_parts::types::{Input as PlanInput, Side, StepId, StepScope};
use ryo_values::domain::plans::types::{Operation, PlanStep};
use ryo_values::domain::query_bindings::types::RuleBindings;
use ryo_values::domain::query_operators::types::OperationKind;
use ryo_values::domain::query_plans::types::Plan;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;

type Input = Plan;
/// The columns of the rule's emitted step.
type Value = usize;
type Failure = LoadRefusal;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_rule_of_the_lowered_operations_is_admitted_with_its_emitted_columns"),
        word: Word::Succeeds,
        input: || named("as-of latest before"),
        expect: Expect::Value(3),
    },
    Case {
        name: CaseName("a_rule_over_the_hosts_rows_is_refused_at_that_step"),
        word: Word::Refuses,
        input: || named("over the host's rows"),
        expect: Expect::Error(LoadRefusal::Unlowerable(PlanNode(0))),
    },
    Case {
        name: CaseName("a_step_of_65_536_columns_is_refused_naming_the_columns_of_a_table"),
        word: Word::Refuses,
        input: || named("a step of 65,536 columns"),
        expect: Expect::Error(LoadRefusal::Budget(PastWidth::TableColumns)),
    },
    Case {
        name: CaseName("an_operation_not_lowered_yet_is_refused_at_its_step"),
        word: Word::Refuses,
        input: || named("follow fixed points"),
        expect: Expect::Error(LoadRefusal::Unlowerable(PlanNode(4))),
    },
    Case {
        name: CaseName("a_step_that_runs_after_the_files_is_refused"),
        word: Word::Refuses,
        input: || {
            let mut plan = named("one tree rule");
            if let Some(step) = plan.steps.first_mut() {
                step.scope = StepScope::CrossFile;
            }
            plan
        },
        expect: Expect::Error(LoadRefusal::Unlowerable(PlanNode(0))),
    },
];

fn rule(plan: &Plan) -> SelectedRule<'_> {
    SelectedRule {
        rule: SelectedRuleOrdinal(0),
        plan,
        bindings: RuleBindings(&[]),
    }
}

fn subject(plan: Input) -> Result<Value, Failure> {
    let nodes = admitted(rule(&plan))?;
    let emitted = nodes.shapes.get(usize::from(plan.emit.step));
    Ok(emitted.and_then(Option::as_ref).map_or(0, |shape| shape.0.len()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

/// The kinds of operation a rule is admitted with: the ones the lowering evaluates. Each other kind is refused, by name.
#[test]
fn the_kinds_of_operation_lowered_are_these() {
    let lowered: Vec<OperationKind> = OperationKind::ALL
        .iter()
        .copied()
        .filter(|kind| admitted(rule(&growth_plan(*kind))).is_ok())
        .collect();
    println!("{lowered:?}");
    assert_eq!(lowered, Vec::<OperationKind>::new());
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
    let rules = [admitted(rule(&plan)).ok()?];
    let store = stored(&rules);
    let kinds = KindVocabulary {
        language: LanguageName("walking".into()),
        kinds: Vec::new(),
    };
    let mut drafts = Drafts::default();
    let mut state = RuleFlattening {
        rules: &rules,
        store: &store,
        kinds: &kinds,
        place: 0,
        flat: Vec::new(),
    };
    let flattened = rule_flattened(&mut state, &mut drafts).ok()?;
    assert_eq!(flattened.branches.len(), 1, "a chain of joins of one branch is one branch");
    let made = u64::try_from(drafts.definitions.len()).ok()?;
    Some(u64::from(drafts.variables) + made)
}

/// A plan walk visits each step once: the flattening of a chain of self-joins of depth n and of depth 2n makes work in
/// proportion to the depth, never to the paths through it (2 to the depth).
#[test]
fn the_flattening_of_a_chain_of_self_joins_is_linear_in_its_depth() {
    let (short, long) = (work(40), work(80));
    println!("work at depth 40: {short:?}; at depth 80: {long:?}");
    let (Some(short), Some(long)) = (short, long) else {
        panic!("a chain was not flattened");
    };
    assert!(long <= short * 2 + 16, "{long} is more than twice {short}");
    let _unused = Relation::SyntaxTree;
}
