//! The planner refuses a program only for its shape: a node that reads a later one.
use super::service::plan;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::types::logical_program::value::LogicalProgram;
use crate::domain::physical_plan::types::{
    LogicalNode, NodeBindings, PlanInput, PlanInputList, PlanNodeId,
};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{Input as StepInput, StepScope};
use ryo_values::domain::plans::types::Operation;

/// The node the program's one node reads.
type Input = u32;
/// How many of its nodes the plan could not lower yet.
type Value = usize;
type Failure = PortableOperatorFault;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_node_reading_only_a_relation_is_planned"),
        word: Word::Succeeds,
        input: || u32::MAX,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("a_node_reading_itself_is_refused"),
        word: Word::Refuses,
        input: || 0,
        expect: Expect::Error(PortableOperatorFault::WrongOperation),
    },
];

/// The kinds of no language: a case's kind tests name none.
fn vocabulary() -> ryo_values::domain::descriptions::types::KindVocabulary {
    ryo_values::domain::descriptions::types::KindVocabulary {
        language: ryo_values::domain::descriptions::types::LanguageName("cases".into()),
        kinds: Vec::new(),
    }
}

fn subject(read: Input) -> Result<Value, Failure> {
    let vocabulary = vocabulary();
    let operation = Operation::Select {
        from: StepInput::Relation(Relation::Function),
        comparisons: Vec::new(),
        kept: Vec::new(),
    };
    let inputs = [if read == u32::MAX {
        PlanInput::Relation(Relation::Function)
    } else {
        PlanInput::Node(PlanNodeId(read))
    }];
    let mut program = LogicalProgram::new(&vocabulary);
    let _id = program.node(LogicalNode {
        operation: &operation,
        inputs: PlanInputList(&inputs),
        bindings: NodeBindings(&[]),
        scope: StepScope::PerFile,
    });
    plan(&program).map(|planned| planned.unserved().len())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

#[cfg(test)]
mod catalog_test;
