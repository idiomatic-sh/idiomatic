//! A project phase: its declared occurrences run after the files through the project pass, in declaration order.
use super::types::ProjectPhase;
use crate::application::file_dag::frontiers::types::PhaseScoping;
use crate::application::file_dag::program::project_phase::interface::ProgramProjectPhasePort;
use crate::application::file_dag::program::project_phase::interface::ProjectRootExecution;
use crate::application::file_dag::program::project_phase::types::OccurrenceNumber;
use crate::application::file_dag::program::project_phase::types::PhasePartials;
use crate::application::file_dag::program::project_phase::types::PhaseRules;
use crate::application::file_dag::program::project_phase::types::PhaseStop;
use crate::application::file_dag::program::rule_outcome::types::ProjectBatch;
use crate::application::file_dag::program::rule_outcome::types::ProjectOutputOutcome;
use crate::application::file_dag::program::rule_outcome::types::RuleOutcome;
use crate::application::file_dag::project_pass::service::{project_reads, run_batch};
use crate::application::file_dag::project_pass::types::PhaseBatch;
use crate::application::file_dag::project_pass::types::{PhaseOccurrence, PhaseState};
use crate::application::file_dag::project_source::service::opened;
use crate::application::file_dag::project_source::types::ProjectWritten;
use crate::application::file_dag::publications::types::{BoundTables, PublicationSlot};
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::WorkerNumber;
use ryo_values::domain::stored_facts::types::ArenaBytes;

impl ProjectRootExecution for ProjectPhase<'_> {
    fn evaluate_rules<H, T, E>(
        &self,
        phase: PhaseRules<'_>,
        symbols: &mut Symbols,
        before: impl FnMut(OccurrenceNumber) -> Result<H, E>,
        partials: impl FnMut(OccurrenceNumber) -> Result<PhasePartials, E>,
        guard: impl FnMut(RuleIndex) -> Result<(), E>,
        mut complete: impl FnMut(
            OccurrenceNumber,
            RuleOutcome<ProjectOutputOutcome>,
            &[ArenaBytes],
            Option<WorkerNumber>,
            &mut Symbols,
        ) -> Result<T, E>,
    ) -> ProjectBatch<Result<T, PhaseStop<E>>, E> {
        let occurrences = phase
            .rules
            .iter()
            .map(|rule| PhaseOccurrence {
                rule: *rule,
                slot: PublicationSlot::Unused,
                ready: None,
                bound: BoundTables::default(),
            })
            .collect::<Vec<_>>();
        let request = self.request;
        // The project's relations are read once for the phase, at its first occurrence its caller admits, every one its rules
        // read after the files.
        let project = opened(
            ProjectWritten {
                rows: phase.project,
                symbols: phase.project_symbols,
            },
            project_reads(&request.pass, phase.rules),
        );
        let arenas = phase.arenas;
        let stage = std::sync::Arc::new(());
        let batch = PhaseBatch {
            program: &request.program,
            pass: &request.pass,
            execution: request,
            scoping: PhaseScoping {
                corpus: phase.corpus,
                covered: phase.covered,
                skipped: phase.skipped,
                inapplicable: phase.inapplicable,
                refused: phase.refused,
            },
            first: phase.first,
            facts: phase.facts,
            project: &project,
            arenas,
            observer: request.services.observer(),
            stage: &stage,
        };
        // A call holds nothing of the earlier ones but the files each rule was declared to be handed over from.
        let mut state = PhaseState {
            declared: std::mem::take(&mut *self.declared.borrow_mut()),
            ..PhaseState::default()
        };
        let ran = run_batch(
            &batch,
            &mut state,
            &occurrences,
            symbols,
            before,
            partials,
            guard,
            |occurrence, outcome, slot, symbols: &mut Symbols| {
                complete(occurrence, outcome, arenas, reserved(slot), symbols)
            },
        );
        *self.declared.borrow_mut() = std::mem::take(&mut state.declared);
        ran
    }
}

/// The worker a publication slot reserved, when it reserved one.
pub(crate) fn reserved(slot: PublicationSlot) -> Option<WorkerNumber> {
    match slot {
        PublicationSlot::Unused => None,
        PublicationSlot::Reserved(worker) => Some(worker),
    }
}

impl ProgramProjectPhasePort for ProjectPhase<'_> {
    fn finish(self) -> Result<(), OperatingError> {
        // A phase holds nothing past its calls: every row it made went with the call that made it.
        Ok(())
    }
}
