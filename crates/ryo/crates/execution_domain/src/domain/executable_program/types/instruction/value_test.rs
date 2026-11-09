//! An instruction answers exactly the operation, contract, operands, demand and partition it was compiled with.

use super::value::Instruction;
use crate::domain::{
    coverage::types::RuleIndex,
    executable_program::types::{DemandAction, InputId, LogicalInstruction, demand_program::value::DemandProgram},
    source_partitions::types::{PartitionRequirement},
    operation_schema::types::InputShape,
    operator::contract::service::prepare_for,
};
use ryo_relations::domain::relations::{service::kinds, types::Relation};
use ryo_values::domain::{
    plan_parts::types::Input, plans::types::Operation, query_operators::types::OperationKind,
};
use std::sync::Arc;

#[test]
fn an_instruction_answers_what_it_was_compiled_with() {
    let operation = Operation::Select {
        from: Input::Relation(Relation::Function),
        comparisons: vec![],
        kept: vec![],
    };
    let inputs = [InputShape {
        kinds: kinds(Relation::Function),
        tree: None,
    }];
    let contract = Arc::new(
        prepare_for(
            OperationKind::Select,
            &operation,
            &inputs,
            &[],
            ryo_values::domain::plan_parts::types::StepScope::PerFile,
        )
        .expect("the select contract prepares"),
    );
    let instruction = Instruction::new(
        LogicalInstruction(3),
        RuleIndex(1),
        operation.clone(),
        Arc::clone(&contract),
        DemandProgram::new(Box::new([DemandAction::Admit(InputId(0))])),
        PartitionRequirement::CurrentFile,
    );
    assert_eq!(instruction.logical(), LogicalInstruction(3));
    assert_eq!(instruction.producer(), RuleIndex(1));
    assert_eq!(instruction.operation(), &operation);
    assert!(Arc::ptr_eq(instruction.contract(), &contract));
    assert_eq!(
        instruction.demand().actions(),
        [DemandAction::Admit(InputId(0))]
    );
    assert_eq!(instruction.partition(), PartitionRequirement::CurrentFile);
}
