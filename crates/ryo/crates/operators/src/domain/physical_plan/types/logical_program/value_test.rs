//! A logical program numbers its nodes in the order they are added.
use super::value::LogicalProgram;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::physical_plan::types::{
    LogicalNode, NodeBindings, PlanInput, PlanInputList, PlanNodeId,
};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{Input as StepInput, StepScope};
use ryo_values::domain::plans::types::Operation;

/// How many nodes are added.
type Input = usize;
type Value = std::borrow::Cow<'static, [PlanNodeId]>;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("each_node_added_answers_the_next_id"),
    word: Word::Succeeds,
    input: || 3,
    expect: Expect::Value(std::borrow::Cow::Borrowed(&[
        PlanNodeId(0),
        PlanNodeId(1),
        PlanNodeId(2),
    ])),
}];

/// The kinds of no language: a case's kind tests name none.
fn vocabulary() -> ryo_values::domain::descriptions::types::KindVocabulary {
    ryo_values::domain::descriptions::types::KindVocabulary {
        language: ryo_values::domain::descriptions::types::LanguageName("cases".into()),
        kinds: Vec::new(),
    }
}

fn subject(count: Input) -> Result<Value, Failure> {
    let vocabulary = vocabulary();
    let operation = Operation::Select {
        from: StepInput::Relation(Relation::Function),
        comparisons: Vec::new(),
        kept: Vec::new(),
    };
    let inputs = [PlanInput::Relation(Relation::Function)];
    let mut program = LogicalProgram::new(&vocabulary);
    let ids: Vec<PlanNodeId> = (0..count)
        .map(|_| {
            program.node(LogicalNode {
                operation: &operation,
                inputs: PlanInputList(&inputs),
                bindings: NodeBindings(&[]),
                scope: StepScope::PerFile,
            })
        })
        .collect();
    Ok(std::borrow::Cow::Owned(ids))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
