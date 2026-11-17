//! The rule a refusal names is the rule that built the refused node: the first rule a step of which is the node, so a node
//! two rules share is the first one's, and a node only the engine keeps (a held frontier, no rule's emit reading it) is still
//! the rule's whose step it is.
use crate::application::file_dag::file_pass::service::pass_program;
use crate::application::file_dag::file_pass::types::PassSource;
use crate::outbound::operators::fixtures::fake::OperatorsFake;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input as StepInput, StepId, StepScope};
use ryo_values::domain::plans::types::{Emit, Operation, Plan, PlanStep};
use ryo_values::domain::runs::types::RuleId;
use std::borrow::Cow;
use std::sync::Arc;

/// The node the growth check refuses, by its number in the program.
type Input = u32;
/// The id of the rule the refusal names.
type Value = Cow<'static, str>;
type Failure = ();

/// A per-file step that keeps every row of `relation`.
fn kept_rows_of(relation: Relation) -> PlanStep {
    PlanStep {
        scope: StepScope::PerFile,
        operation: Operation::Select {
            from: StepInput::Relation(relation),
            comparisons: Vec::new(),
            kept: Vec::new(),
        },
    }
}

/// A plan of `steps` that emits its first step and holds `frontier`.
fn plan(steps: Vec<PlanStep>, frontier: Vec<StepInput>) -> Arc<Plan> {
    Arc::new(Plan {
        observation_inputs: Vec::new(),
        parameters: Vec::new(),
        comparison_origins: Vec::new(),
        observations: Vec::new(),
        frontier,
        emit: Emit {
            step: StepId(0),
            subject: ColumnIndex(0),
            key: Vec::new(),
            headings: Vec::new(),
        },
        steps,
    })
}

/// Rule `first` emits its select of `Defines` (node 0). Rule `second` emits the same select, which is node 0 again, and holds
/// a select of `Entry` as its frontier (node 1), which no emit reads. The fake operators refuse the plan at `node`; answers
/// the rule the refusal names.
fn subject(node: Input) -> Result<Value, Failure> {
    let source = PassSource {
        plans: vec![
            plan(vec![kept_rows_of(Relation::Defines)], Vec::new()),
            plan(
                vec![
                    kept_rows_of(Relation::Defines),
                    kept_rows_of(Relation::Entry),
                ],
                vec![StepInput::Step(StepId(1))],
            ),
        ],
        bindings: vec![Vec::new(), Vec::new()],
        operators: Arc::new(OperatorsFake {
            refuses_node: Some(node),
            ..OperatorsFake::default()
        }),
        hosted: Vec::new(),
        ids: vec![RuleId("first".into()), RuleId("second".into())],
    };
    match pass_program(&source) {
        Err(OperatingError::Superlinear { rule, .. }) => Ok(Cow::Owned(rule.0.to_string())),
        _ => Err(()),
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_node_two_rules_share_is_named_by_the_first"),
        word: Word::Succeeds,
        input: || 0,
        expect: Expect::Value(Cow::Borrowed("first")),
    },
    Case {
        name: CaseName("a_node_only_a_held_frontier_keeps_is_named_by_the_rule_that_built_it"),
        word: Word::Succeeds,
        input: || 1,
        expect: Expect::Value(Cow::Borrowed("second")),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
