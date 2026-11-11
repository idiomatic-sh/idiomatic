//! An observation stage: its declared batches run behind one call, with its reservations, installs and restored frontiers.
use crate::application::file_dag::frontiers::types::BoundInput;
use crate::application::file_dag::frontiers::types::InputReadiness;
use crate::application::file_dag::frontiers::types::PublicationSlot;
use crate::application::file_dag::frontiers::types::ReadinessProof;
use crate::application::file_dag::observations::types::RuleSelection;
use crate::application::file_dag::program::observation_stage::interface::ProgramObservationStagePort;
use crate::application::file_dag::program::observation_stage::types::{
    AdmittedOutput, BatchOccurrence, Install, ObservationBatches,
};
use crate::application::file_dag::program::project_phase::interface::ProgramProjectPhasePort;
use crate::application::file_dag::program::project_phase::service::ProjectPhase;
use crate::application::file_dag::program::project_phase::service::reserved;
use crate::application::file_dag::program::project_phase::types::{
    FrontierDemand, OccurrenceNumber, PhaseOccurrence, PhaseOccurrences, PhasePartials, PhaseStop,
};
use crate::application::file_dag::program::rule_outcome::types::ProjectBatch;
use crate::application::file_dag::program::rule_outcome::types::ProjectOutputOutcome;
use crate::application::file_dag::program::rule_outcome::types::RuleOutcome;
use crate::outbound::facts::interface::FactsFinish;
use ryo_execution_domain::domain::coverage::types::Covered;
use ryo_execution_domain::domain::coverage::types::FileObservationReadiness;
use ryo_execution_domain::domain::coverage::types::InapplicableFiles;
use ryo_execution_domain::domain::coverage::types::RefusedFiles;
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::WorkerNumber;
use ryo_values::domain::files::types::CorpusInput;
use ryo_values::domain::records::types::CrossPublicationTransfer;
use ryo_values::domain::runs::types::Skipped;
use ryo_values::domain::stored_facts::types::ArenaBytes;
use ryo_values::domain::stored_facts::types::Bucket;
impl ProgramObservationStagePort for ObservationStage<'_> {
    fn run_batches<H, T, E: From<OperatingError>>(
        mut self,
        batches: ObservationBatches<'_>,
        symbols: &mut Symbols,
        mut before: impl FnMut(OccurrenceNumber) -> Result<H, E>,
        mut partials: impl FnMut(OccurrenceNumber) -> Result<PhasePartials, E>,
        mut guard: impl FnMut(RuleIndex) -> Result<(), E>,
        mut complete: impl FnMut(
            OccurrenceNumber,
            RuleOutcome<ProjectOutputOutcome>,
            &[ArenaBytes],
            Option<WorkerNumber>,
            &mut Symbols,
        ) -> Result<T, E>,
        mut deliver: impl FnMut(OccurrenceNumber, Result<T, PhaseStop<E>>) -> Result<Option<Install>, E>,
    ) -> Result<(), E> {
        let mut selected = vec![RuleSelection::Unselected; batches.rules.len()];
        for occurrence in batches.occurrences {
            *selected
                .get_mut(occurrence.rule.0)
                .ok_or(OperatingError::NotRun)? = RuleSelection::Selected;
        }
        for (rule, selected) in selected.iter().enumerate() {
            if *selected == RuleSelection::Unselected {
                self.finish(RuleIndex(rule))?;
            }
        }
        self.restore(&batches, &selected)?;
        let mut start = 0;
        while let Some(BatchOccurrence { batch, .. }) = batches.occurrences.get(start) {
            // Each declared occurrence of the batch in declaration order, numbered by its place among all of them.
            let end = start
                + batches.occurrences[start..]
                    .iter()
                    .take_while(|each| each.batch == *batch)
                    .count();
            let declared = &batches.occurrences[start..end];
            let bindings = declared
                .iter()
                .map(|occurrence| self.bind(occurrence.rule))
                .collect::<Vec<_>>();
            let (results, refused) = {
                // The occurrences whose bindings hold reach the phase; a refused binding stays with its occurrence below.
                let submitted = (start..end)
                    .zip(declared)
                    .zip(&bindings)
                    .filter_map(|((number, occurrence), binding)| match binding {
                        Ok(binding) => Some((OccurrenceNumber(number), occurrence.rule, binding)),
                        Err(_) => None,
                    })
                    .collect::<Vec<_>>();
                let occurrences = submitted
                    .iter()
                    .map(|(_, rule, binding)| PhaseOccurrence {
                        rule: *rule,
                        bound: &binding.bound,
                        slot: binding.slot,
                        demand: FrontierDemand::Observations {
                            ready: &binding.ready,
                        },
                    })
                    .collect::<Vec<_>>();
                // The evaluation numbers the submitted occurrences among themselves; each callback is given the declared number.
                let declared_number = |submitted_at: OccurrenceNumber| {
                    submitted
                        .get(submitted_at.0)
                        .map(|(number, _, _)| *number)
                        .ok_or(OperatingError::NotRun)
                };
                let arenas = self.arenas();
                let ran = self.phase.evaluate_occurrences(
                    PhaseOccurrences {
                        occurrences: &occurrences,
                        first: batches.first,
                        corpus: batches.corpus,
                        covered: batches.covered,
                        skipped: batches.skipped,
                        inapplicable: batches.inapplicable,
                        refused: batches.refused,
                        facts: batches.facts,
                        project: batches.project,
                        project_symbols: batches.project_symbols,
                        arenas,
                    },
                    symbols,
                    |at| before(declared_number(at)?),
                    |at| partials(declared_number(at)?),
                    &mut guard,
                    |at, outcome, slot, symbols: &mut Symbols| {
                        complete(
                            declared_number(at)?,
                            outcome,
                            arenas,
                            reserved(slot),
                            symbols,
                        )
                    },
                );
                match ran {
                    ProjectBatch::Complete(completed) => (completed, None),
                    ProjectBatch::Refused { completed, error } => (completed, Some(error)),
                }
            };
            let mut first_error = None;
            let mut outcomes = results.into_iter();
            // Past the guard's refusal no occurrence has an outcome to deliver; every one is still retired.
            let mut delivering = true;
            for ((number, BatchOccurrence { rule, .. }), binding) in
                (start..end).zip(declared).zip(bindings)
            {
                let (outcome, slot) = match binding {
                    Err(error) if delivering => (Some(Err(PhaseStop::Unbound(error))), None),
                    Err(_) => (None, None),
                    Ok(binding) => {
                        let outcome = outcomes.next();
                        delivering = outcome.is_some();
                        (outcome, Some(binding.slot))
                    }
                };
                if let Some(outcome) = outcome {
                    let delivered =
                        deliver(OccurrenceNumber(number), outcome).and_then(|install| {
                            match (install, slot) {
                                (None, _) => Ok(()),
                                (Some(install), Some(slot)) => self
                                    .install(*rule, slot, install.transfer, install.publications)
                                    .map_err(E::from),
                                // An occurrence whose binding was refused reserved no slot to install in.
                                (Some(_), None) => Err(E::from(OperatingError::NotRun)),
                            }
                        });
                    if let Err(error) = delivered {
                        first_error.get_or_insert(error);
                    }
                }
                if let Err(error) = self.finish(*rule) {
                    first_error.get_or_insert(E::from(error));
                }
            }
            if let Some(error) = refused {
                first_error.get_or_insert(error);
            }
            if let Some(error) = first_error {
                let _finished = self.finish_stage();
                return Err(error);
            }
            start = end;
        }
        self.finish_stage()?;
        Ok(())
    }
}
impl ObservationStage<'_> {
    pub(crate) fn reserve(&mut self, rule: RuleIndex) -> Result<PublicationSlot, OperatingError> {
        self.publications.reserve(rule)
    }
    pub(crate) fn bindings(&self, rule: RuleIndex) -> Result<Vec<BoundInput>, OperatingError> {
        self.publications.bindings(rule)
    }
    /// The readiness of each retained frontier of `rule`, from the bindings each frontier depends on: a frontier is ready only when
    /// every one of them is published in this stage.
    pub(crate) fn frontier_readiness(
        &self,
        rule: RuleIndex,
    ) -> Result<ReadinessProof, OperatingError> {
        let readiness = self.publications.binding_readiness(rule)?;
        let dependencies = self.phase.request.program.frontier_dependencies(rule)?;
        Ok(ReadinessProof(
            dependencies
                .iter()
                .map(|inputs| {
                    if inputs
                        .iter()
                        .all(|input| readiness.get(input).copied() == Some(InputReadiness::Ready))
                    {
                        InputReadiness::Ready
                    } else {
                        InputReadiness::Unavailable
                    }
                })
                .collect(),
        ))
    }
    pub(crate) fn arenas(&self) -> &[ArenaBytes] {
        self.publications.arenas()
    }
    pub(crate) fn finish(&mut self, rule: RuleIndex) -> Result<(), OperatingError> {
        self.publications.finish(rule)
    }
    pub(crate) fn finish_stage(self) -> Result<(), OperatingError> {
        drop(self.publications);
        self.phase.finish()
    }
    pub(crate) fn install(
        &mut self,
        rule: RuleIndex,
        slot: PublicationSlot,
        transfer: CrossPublicationTransfer,
        publications: Vec<AdmittedOutput>,
    ) -> Result<(), OperatingError> {
        self.publications
            .install(rule, slot, transfer, publications)
    }
    #[expect(
        clippy::too_many_arguments,
        reason = "the restoration reads each of the phase's scoping inputs, as the frontiers' restoration does"
    )]
    pub(crate) fn restore_frontiers(
        &mut self,
        facts: &dyn FactsFinish,
        corpus: &[CorpusInput],
        covered: &[Covered],
        skipped: &[Skipped],
        inapplicable: &[InapplicableFiles],
        refused: &[RefusedFiles],
        frames: &[FileObservationReadiness],
        first_bucket: &[Bucket],
        selected: &[RuleSelection],
    ) -> Result<(), OperatingError> {
        crate::application::file_dag::observations::restore::service::restore_frontiers(
            &mut self.publications,
            &self.phase.request.program,
            &self.phase.request.services,
            &self.phase.publication,
            facts,
            corpus,
            covered,
            skipped,
            inapplicable,
            refused,
            frames,
            first_bucket,
            selected,
        )
    }
    /// What binds one declared occurrence: the readiness of its frontiers, the publication slot it reserves and its bound inputs,
    /// in that order.
    pub(crate) fn bind(&mut self, rule: RuleIndex) -> Result<OccurrenceBinding, OperatingError> {
        let ready = self.frontier_readiness(rule)?;
        let slot = self.reserve(rule)?;
        let bound = self.bindings(rule)?;
        Ok(OccurrenceBinding { ready, slot, bound })
    }
    /// The frontiers of the selected rules restored over what the batches read.
    fn restore(
        &mut self,
        batches: &ObservationBatches<'_>,
        selected: &[RuleSelection],
    ) -> Result<(), OperatingError> {
        self.restore_frontiers(
            batches.facts,
            batches.corpus,
            batches.covered,
            batches.skipped,
            batches.inapplicable,
            batches.refused,
            batches.readiness,
            batches.first,
            selected,
        )
    }
}
/// What one declared occurrence was bound with before it reaches the phase.
pub(crate) struct OccurrenceBinding {
    ready: ReadinessProof,
    slot: PublicationSlot,
    bound: Vec<BoundInput>,
}

/// One project publication generation shared by every ready batch.
pub struct ObservationStage<'a> {
    pub(crate) phase: ProjectPhase<'a>,
    pub(crate) publications:
        crate::application::file_dag::observations::types::CrossPublications<'a>,
}
