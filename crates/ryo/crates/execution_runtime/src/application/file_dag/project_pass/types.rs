//! The types of the `project_pass` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::application::file_dag::frontiers::types::{
    HandedOver, ProvenFrontier, ReadinessProof, Withholdings,
};
use crate::application::file_dag::program::admission::types::RequiredPartitions;
use crate::application::file_dag::program::project_phase::types::{
    HandoverDeclaration, PhasePartials,
};
use crate::application::file_dag::publications::types::{BoundTables, PublicationSlot};
use ryo_execution_domain::domain::coverage::types::{Partials, RuleIndex};
use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::files::types::FilePath;
use ryo_language::domain::symbols::types::Symbols;
use ryo_operators::domain::physical_plan::types::file_buffers::value::FileBuffers;
use ryo_values::domain::rows::types::Table;
use ryo_values::domain::stored_facts::types::ArenaBytes;

/// One declared occurrence of a phase as the pass runs it: its rule, its publication slot, what its frontier is handed over
/// for and the tables its bound observations are bound to.
#[derive(Debug)]
pub(crate) struct PhaseOccurrence {
    /// The occurrence's rule; a rule may occur more than once.
    pub(crate) rule: RuleIndex,
    /// The publication slot it reserved.
    pub(crate) slot: PublicationSlot,
    /// Whether its frontier is handed over for its observations, with each entry's readiness, or for its diagnostic.
    pub(crate) ready: Option<ReadinessProof>,
    /// The tables its bound observations are bound to.
    pub(crate) bound: BoundTables,
}

/// One occurrence once its frontier was handed over and its caller gave its partials.
#[derive(Clone, Copy, Debug)]
pub(crate) struct OccurrenceRun<'a> {
    /// The rule.
    pub(crate) rule: RuleIndex,
    /// The partials its answer sees.
    pub(crate) evaluation: &'a Partials,
    /// The partials its publication sees.
    pub(crate) publication: &'a Partials,
}

/// The host's rows of each declared relation, interned once per phase in the phase's symbols, by the relation's place.
#[derive(Debug, Default)]
pub(crate) struct HostedTables(pub(crate) Vec<Table>);

/// One rule's handover, kept for the later occurrences of the rule handed over for the same readiness.
pub(crate) struct RuleHandover {
    /// The rule.
    pub(crate) rule: RuleIndex,
    /// The readiness it was handed over for; none for the diagnostic.
    pub(crate) ready: Option<ReadinessProof>,
    /// Where its handover stands: proven, decoded or released.
    pub(crate) handed: HandoverState,
}

/// Where one rule's handover stands in a phase: proven before any occurrence evaluates, decoded at the first occurrence of the
/// rule that evaluates, and released after the rule's last declared occurrence.
pub(crate) enum HandoverState {
    /// The store was read and every proof checked; no entry is decoded yet.
    Proven {
        /// What the proof left of each entry.
        proven: ProvenFrontier,
        /// The files the rule is handed over from.
        required: RequiredPartitions,
    },
    /// What the files handed over, decoded into the phase's symbols.
    Handed(HandedOver),
    /// The rule has no occurrence left in the phase: its handover is gone.
    Released,
}

/// What a phase holds from its first occurrence to its last: its nodes' rows, the rules' handovers, why each withheld input is
/// withheld and the host's rows.
#[derive(Default)]
pub(crate) struct PhaseState {
    /// The nodes' rows.
    pub(crate) buffers: FileBuffers,
    /// Each rule's handover, by its first occurrence.
    pub(crate) handovers: Vec<RuleHandover>,
    /// Why each withheld input is withheld.
    pub(crate) withholdings: Withholdings,
    /// The host's rows, once interned.
    pub(crate) hosted: Option<HostedTables>,
    /// The handover each rule's nodes last ran over, by rule: a rule handed over again for another readiness runs its nodes
    /// again.
    pub(crate) last: Vec<LastHandover>,
    /// The files each rule was handed over from, as the phase first declared them: kept across the phase's calls.
    pub(crate) declared: Vec<HandoverDeclaration>,
    /// The occurrences each rule has still to evaluate in the phase.
    pub(crate) uses: RuleUses,
}

/// How many declared occurrences of each declared rule are still to evaluate in a phase, one entry per rule in first-declared
/// order: sized by the phase's rules, never by the request's. It counts every declared occurrence, so one that never evaluates
/// (a refused binding, a stop) leaves its rule counted: a count only ever runs late, never early.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct RuleUses(pub(crate) Vec<RuleUse>);

