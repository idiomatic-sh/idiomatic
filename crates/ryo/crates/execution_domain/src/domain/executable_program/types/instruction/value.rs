//! A compiled instruction: fields private, made only by the executable-program compiler, read through accessors.
use super::super::demand_program::value::DemandProgram;
use super::super::{LogicalInstruction, PartitionRequirement};
use crate::domain::{coverage::types::RuleIndex, operator::contract::types::OperatorContract};
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
    pub fn logical(&self) -> LogicalInstruction {
        self.logical
    }
    pub fn producer(&self) -> RuleIndex {
        self.producer
    }
    pub fn operation(&self) -> &Operation {
        &self.operation
    }
    pub fn contract(&self) -> &Arc<OperatorContract> {
        &self.contract
    }
    pub fn demand(&self) -> &DemandProgram {
        &self.demand
    }
    pub(crate) fn partition(&self) -> PartitionRequirement {
        self.partition
    }
}
