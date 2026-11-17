//! The `PhysicalPlan` value's cases: a plan runs one node per node of its program.
use super::value::PhysicalPlan;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::physical_plan::service::plan;
use crate::domain::physical_plan::types::logical_program::value::LogicalProgram;
use crate::domain::physical_plan::types::rule_ordinal::value::RuleOrdinal;
use crate::domain::physical_plan::types::{LogicalNode, NodeBindings, PlanInput, PlanInputList};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{Input as Source, StepScope};
use ryo_values::domain::plans::types::Operation;

/// How many selects the program chains, each over the one before: none, one or two.
type Input = usize;
/// How many nodes the plan runs.
type Value = usize;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_plan_of_no_program_runs_no_node"),
        word: Word::Succeeds,
        input: || 0,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("a_plan_runs_one_node_per_node_of_its_program"),
        word: Word::Succeeds,
        input: || 2,
        expect: Expect::Value(2),
    },
];

fn subject(selects: Input) -> Result<Value, Failure> {
    if selects == 0 {
        return Ok(PhysicalPlan::default().node_count().0);
    }
    let operation = Operation::Select {
        from: Source::Relation(Relation::Function),
        comparisons: Vec::new(),
        kept: Vec::new(),
    };
    let first = [PlanInput::Relation(Relation::Function)];
    let mut program = LogicalProgram::new();
    let head = program.node(LogicalNode {
        operation: &operation,
        inputs: PlanInputList(&first),
        bindings: NodeBindings(&[]),
        scope: StepScope::PerFile,
    });
    let second = [PlanInput::Node(head)];
    let mut emitted = head;
    if selects > 1 {
        emitted = program.node(LogicalNode {
            operation: &operation,
            inputs: PlanInputList(&second),
            bindings: NodeBindings(&[]),
            scope: StepScope::PerFile,
        });
    }
    program.emit(RuleOrdinal(0), emitted);
    plan(&program)
        .map(|planned| planned.node_count().0)
        .map_err(|_| ())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
