//! Dense executable instructions and their real publication boundaries.
use crate::domain::{
    column_kernel::types::Chain, coverage::types::RuleIndex, operating::errors::OperatingError,
    stored_selection::types::Selector,
};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::StepId;
use std::sync::Arc;

pub(crate) mod demand_program;
pub(crate) mod executable_program;
pub(crate) mod instruction;
pub(crate) mod region;

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
    Observation(ObservationPlace),
    /// An explicitly declared, closed observation with no producers.
    EmptyObservation(ObservationPlace),
    /// One exact producer of a request-qualified named input.
    ObservationMember {
        slot: ObservationPlace,
        member: ProducerMember,
    },
}
/// The request-qualified slot of one named input.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct ObservationPlace(pub usize);
/// One producer of a named input, by its place among the slot's producers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct ProducerMember(pub usize);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum OperandSource {
    Boundary(InputId),
    /// Only the immediately preceding instruction's borrowed row; no publication exists.
    PreviousRow,
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
    #[cfg(any(test, feature = "fixtures"))]
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
/// Whether a root of its rule reads an original step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum StepUse {
    /// A root reads it, directly or through the steps that read it.
    Read,
    /// No root reads it.
    Unread,
}
/// The address of the compiled regular expression an operator call was prepared with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RegexIdentity(pub usize);
/// Whether an invocation holds the native join lease, whose views a join call must then carry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum JoinLease {
    /// The native join lease is held.
    Native,
    /// No native join lease is held.
    Absent,
}
/// Whether a row passes the first applicable row gate of an instruction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RowGate {
    /// The row is invoked.
    Admitted,
    /// The row is skipped before invocation.
    Withheld,
}
/// The column-kernel alternative of one join-filter-project chain: its target, left input, join and filter, in original
/// nodes when the compiler finds it and in executable regions once lowered. The typed semantic shape required by the shared
/// physical invocation.
#[derive(Clone, Debug)]
pub(crate) struct ColumnAlternative<Id = RegionId> {
    pub target: Id,
    pub left: Id,
    pub join: Id,
    pub filter: Id,
    pub chain: Chain,
}
/// An immutable semantic alternative for one exact compiled instruction: the stored selection of a direct select or
/// project. The selector is the request-prepared Arc also registered with the derived owner.
#[derive(Clone, Debug)]
pub(crate) struct SelectionAlternative {
    pub target: RegionId,
    pub selector: Arc<Selector>,
    pub producer: RuleIndex,
    pub logical: LogicalInstruction,
}
/// Where an original step of a rule is in the executable program.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct StepOrigin {
    pub rule: RuleIndex,
    pub step: StepId,
    pub logical: LogicalInstruction,
}
#[derive(Debug)]
pub(crate) struct RuleProgram {
    /// External root-reader obligations, including repeated aliases.
    pub root_readers: Box<[RegionId]>,
    pub original_steps: Box<[Result<StepOrigin, OperatingError>]>,
    /// Collected semantic root shared only with collected frontier/observation aliases.
    pub emit: Result<RegionId, OperatingError>,
    /// Private diagnostic delivery boundary; no ordinary dependency or persisted alias targets it.
    pub frontier: Box<[Result<InputSource, OperatingError>]>,
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

/// The program of every rule, and the region each plan's named output reads, in the plan's own output order.
#[derive(Debug)]
pub(crate) struct RulePrograms {
    pub rules: Vec<RuleProgram>,
    pub observed_roots: Vec<Vec<Option<RegionId>>>,
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
    /// The last initial reader counts, shared by every file that selects the same roots.
    pub remaining: std::sync::Mutex<RemainingMemo>,
}
/// How many reader-count tables a request has computed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ComputationCount(pub usize);
/// The retained initial reader counts and how many times they were computed.
#[derive(Debug, Default)]
pub(crate) struct RemainingMemo {
    pub table: Option<std::sync::Arc<RemainingTable>>,
    pub computations: ComputationCount,
}
/// Initial reader count of every region for one selection of external roots.
#[derive(Debug)]
pub(crate) struct RemainingTable {
    pub selected: std::collections::HashMap<RegionId, ReaderMultiplicity>,
    pub counts: Box<[ReaderMultiplicity]>,
}
/// A shared table and the cumulative number of tables the request has computed.
#[derive(Debug)]
pub(crate) struct Remaining {
    pub table: std::sync::Arc<RemainingTable>,
    #[cfg(test)]
    pub computations: ComputationCount,
}
/// One region as the compiler drafts it, before equivalent regions merge: its instructions `I` in order.
pub(super) struct DraftRegion<I> {
    pub phase: StepScope,
    pub nodes: Vec<LogicalInstruction>,
    pub instructions: Vec<I>,
    pub inputs: Vec<crate::domain::file_program::types::NodeInput>,
    pub needs: Result<Vec<crate::domain::trust::types::RelationRead>, OperatingError>,
    /// Whether every instruction's answer depends only on its operation and inputs.
    pub sharing: super::fusion::types::RegionSharing,
}
/// Whether two drafted regions compute the same rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Equivalence {
    /// Same instructions, same inputs, same phase.
    Same,
    /// They differ.
    Different,
}
/// The bucket equivalent regions share: a hash of what their comparison reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) struct RegionBucket(pub u64);
/// What merging equivalent regions kept and how many comparisons it made.
#[derive(Debug)]
pub(super) struct RegionMerge {
    /// The kept regions in order.
    pub shared: Vec<DraftIndex>,
    pub comparisons: ComparisonCount,
}
/// The place of a drafted region among the drafts of one compile.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) struct DraftIndex(pub usize);
/// How many full comparisons of two regions a merge made.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct ComparisonCount(pub usize);
