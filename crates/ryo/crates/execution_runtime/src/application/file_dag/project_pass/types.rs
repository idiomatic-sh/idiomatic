//! The types of the `project_pass` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::application::file_dag::frontiers::types::{HandedOver, ReadinessProof, Withholdings};
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
#[derive(Debug)]
pub(crate) struct RuleHandover {
    /// The rule.
    pub(crate) rule: RuleIndex,
    /// The readiness it was handed over for; none for the diagnostic.
    pub(crate) ready: Option<ReadinessProof>,
    /// What the files handed over.
    pub(crate) handed: HandedOver,
}

/// What a phase holds from its first occurrence to its last: its nodes' rows, the rules' handovers, why each withheld input is
/// withheld and the host's rows.
#[derive(Debug, Default)]
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
    pub(crate) last: Vec<(RuleIndex, usize)>,
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
    pub(crate) unhanded: std::cell::Cell<bool>,
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
    pub(crate) program: &'b crate::application::file_dag::program::preparation::types::PreparedProgram,
    /// The program the file pass runs.
    pub(crate) pass: &'b crate::application::file_dag::file_pass::types::PassProgram,
    /// The request's files.
    pub(crate) request: &'b std::sync::Arc<crate::application::file_dag::program::request_identity::service::RequestIdentity>,
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
