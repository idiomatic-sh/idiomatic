//! The sole immutable executable form: fields private, made only by the executable-program compiler, read through accessors.
//! There is no retained RequestGraph, PhysicalRegion, PlanLayout or per-plan evaluator here.
use super::super::region::value::Region;
use super::super::{
    FileReaderGraph, InputSource, LogicalInstruction, PartitionRequirement, RegionId, RoutePolicy,
    RuleProgram,
};
use crate::domain::operating::errors::OperatingError;
use ryo_values::domain::{plan_parts::types::StepScope, plans::types::Operation};

/// The compiled program of one request.
#[derive(Debug)]
pub(crate) struct ExecutableProgram {
    regions: Box<[Region]>,
    file_readers: FileReaderGraph,
    rules: Box<[RuleProgram]>,
    logical_regions: Box<[RegionId]>,
    observation_regions: Box<[Result<RegionId, OperatingError>]>,
    /// The strategies the composition wired when this program was compiled.
    routes: RoutePolicy,
    /// The fusion classification of every instruction, indexed by its logical number.
    #[cfg(test)]
    fusibility: Box<[crate::domain::executable_program::fusion::types::Fusibility]>,
}

impl ExecutableProgram {
    /// The program the compiler made.
    pub(in crate::domain::executable_program) fn new(
        regions: Box<[Region]>,
        file_readers: FileReaderGraph,
        rules: Box<[RuleProgram]>,
        logical_regions: Box<[RegionId]>,
        observation_regions: Box<[Result<RegionId, OperatingError>]>,
        routes: RoutePolicy,
    ) -> Self {
        Self {
            regions,
            file_readers,
            rules,
            logical_regions,
            observation_regions,
            routes,
            #[cfg(test)]
            fusibility: Box::new([]),
        }
    }
    /// The same program with the fusion classification the compiler made, which only the controls read.
    #[cfg(test)]
    pub(in crate::domain::executable_program) fn with_fusibility(
        mut self,
        fusibility: Box<[crate::domain::executable_program::fusion::types::Fusibility]>,
    ) -> Self {
        self.fusibility = fusibility;
        self
    }
    /// How the compiler classified an instruction against the region of its producer.
    #[cfg(test)]
    pub(crate) fn fusibility(
        &self,
        logical: LogicalInstruction,
    ) -> Option<crate::domain::executable_program::fusion::types::Fusibility> {
        self.fusibility.get(logical.0).copied()
    }
    pub(crate) fn file_readers(&self) -> &FileReaderGraph {
        &self.file_readers
    }
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
                .map(Region::phase)
                .ok_or(OperatingError::NotRun),
            InputSource::Observation(slot) => {
                let root = self
                    .observation_regions
                    .get(slot.0)
                    .ok_or(OperatingError::NotRun)?
                    .as_ref()
                    .map_err(Clone::clone)?;
                self.regions
                    .get(root.0)
                    .map(Region::phase)
                    .ok_or(OperatingError::NotRun)
            }
            _ => Err(OperatingError::WrongOperation),
        }
    }
    pub(crate) fn source_partition(
        &self,
        source: InputSource,
    ) -> Result<PartitionRequirement, OperatingError> {
        let last = |region: &Region| {
            region
                .instructions()
                .last()
                .map(super::super::instruction::value::Instruction::partition)
        };
        match source {
            InputSource::Relation(_) => self
                .regions
                .iter()
                .flat_map(|region| region.source_partitions().iter())
                .find(|declaration| declaration.source == source)
                .map(|declaration| declaration.requirement)
                .ok_or(OperatingError::NotRead),
            InputSource::Region(id) => self
                .regions
                .get(id.0)
                .and_then(last)
                .ok_or(OperatingError::NotRead),
            InputSource::Hosted { rule, relation } => self
                .regions
                .iter()
                .flat_map(|region| region.instructions().iter())
                .find(|instruction| {
                    instruction.producer() == rule
                        && matches!(instruction.operation(), Operation::Hosted { relation: actual } if *actual == relation)
                })
                .map(super::super::instruction::value::Instruction::partition)
                .ok_or(OperatingError::NotRead),
            InputSource::EmptyObservation(slot) => {
                let root = self
                    .observation_regions
                    .get(slot.0)
                    .ok_or(OperatingError::NotRead)?
                    .as_ref()
                    .map_err(Clone::clone)?;
                self.regions
                    .get(root.0)
                    .and_then(last)
                    .ok_or(OperatingError::NotRead)
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
    /// The strategies the composition wired for this program.
    pub(crate) fn routes(&self) -> RoutePolicy {
        self.routes
    }
}
