//! Executable acceptance steps for planning a program.
use crate::Behavior;
use cucumber::{given, then, when};
use ryo_operators::domain::physical_plan::types::logical_program::value::LogicalProgram;
use ryo_operators::domain::physical_plan::types::{
    LogicalNode, NodeBindings, PlanInput, PlanInputList, PlanNodeId, RuleOrdinal,
};
use ryo_operators::inbound::use_cases::plan::interface::PlanUseCase as _;
use ryo_operators::inbound::use_cases::plan::use_case::PlanUseCaseImpl;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{Input, StepScope};
use ryo_values::domain::plans::types::Operation;

#[given(expr = "the plan program {string}")]
fn given_program(world: &mut Behavior, program: String) {
    world.input = program;
}

#[when(expr = "plan runs")]
fn runs(world: &mut Behavior) {
    let vocabulary = ryo_operators::inbound::use_cases::run::fixtures::vocabulary();
    let operation = Operation::Select {
        from: Input::Relation(Relation::Function),
        comparisons: Vec::new(),
        kept: Vec::new(),
    };
    let read = if world.input == "reads later" { 1 } else { 0 };
    let relation = [PlanInput::Relation(Relation::Function)];
    let node = [PlanInput::Node(PlanNodeId(read))];
    let mut program = LogicalProgram::new(&vocabulary);
    for inputs in [&relation[..], &node[..]] {
        let _id = program.node(LogicalNode {
            operation: &operation,
            inputs: PlanInputList(inputs),
            bindings: NodeBindings(&[]),
            scope: StepScope::PerFile,
        });
    }
    program.emit(RuleOrdinal(0), PlanNodeId(1));
    world.answer = match PlanUseCaseImpl.plan(&program) {
        Ok(_) => "planned".into(),
        Err(fault) => format!("refused {fault:?}"),
    };
}

#[then(expr = "plan answers {string}")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "cucumber owns captured strings"
)]
fn answers(world: &mut Behavior, expected: String) {
    assert_eq!(world.answer, expected);
}
