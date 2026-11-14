//! A compiled instruction: fields private, made only by the executable-program compiler, read through accessors.
use super::super::demand_program::value::DemandProgram;
use super::super::{LogicalInstruction, PartitionRequirement};
use crate::domain::{
    coverage::types::RuleIndex,
    operator::contract::types::OperatorContract,
};
use ryo_values::domain::plans::types::Operation;
use std::sync::Arc;

/// One instruction of a region: the operation of one original step, its prepared contract, operands, demand and partition.
#[derive(Clone, Debug)]
pub struct Instruction {
    logical: LogicalInstruction,
    producer: RuleIndex,
    operation: Operation,
    contract: Arc<OperatorContract>,
    demand: DemandProgram,
    partition: PartitionRequirement,
}

impl Instruction {
    /// The instruction the compiler lowered from one original step.
    pub(in crate::domain::executable_program) fn new(
        logical: LogicalInstruction,
        producer: RuleIndex,
        operation: Operation,
        contract: Arc<OperatorContract>,
        demand: DemandProgram,
        partition: PartitionRequirement,
    ) -> Self {
        Self {
            logical,
            producer,
            operation,
            contract,
            demand,
            partition,
        }
    }
    /// The instruction's logical number.
    pub fn logical(&self) -> LogicalInstruction {
        self.logical
    }
    /// The rule that produces the instruction.
    pub fn producer(&self) -> RuleIndex {
        self.producer
    }
    /// The operation the instruction evaluates.
    pub fn operation(&self) -> &Operation {
        &self.operation
    }
    /// The prepared operator contract of the operation.
    pub fn contract(&self) -> &Arc<OperatorContract> {
        &self.contract
    }
    /// The ordered admissions of the instruction's inputs.
    pub fn demand(&self) -> &DemandProgram {
        &self.demand
    }
    pub(crate) fn partition(&self) -> PartitionRequirement {
        self.partition
    }
}