/// One rule's occurrences still to evaluate in a phase.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RuleUse {
    /// The rule.
    pub(crate) rule: RuleIndex,
    /// Its occurrences still to evaluate.
    pub(crate) left: OccurrencesLeft,
}

/// How many declared occurrences of a rule a phase has still to evaluate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct OccurrencesLeft(pub(crate) u32);

/// The place of one rule's handover among a phase's handovers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct HandoverPlace(pub(crate) usize);

/// The handover a rule's nodes last ran over.
#[derive(Clone, Copy, Debug)]
pub(crate) struct LastHandover {
    /// The rule.
    pub(crate) rule: RuleIndex,
    /// The handover its nodes last ran over.
    pub(crate) place: HandoverPlace,
}

/// An occurrence once its rule was handed over and its caller gave its partials.
#[derive(Debug)]
pub(crate) struct ReadyOccurrence {
    /// Its rule's handover.
    pub(crate) handover: HandoverPlace,
    /// The partials its caller gave.
    pub(crate) partials: PhasePartials,
}

/// Whether a step after the files read a relation no handover answers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HandoverReads {
    /// Every relation it read was handed over.
    Handed,
    /// It read one no handover answers.
    Unhanded,
}

/// What the steps after the files read besides each other's rows: the occurrence's handover (the files' rows and the project's
/// relations), its bound observations and the host's rows.
pub(crate) struct PhaseInputs<'p> {
    /// What the files handed over to the occurrence's rule.
    pub(crate) handed: &'p HandedOver,
    /// The tables its bound observations are bound to.
    pub(crate) bound: &'p BoundTables,
    /// The host's rows.
    pub(crate) hosted: &'p HostedTables,
    /// The rule's frontier: a bound observation on it whose producer the stage withholds is withheld wherever it is read.
    pub(crate) frontier: &'p [ryo_values::domain::plan_parts::types::Input],
    /// Whether a step read a relation no handover answers.
    pub(crate) unhanded: std::cell::Cell<HandoverReads>,
}

/// What the steps after the files borrow besides rows: the phase's symbols, the paths of the run's files and the arenas.
pub(crate) struct PhaseResources<'r> {
    /// The phase's symbols, where a made text is interned.
    pub(crate) symbols: &'r mut Symbols,
    /// The path of each file of the run.
    pub(crate) paths: &'r [FilePath],
    /// The arenas the stage's texts are in.
    pub(crate) arenas: &'r [ArenaBytes],
    /// No file is open after the files: an empty tree.
    pub(crate) elements: &'r Elements,
}

/// What a batch of a phase reads besides its occurrences: the program, the files it is scoped over, the facts the frontiers
/// are read from, the project's relations and the observer.
pub(crate) struct PhaseBatch<'b, 's> {
    /// The request's prepared program.
    pub(crate) program:
        &'b crate::application::file_dag::program::preparation::types::PreparedProgram,
    /// The program the file pass runs.
    pub(crate) pass: &'b crate::application::file_dag::file_pass::types::PassProgram,
    /// The request's execution: its files and services.
    pub(crate) execution:
        &'b crate::application::file_dag::program::request_execution::types::RequestExecution,
    /// What the files are scoped by.
    pub(crate) scoping: crate::application::file_dag::frontiers::types::PhaseScoping<'b>,
    /// The first bucket of each rule's frontier, by rule.
    pub(crate) first: &'b [ryo_values::domain::stored_facts::types::Bucket],
    /// Finishes the facts the frontiers are read from.
    pub(crate) facts: &'b dyn crate::outbound::facts::interface::FactsFinish,
    /// The project's relations.
    pub(crate) project: &'b crate::application::file_dag::project_source::types::ProjectSource<'s>,
    /// The arenas the stage's texts are in.
    pub(crate) arenas: &'b [ArenaBytes],
    /// Where each rule's steps are reported, when the request has an observer.
    pub(crate) observer: Option<&'b dyn crate::outbound::observer::interface::ObserverPort>,
    /// The life of the stage the batch runs in: what it publishes holds it.
    pub(crate) stage: &'b std::sync::Arc<()>,
}

/// A step after the files that answers from the absence of rows, with the coverage of its operands: its rows the missing ones
/// could change are dropped where it stands.
#[derive(Debug)]
pub(crate) struct SettledStep {
    /// The rule's step.
    pub(crate) step: ryo_values::domain::plan_parts::types::StepId,
    /// Each operand's cell kinds, coverage and lineage, in operand order.
    pub(crate) operands:
        Vec<crate::application::file_dag::file_pass::service::nodes::service::StepCoverage>,
    /// The operands whose absence of rows the step trusts.
    pub(crate) absence: ryo_execution_domain::domain::operator::contract::types::AbsenceMask,
}
