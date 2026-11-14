//! The plan use case: a program is refused only for its shape, never for a kind it cannot lower yet.
use super::interface::PlanUseCase as _;
use super::use_case::PlanUseCaseImpl;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::types::logical_program::value::LogicalProgram;
use crate::domain::physical_plan::types::{
    LogicalNode, NodeBindings, PlanInput, PlanInputList, PlanNodeId, RuleOrdinal,
};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{Input as StepInput, StepScope};
use ryo_values::domain::plans::types::Operation;

#[derive(Clone, Copy)]
enum Scenario {
    ReadsEarlier,
    ReadsLater,
    EmitsUnknown,
}
type Input = Scenario;
type Value = usize;
type Failure = PortableOperatorFault;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_program_reading_only_earlier_nodes_is_planned"),
        word: Word::Succeeds,
        input: || Scenario::ReadsEarlier,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("a_node_reading_a_later_node_is_refused"),
        word: Word::Refuses,
        input: || Scenario::ReadsLater,
        expect: Expect::Error(PortableOperatorFault::WrongOperation),
    },
    Case {
        name: CaseName("an_emit_of_no_node_is_refused"),
        word: Word::Refuses,
        input: || Scenario::EmitsUnknown,
        expect: Expect::Error(PortableOperatorFault::WrongOperation),
    },
];

/// The unserved nodes of a two-node program: a select of a relation, then a select of node `read`, emitting `emit`.
fn subject(scenario: Scenario) -> Result<Value, Failure> {
    let operation = Operation::Select {
        from: StepInput::Relation(Relation::Function),
        comparisons: Vec::new(),
        kept: Vec::new(),
    };
    let (read, emit) = match scenario {
        Scenario::ReadsEarlier => (0, 1),
        Scenario::ReadsLater => (1, 1),
        Scenario::EmitsUnknown => (0, 2),
    };
    let relation = [PlanInput::Relation(Relation::Function)];
    let node = [PlanInput::Node(PlanNodeId(read))];
    let mut program = LogicalProgram::new();
    for inputs in [&relation[..], &node[..]] {
        let _id = program.node(LogicalNode {
            operation: &operation,
            inputs: PlanInputList(inputs),
            bindings: NodeBindings(&[]),
            scope: StepScope::PerFile,
        });
    }
    program.emit(RuleOrdinal(0), PlanNodeId(emit));
    PlanUseCaseImpl
        .plan(&program)
        .map(|plan| plan.unserved().len())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
