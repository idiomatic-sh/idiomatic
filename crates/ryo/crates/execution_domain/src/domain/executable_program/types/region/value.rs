//! A compiled region: fields private, written only by the executable-program compiler, read through accessors.
use super::super::instruction::value::Instruction;
use super::super::{InputSource, RegionId, Route, SourcePartition};
use crate::domain::operator::types::ReaderMultiplicity;
use crate::domain::{
    coverage::types::RuleIndex, operating::errors::OperatingError, trust::types::RelationRead,
};
use ryo_values::domain::lineage::types::Lineage;
use ryo_values::domain::{plan_parts::types::StepScope, plans::types::Operation};

/// One region of the executable program: what it reads, what it computes and how many readers its publication has.
#[derive(Clone, Debug)]
pub struct Region {
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
        producer: RuleIndex,
        needs: Result<Box<[RelationRead]>, OperatingError>,
        phase: StepScope,
        instructions: Box<[Instruction]>,
        inputs: Box<[InputSource]>,
        consumers: ReaderMultiplicity,
    ) -> Self {
        Self {
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
    pub(in crate::domain::executable_program) fn set_consumers(
        &mut self,
        consumers: ReaderMultiplicity,
    ) {
        self.consumers = consumers;
    }
    /// The one route the compiler kept for the region.
    pub(in crate::domain::executable_program) fn set_route(&mut self, route: Route) {
        self.route = route;
    }
    /// The regions the region depends on, resolved once by the compiler.
    pub(in crate::domain::executable_program) fn set_dependencies(
        &mut self,
        dependencies: Box<[RegionId]>,
    ) {
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
    /// The rule that produces the region.
    pub fn producer(&self) -> RuleIndex {
        self.producer
    }
    /// The reads whose absence of rows the region trusts, or why they could not be known.
    pub fn needs(&self) -> &Result<Box<[RelationRead]>, OperatingError> {
        &self.needs
    }
    /// Whether the region runs for each file or for the project.
    pub fn phase(&self) -> StepScope {
        self.phase
    }
    /// The region's instructions, in order.
    pub fn instructions(&self) -> &[Instruction] {
        &self.instructions
    }
    /// Where each of the region's inputs comes from.
    pub fn inputs(&self) -> &[InputSource] {
        &self.inputs
    }
    pub(crate) fn source_partitions(&self) -> &[SourcePartition] {
        &self.source_partitions
    }
    /// How many readers consume the region.
    pub fn consumers(&self) -> ReaderMultiplicity {
        self.consumers
    }
    /// The regions the region depends on.
    pub fn dependencies(&self) -> &[RegionId] {
        &self.dependencies
    }
    /// How the region is evaluated.
    pub fn route(&self) -> &Route {
        &self.route
    }
    /// Which file the rows of the region come from.
    pub fn lineage(&self) -> &Lineage {
        &self.lineage
    }
    /// The last operation of the region, or why it has none.
    pub fn last_operation(&self) -> Result<&Operation, OperatingError> {
        self.instructions
            .last()
            .map(Instruction::operation)
            .ok_or(OperatingError::NotRun)
    }
}
