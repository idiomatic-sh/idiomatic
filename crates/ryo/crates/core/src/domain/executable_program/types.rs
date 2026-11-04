//! Dense executable instructions and their real publication boundaries.
use crate::domain::{
    coverage::types::RuleIndex,
    operating::errors::OperatingError,
    operator::contract::types::{ModeContract, OperatorContract},
};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::{
    plan_parts::types::StepId,
    plans::types::{Operation, StepScope},
};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct RegionId(pub usize);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct InputId(pub usize);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct LogicalInstruction(pub usize);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum InputSource {
    Relation(Relation),
    Hosted {
        rule: RuleIndex,
        relation: ryo_values::domain::plan_parts::types::HostedNumber,
    },
    Region(RegionId),
    Observation(usize),
    /// One exact producer of a request-qualified named input.
    ObservationMember {
        slot: usize,
        member: usize,
    },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PartitionRequirement {
    RequestInvariant,
    CurrentFile,
    /// Must be bound to the consumer's finite demanded partition set.
    Project,
    InputLineage,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HeldJoinOrder {
    LeftWithEmptyGate,
    RightBeforeLeft,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DemandAction {
    Admit(InputId),
    /// Return without touching subsequent inputs when the admitted population is empty.
    EmptyGate(InputId),
    /// Application selects the admitted transport, never the semantic kernel.
    JoinInputs {
        held: HeldJoinOrder,
    },
    AxisInputs,
    ValidateBeforeInputs,
    ValidateFirstRow,
    PublishAfterSuccess,
}
/// Closed, ordered code interpreted only by the common application invocation.
#[derive(Debug)]
pub(crate) struct DemandProgram {
    pub(super) actions: Box<[DemandAction]>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum OperandSource {
    Boundary(InputId),
    /// Only the immediately preceding instruction's borrowed row; no publication exists.
    PreviousRow,
}
#[derive(Debug)]
pub(crate) struct Instruction {
    pub logical: LogicalInstruction,
    pub producer: RuleIndex,
    pub operation: Operation,
    pub input_delivery: Box<[crate::domain::operating::types::Supply]>,
    pub contract: Arc<OperatorContract>,
    pub inputs: Box<[OperandSource]>,
    pub streaming_demand: DemandProgram,
    pub collected_demand: DemandProgram,
    pub partition: PartitionRequirement,
}
#[derive(Clone, Debug)]
pub(crate) struct OrderedExistence {
    pub source: RegionId,
    pub join: RegionId,
    pub number: crate::domain::ordered_existence::types::NumberRequirement,
    pub partition: Box<[ryo_values::domain::plan_parts::types::ColumnIndex]>,
    pub order: ryo_values::domain::plan_parts::types::ColumnIndex,
    pub extremum: crate::domain::ordered_existence::types::Extremum,
    pub output: Box<[ryo_values::domain::plan_parts::types::JoinColumn]>,
}
#[derive(Debug)]
pub(crate) struct Region {
    pub id: RegionId,
    pub producer: RuleIndex,
    pub needs: Result<Box<[crate::domain::trust::types::RelationRead]>, OperatingError>,
    pub phase: StepScope,
    pub instructions: Box<[Instruction]>,
    pub inputs: Box<[InputSource]>,
    pub consumers: usize,
    pub dependencies: Box<[RegionId]>,
    pub ordered_existence: Option<OrderedExistence>,
    pub column: Option<super::column::ColumnAlternative>,
    pub selection: Option<super::selection::SelectionAlternative>,
    pub publishes: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Origin {
    pub rule: RuleIndex,
    pub step: StepId,
    pub logical: LogicalInstruction,
}
#[derive(Debug)]
pub(crate) struct RuleProgram {
    pub original_steps: Box<[Result<Origin, OperatingError>]>,
    pub emit: Result<RegionId, OperatingError>,
    pub frontier: Box<[Result<InputSource, OperatingError>]>,
}
/// There is no retained RequestGraph, PhysicalRegion, PlanLayout or per-plan evaluator here.
#[derive(Debug)]
pub(crate) struct ExecutableProgram {
    pub(super) regions: Box<[Region]>,
    pub(super) rules: Box<[RuleProgram]>,
    pub(super) logical_regions: Box<[RegionId]>,
    pub(super) observation_regions: Box<[Result<RegionId, OperatingError>]>,
    pub(super) original_instruction_count: usize,
    pub(super) fused_instruction_count: usize,
}
impl ExecutableProgram {
    pub(crate) fn regions(&self) -> &[Region] {
        &self.regions
    }
    pub(crate) fn rules(&self) -> &[RuleProgram] {
        &self.rules
    }
    pub(crate) fn logical_regions(&self) -> &[RegionId] {
        &self.logical_regions
    }
    pub(crate) fn observation_regions(&self) -> &[Result<RegionId, OperatingError>] {
        &self.observation_regions
    }
}
impl DemandProgram {
    pub(crate) fn actions(&self) -> &[DemandAction] {
        &self.actions
    }
}
pub(crate) struct KernelMode {
    pub mode: ModeContract,
}

/// Mandatory producer declarations. No operation kind implies broadcast completeness.
/// Used by preparation/admission only; semantic kernels never receive this interface.
pub(crate) trait ProducerPartitions {
    fn hosted(
        &self,
        rule: RuleIndex,
        relation: ryo_values::domain::plan_parts::types::HostedNumber,
    ) -> Result<PartitionRequirement, OperatingError>;
    fn parameter(
        &self,
        rule: RuleIndex,
        parameter: ryo_values::domain::plan_parts::types::ParameterId,
    ) -> Result<PartitionRequirement, OperatingError>;
    fn relation(&self, relation: Relation) -> PartitionRequirement;
    fn observation(
        &self,
        rule: RuleIndex,
        input: ryo_values::domain::observations::types::ObservationBindingId,
    ) -> Result<PartitionRequirement, OperatingError>;
}

impl Region {
    pub(crate) fn last_operation(&self) -> Result<&Operation, OperatingError> {
        self.instructions
            .last()
            .map(|instruction| &instruction.operation)
            .ok_or(OperatingError::NotRun)
    }
}
