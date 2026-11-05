//! Dense executable instructions and their real publication boundaries.
use crate::domain::{
    coverage::types::RuleIndex, operating::errors::OperatingError,
    operator::contract::types::OperatorContract,
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
pub(crate) enum DemandAction {
    Admit(InputId),
}
/// Closed, ordered code interpreted only by the common application invocation.
#[derive(Clone, Debug)]
pub(crate) struct DemandProgram {
    pub(super) actions: Box<[DemandAction]>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum OperandSource {
    Boundary(InputId),
    /// Only the immediately preceding instruction's borrowed row; no publication exists.
    PreviousRow,
}
#[derive(Clone, Debug)]
pub(crate) struct Instruction {
    pub logical: LogicalInstruction,
    pub producer: RuleIndex,
    pub operation: Operation,
    pub contract: Arc<OperatorContract>,
    pub inputs: Box<[OperandSource]>,
    pub demand: DemandProgram,
    pub partition: PartitionRequirement,
}
#[derive(Clone, Debug)]
pub(crate) struct OrderedExistence {
    pub source: RegionId,
    pub number: crate::domain::ordered_existence::types::NumberRequirement,
    pub partition: Box<[ryo_values::domain::plan_parts::types::ColumnIndex]>,
    pub order: ryo_values::domain::plan_parts::types::ColumnIndex,
    pub extremum: crate::domain::ordered_existence::types::Extremum,
    pub output: Box<[ryo_values::domain::plan_parts::types::JoinColumn]>,
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct SourcePartition {
    pub source: InputSource,
    pub requirement: PartitionRequirement,
}
#[derive(Clone, Debug)]
pub(crate) struct Region {
    pub id: RegionId,
    pub producer: RuleIndex,
    pub needs: Result<Box<[crate::domain::trust::types::RelationRead]>, OperatingError>,
    pub phase: StepScope,
    pub instructions: Box<[Instruction]>,
    pub inputs: Box<[InputSource]>,
    pub source_partitions: Box<[SourcePartition]>,
    pub consumers: usize,
    pub dependencies: Box<[RegionId]>,
    pub ordered_existence: Option<OrderedExistence>,
    pub column: Option<super::column::ColumnAlternative>,
    pub selection: Option<super::selection::SelectionAlternative>,
    pub native_positions: Option<Arc<crate::domain::lowering::types::LoweredShape>>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Origin {
    pub rule: RuleIndex,
    pub step: StepId,
    pub logical: LogicalInstruction,
}
#[derive(Debug)]
pub(crate) struct RuleProgram {
    /// External root-reader obligations, including repeated aliases.
    pub root_readers: Box<[RegionId]>,
    pub original_steps: Box<[Result<Origin, OperatingError>]>,
    /// Collected semantic root shared only with collected frontier/observation aliases.
    pub emit: Result<RegionId, OperatingError>,
    /// Private diagnostic delivery boundary; no ordinary dependency or persisted alias targets it.
    pub frontier: Box<[Result<InputSource, OperatingError>]>,
}
/// There is no retained RequestGraph, PhysicalRegion, PlanLayout or per-plan evaluator here.
#[derive(Debug)]
pub(crate) struct ExecutableProgram {
    pub(super) regions: Box<[Region]>,
    pub(super) rules: Box<[RuleProgram]>,
    pub(super) logical_regions: Box<[RegionId]>,
    pub(super) observation_regions: Box<[Result<RegionId, OperatingError>]>,
}
impl ExecutableProgram {
    /// The prepared physical phase of an original frontier root. No file or
    /// project runtime can change this declaration while admitting its input.
    pub(crate) fn frontier_phase(&self, source: InputSource) -> Result<StepScope, OperatingError> {
        match source {
            InputSource::Relation(_) => match self.source_partition(source)? {
                PartitionRequirement::CurrentFile => Ok(StepScope::PerFile),
                PartitionRequirement::Project => Ok(StepScope::CrossFile),
                _ => Err(OperatingError::WrongOperation),
            },
            InputSource::Region(id) => self
                .regions
                .get(id.0)
                .map(|region| region.phase)
                .ok_or(OperatingError::NotRun),
            InputSource::Observation(slot) => {
                let root = self
                    .observation_regions
                    .get(slot)
                    .ok_or(OperatingError::NotRun)?
                    .as_ref()
                    .map_err(Clone::clone)?;
                self.regions
                    .get(root.0)
                    .map(|region| region.phase)
                    .ok_or(OperatingError::NotRun)
            }
            _ => Err(OperatingError::WrongOperation),
        }
    }
    pub(crate) fn source_partition(
        &self,
        source: InputSource,
    ) -> Result<PartitionRequirement, OperatingError> {
        match source {
            InputSource::Relation(_) => self.regions.iter().flat_map(|region| region.source_partitions.iter()).find(|declaration| declaration.source == source).map(|declaration| declaration.requirement).ok_or(OperatingError::NotRead),
            InputSource::Region(id) => self.regions.get(id.0).and_then(|region| region.instructions.last()).map(|instruction| instruction.partition).ok_or(OperatingError::NotRead),
            InputSource::Hosted { rule, relation } => {
                self.regions.iter().flat_map(|region| region.instructions.iter())
                    .find(|instruction| instruction.producer == rule && matches!(instruction.operation, Operation::Hosted { relation: actual } if actual == relation))
                    .map(|instruction| instruction.partition).ok_or(OperatingError::NotRead)
            }
            _ => Err(OperatingError::NotRead),
        }
    }
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
