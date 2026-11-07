//! Dense executable instructions and their real publication boundaries.
use crate::domain::{
    column_kernel::types::Chain, coverage::types::RuleIndex, operating::errors::OperatingError,
    operator::contract::types::OperatorContract, stored_selection::types::Selector,
};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::{
    plan_parts::types::{StepId, StepScope},
    plans::types::Operation,
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
    /// An explicitly declared, closed observation with no producers.
    EmptyObservation(usize),
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
    pub number: crate::domain::self_join_recogniser::types::NumberRequirement,
    pub partition: Box<[ryo_values::domain::plan_parts::types::ColumnIndex]>,
    pub order: ryo_values::domain::plan_parts::types::ColumnIndex,
    pub extremum: crate::domain::self_join_recogniser::types::Extremum,
    pub output: Box<[ryo_values::domain::plan_parts::types::JoinColumn]>,
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct SourcePartition {
    pub source: InputSource,
    pub requirement: PartitionRequirement,
}
/// How a region is evaluated. Exactly one per region, kept by `service::choose`.
#[derive(Clone, Debug, Default)]
pub(crate) enum Route {
    /// The registered operator of each instruction, in order: the definition of the operation.
    #[default]
    Generic,
    /// The reviewed ordered-existence replacement of a self-join anti-join select.
    OrderedExistence(OrderedExistence),
    /// The column kernel over a stored equality index, replacing a join-filter-project chain.
    Column(ColumnAlternative),
    /// The stored selection of a direct select or project over one relation.
    Selection(SelectionAlternative),
    /// The native positions of a step that reads only the tree.
    NativePositions(Arc<crate::domain::lowering::types::LoweredShape>),
}
/// Whether the composition wires one strategy: an unwired strategy offers no route, so the generic
/// evaluation of the operators serves what it would have served.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RouteWiring {
    /// The strategy is offered to the one selection rule.
    #[default]
    Wired,
    /// The strategy is never offered.
    Unwired,
}
/// Which strategies the composition wires, every one by default. Each strategy implements a driver
/// port of `ryo_operators`; the policy only chooses which implementations a program may use, so a
/// differential run can compare an engine with a strategy and the same engine without it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RoutePolicy {
    /// The ordered-existence replacement of a self-join anti-join select.
    pub ordered_existence: RouteWiring,
    /// The column kernel over a stored equality index.
    pub column_kernel: RouteWiring,
    /// The stored (compact) selection of a direct select or project.
    pub stored_selection: RouteWiring,
    /// The lowered native positions of a step that reads only the tree.
    pub native_positions: RouteWiring,
    /// The tree join cursor that serves a native join without the generic join item.
    pub tree_join: RouteWiring,
}
/// A route without its payload: what a permit is issued for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RouteKind {
    Generic,
    OrderedExistence,
    Column,
    NativePositions,
}
/// What the family analysers found eligible for one region, before the one rule keeps a route.
#[derive(Clone, Debug, Default)]
pub(crate) struct RouteOffers {
    pub ordered: Option<OrderedExistence>,
    pub column: Option<ColumnAlternative>,
    pub selection: Option<SelectionAlternative>,
    pub native: Option<Arc<crate::domain::lowering::types::LoweredShape>>,
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
    pub route: Route,
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
    pub(super) file_readers: FileReaderGraph,
    pub(super) rules: Box<[RuleProgram]>,
    pub(super) logical_regions: Box<[RegionId]>,
    pub(super) observation_regions: Box<[Result<RegionId, OperatingError>]>,
    /// The strategies the composition wired when this program was compiled.
    pub(super) routes: RoutePolicy,
    /// The fusion classification of every instruction, indexed by its logical number.
    #[cfg(test)]
    pub fusibility: Box<[super::fusion::types::Fusibility]>,
}
impl ExecutableProgram {
    /// How the compiler classified an instruction against the region of its producer.
    #[cfg(test)]
    pub(crate) fn fusibility(
        &self,
        logical: LogicalInstruction,
    ) -> Option<super::fusion::types::Fusibility> {
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
            InputSource::EmptyObservation(slot) => {
                let root = self.observation_regions.get(slot)
                    .ok_or(OperatingError::NotRead)?.as_ref().map_err(Clone::clone)?;
                self.regions.get(root.0).and_then(|region| region.instructions.last())
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
    /// The strategies the composition wired for this program.
    pub(crate) fn routes(&self) -> RoutePolicy {
        self.routes
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
        input: ryo_values::domain::plan_parts::types::ObservationBindingId,
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

/// The compiler's fusion classification of the whole request graph, indexed by original node.
#[derive(Debug)]
pub(crate) struct Fusibilities {
    pub fusibility: Vec<super::fusion::types::Fusibility>,
    /// Independent readers of each node's publication, including rule roots.
    pub readers: Vec<ReaderMultiplicity>,
    /// The identity that holds each node's publication, when one does.
    pub held: Vec<Option<super::fusion::types::Barrier>>,
}

/// Number of independent readers of the same publication.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct ReaderMultiplicity(pub usize);
/// One direct consumer and the number of its repeated dependency edges.
#[derive(Debug)]
pub(crate) struct ConsumerObligation {
    pub consumer: RegionId,
    pub readers: ReaderMultiplicity,
}
/// Request-compiled potential obligations, indexed by external root and producer.
#[derive(Debug)]
pub(crate) struct FileReaderGraph {
    pub consumers: std::collections::HashMap<
        RegionId,
        std::collections::HashMap<RegionId, Box<[ConsumerObligation]>>,
    >,
}
/// The typed semantic shape required by the shared physical invocation.
#[derive(Clone, Debug)]
pub(crate) struct ColumnAlternative {
    pub(super) target: RegionId,
    pub(super) left: RegionId,
    pub(super) join: RegionId,
    pub(super) filter: RegionId,
    pub(super) chain: Chain,
}

impl ColumnAlternative {
    pub(crate) fn target(&self) -> RegionId {
        self.target
    }

    pub(crate) fn left(&self) -> RegionId {
        self.left
    }

    pub(crate) fn join(&self) -> RegionId {
        self.join
    }

    pub(crate) fn filter(&self) -> RegionId {
        self.filter
    }

    pub(crate) fn chain(&self) -> &Chain {
        &self.chain
    }
}
/// An immutable semantic alternative for one exact compiled instruction.
/// The selector is the request-prepared Arc also registered with the derived owner.
#[derive(Clone, Debug)]
pub(crate) struct SelectionAlternative {
    pub(super) target: RegionId,
    pub(super) selector: Arc<Selector>,
    pub(super) producer: RuleIndex,
    pub(super) logical: LogicalInstruction,
}

impl SelectionAlternative {
    pub(crate) fn target(&self) -> RegionId {
        self.target
    }

    pub(crate) fn selector(&self) -> &Arc<Selector> {
        &self.selector
    }

    pub(crate) fn producer(&self) -> RuleIndex {
        self.producer
    }

    pub(crate) fn logical(&self) -> LogicalInstruction {
        self.logical
    }
}
