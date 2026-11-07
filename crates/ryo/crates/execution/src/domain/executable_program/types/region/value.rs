//! A compiled region: fields private, written only by the executable-program compiler, read through accessors.
use super::super::instruction::value::Instruction;
use super::super::{InputSource, ReaderMultiplicity, RegionId, Route, SourcePartition};
use crate::domain::lineage::types::Lineage;
use crate::domain::{
    coverage::types::RuleIndex, operating::errors::OperatingError, trust::types::RelationRead,
};
use ryo_values::domain::{plan_parts::types::StepScope, plans::types::Operation};

/// One region of the executable program: what it reads, what it computes and how many readers its publication has.
#[derive(Clone, Debug)]
pub(crate) struct Region {
    id: RegionId,
    producer: RuleIndex,
    needs: Result<Box<[RelationRead]>, OperatingError>,
    phase: StepScope,
    instructions: Box<[Instruction]>,
    inputs: Box<[InputSource]>,
    source_partitions: Box<[SourcePartition]>,
    consumers: ReaderMultiplicity,
    dependencies: Box<[RegionId]>,
    route: Route,
    lineage: Lineage,
}

impl Region {
    /// A region as the compiler drafted it, with the generic route and no resolved dependencies yet.
    pub(in crate::domain::executable_program) fn new(
        id: RegionId,
        producer: RuleIndex,
        needs: Result<Box<[RelationRead]>, OperatingError>,
        phase: StepScope,
        instructions: Box<[Instruction]>,
        inputs: Box<[InputSource]>,
        consumers: ReaderMultiplicity,
    ) -> Self {
        Self {
            id,
            producer,
            needs,
            phase,
            instructions,
            inputs,
            source_partitions: Box::new([]),
            consumers,
            dependencies: Box::new([]),
            route: Route::Generic,
            lineage: Lineage::Mixed,
        }
    }
    /// One more reader of the region's publication.
    pub(in crate::domain::executable_program) fn add_consumer(&mut self) {
        self.consumers = ReaderMultiplicity(self.consumers.0 + 1);
    }
    /// The readers of the region's publication, counted by the compiler.
    pub(in crate::domain::executable_program) fn set_consumers(&mut self, consumers: ReaderMultiplicity) {
        self.consumers = consumers;
    }
    /// The one route the compiler kept for the region.
    pub(in crate::domain::executable_program) fn set_route(&mut self, route: Route) {
        self.route = route;
    }
    /// The regions the region depends on, resolved once by the compiler.
    pub(in crate::domain::executable_program) fn set_dependencies(&mut self, dependencies: Box<[RegionId]>) {
        self.dependencies = dependencies;
    }
    /// Where the rows the region publishes come from, compiled once from its instructions and inputs.
    pub(in crate::domain::executable_program) fn set_lineage(&mut self, lineage: Lineage) {
        self.lineage = lineage;
    }
    /// The partition each relation input of the region declares.
    pub(in crate::domain::executable_program) fn set_source_partitions(
        &mut self,
        partitions: Box<[SourcePartition]>,
    ) {
        self.source_partitions = partitions;
    }
    pub(crate) fn id(&self) -> RegionId {
        self.id
    }
    pub(crate) fn producer(&self) -> RuleIndex {
        self.producer
    }
    pub(crate) fn needs(&self) -> &Result<Box<[RelationRead]>, OperatingError> {
        &self.needs
    }
    pub(crate) fn phase(&self) -> StepScope {
        self.phase
    }
    pub(crate) fn instructions(&self) -> &[Instruction] {
        &self.instructions
    }
    pub(crate) fn inputs(&self) -> &[InputSource] {
        &self.inputs
    }
    pub(crate) fn source_partitions(&self) -> &[SourcePartition] {
        &self.source_partitions
    }
    pub(crate) fn consumers(&self) -> ReaderMultiplicity {
        self.consumers
    }
    pub(crate) fn dependencies(&self) -> &[RegionId] {
        &self.dependencies
    }
    pub(crate) fn route(&self) -> &Route {
        &self.route
    }
    pub(crate) fn lineage(&self) -> &Lineage {
        &self.lineage
    }
    pub(crate) fn last_operation(&self) -> Result<&Operation, OperatingError> {
        self.instructions
            .last()
            .map(Instruction::operation)
            .ok_or(OperatingError::NotRun)
    }
}
