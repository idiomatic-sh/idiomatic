//! Dense executable instructions and their real publication boundaries.
use crate::domain::operator::types::ReaderMultiplicity;
use crate::domain::source_partitions::types::PartitionRequirement;
use crate::domain::{coverage::types::RuleIndex, operating::errors::OperatingError};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{StepId, StepScope};

pub mod absent_regions;
pub mod absent_roots;
pub mod demand_program;
pub mod file_rule_demand;
pub mod instruction;
pub mod interpreted_program;
pub mod region;

/// Distinct physical regions in this compiled program.
#[derive(Debug, Clone, Copy)]
pub struct RegionCount(pub usize);

/// A region of the executable program, by its place among the program's regions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RegionId(pub usize);
/// An input of an instruction, by its place among the instruction's inputs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct InputId(pub usize);
/// An instruction of the compiled request, by its logical number.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LogicalInstruction(pub usize);
/// Where an instruction's input comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputSource {
    /// A relation the language provides.
    Relation(Relation),
    /// A relation the host provides to one rule.
    Hosted {
        /// The rule the host relation is provided to.
        rule: RuleIndex,
        /// The host relation, by its number in the rule.
        relation: ryo_values::domain::plan_parts::types::HostedNumber,
    },
    /// The output of another region.
    Region(RegionId),
    /// A request-qualified named observation.
    Observation(ObservationPlace),
    /// An explicitly declared, closed observation with no producers.
    EmptyObservation(ObservationPlace),
    /// One exact producer of a request-qualified named input.
    ObservationMember {
        /// The observation the producer is a member of.
        slot: ObservationPlace,
        /// The producer, by its place among the observation's members.
        member: ProducerMember,
    },
}
/// The request-qualified slot of one named input.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ObservationPlace(pub usize);
/// One producer of a named input, by its place among the slot's producers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ProducerMember(pub usize);
/// What the engine does for a demanded input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DemandAction {
    /// Admits the input.
    Admit(InputId),
}
/// A self join recognised as an ordered existence: the producer, its number requirement, partition, order, extremum and output.
#[derive(Clone, Debug)]
pub struct OrderedExistence {
    /// The region whose rows the existence reads.
    pub source: RegionId,
    /// The number the rows must carry.
    pub number: crate::domain::self_join_recogniser::types::NumberRequirement,
    /// The columns the rows are partitioned by.
    pub partition: Box<[ryo_values::domain::plan_parts::types::ColumnIndex]>,
    /// The column that orders the rows of a partition.
    pub order: ryo_values::domain::plan_parts::types::ColumnIndex,
    /// Which end of the order is kept.
    pub extremum: crate::domain::self_join_recogniser::types::Extremum,
    /// The columns the existence outputs.
    pub output: Box<[ryo_values::domain::plan_parts::types::JoinColumn]>,
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct SourcePartition {
    pub source: InputSource,
    pub requirement: PartitionRequirement,
}
/// How a region is evaluated. Exactly one per region, decided when the program is compiled.
#[derive(Clone, Debug, Default)]
pub enum Route {
    /// The registered operator of each instruction, in order: the operators' one meaning of its kind.
    #[default]
    Generic,
    /// The ordered-existence replacement of a self-join anti-join select the recogniser resolved: the one
    /// implementation of that region, through the operators' ordered-existence driver port.
    OrderedExistence(OrderedExistence),
}
/// A route without its payload: what a permit is issued for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RouteKind {
    /// The generic evaluation of an instruction.
    Generic,
    /// An ordered existence.
    OrderedExistence,
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
pub struct RegexIdentity(pub usize);
/// Whether a row passes the first applicable row gate of an instruction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowGate {
    /// The row is invoked.
    Admitted,
    /// The row is skipped before invocation.
    Withheld,
}
/// Where an original step of a rule is in the executable program.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StepOrigin {
    /// The rule the step belongs to.
    pub rule: RuleIndex,
    /// The step, by its place in the rule's plan.
    pub step: StepId,
    /// The instruction the step was compiled to.
    pub logical: LogicalInstruction,
    /// Where that instruction is: its region and its place among the region's instructions; `None` when the program has no
    /// such instruction, which reads as a step that has not run.
    pub place: Option<StepSite>,
}
/// Where one compiled instruction is: its region and its place among that region's instructions.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::interpreted_program::types::{InstructionPlace, RegionId, StepSite};
/// let site = StepSite { region: RegionId(0), instruction: InstructionPlace(2) };
/// assert_eq!(site.instruction, InstructionPlace(2));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StepSite {
    /// The region the instruction is in.
    pub region: RegionId,
    /// The instruction's place among the region's instructions.
    pub instruction: InstructionPlace,
}
/// An instruction of a region, by its place among the region's instructions.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::interpreted_program::types::InstructionPlace;
/// assert_eq!(InstructionPlace(3).0, 3);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InstructionPlace(pub usize);
/// One rule's part of the executable program: its root readers and where each of its original steps is.
#[derive(Debug)]
pub struct RuleProgram {
    /// External root-reader obligations, including repeated aliases.
    pub root_readers: Box<[RegionId]>,
    /// Where each original step of the rule is, or why it could not be placed.
    pub original_steps: Box<[Result<StepOrigin, OperatingError>]>,
    /// Collected semantic root shared only with collected frontier/observation aliases.
    pub emit: Result<RegionId, OperatingError>,
    /// Private diagnostic delivery boundary; no ordinary dependency or persisted alias targets it.
    pub frontier: Box<[Result<InputSource, OperatingError>]>,
}

/// The program of every rule.
#[derive(Debug)]
pub(crate) struct RulePrograms {
    pub rules: Vec<RuleProgram>,
    /// The region each plan's named output reads, in the plan's own output order: the lineage pass reads it.
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

/// Request-compiled file roots: the regions a file may select as external readers.
#[derive(Debug, Default)]
pub struct FileReaderGraph {
    /// Every region a rule names as a root reader.
    pub roots: std::collections::HashSet<RegionId>,
}
/// Initial reader count of every region for one file's selection of external roots.
#[derive(Debug)]
pub struct RemainingTable {
    /// The initial reader count of every region, by region.
    pub counts: Box<[ReaderMultiplicity]>,
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
