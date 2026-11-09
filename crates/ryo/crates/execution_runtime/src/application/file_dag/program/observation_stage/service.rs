//! An observation stage: its reservations, installs and restored frontiers.
use crate::application::file_dag::frontiers::types::BoundInput;
use crate::application::file_dag::frontiers::types::InputReadiness;
use crate::application::file_dag::frontiers::types::PublicationSlot;
use crate::application::file_dag::frontiers::types::ReadinessProof;
use crate::application::file_dag::observations::types::RuleSelection;
use crate::application::file_dag::program::observation_stage::interface::ObservationExecution;
use crate::application::file_dag::program::observation_stage::interface::ObservationPublication;
use crate::application::file_dag::program::observation_stage::types::ObservationStage;
use crate::application::file_dag::program::project_phase::interface::{
    ProjectRootExecution, ProjectStage,
};
use crate::application::file_dag::program::observation_stage::types::{
    AdmittedOutput,
    Install, ObservationBatches,
};
use crate::application::file_dag::program::project_phase::service::reserved;
use crate::application::file_dag::program::project_phase::types::{
    FrontierDemand, OccurrenceNumber, PhaseOccurrence, PhaseOccurrences, PhasePartials, PhaseRules,
    PhaseStop,
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
use ryo_execution_domain::domain::readiness::types::ReadinessCounts;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::WorkerNumber;
use ryo_values::domain::stored_facts::types::ArenaBytes;
use ryo_values::domain::files::types::CorpusInput;
use ryo_values::domain::records::types::CrossPublicationTransfer;
use ryo_values::domain::runs::types::Skipped;
use ryo_values::domain::stored_facts::types::Bucket;
impl ProjectRootExecution for ObservationStage<'_> {
    fn evaluate_occurrences<H, T, E>(
        &self,
        phase: PhaseOccurrences<'_>,
        symbols: &mut Symbols,
        before: impl FnMut(OccurrenceNumber) -> Result<H, E>,
        partials: impl FnMut(OccurrenceNumber) -> Result<PhasePartials, E>,
        guard: impl FnMut(RuleIndex) -> Result<(), E>,
        complete: impl FnMut(
            OccurrenceNumber,
            RuleOutcome<ProjectOutputOutcome>,
            PublicationSlot,
            &mut Symbols,
        ) -> Result<T, E>,
    ) -> ProjectBatch<Result<T, PhaseStop<E>>, E> {
        self.phase
            .evaluate_occurrences(phase, symbols, before, partials, guard, complete)
    }
    fn evaluate_rules<H, T, E>(
        &self,
        phase: PhaseRules<'_>,
        symbols: &mut Symbols,
        before: impl FnMut(OccurrenceNumber) -> Result<H, E>,
        partials: impl FnMut(OccurrenceNumber) -> Result<PhasePartials, E>,
        guard: impl FnMut(RuleIndex) -> Result<(), E>,
        complete: impl FnMut(
            OccurrenceNumber,
            RuleOutcome<ProjectOutputOutcome>,
            &[ArenaBytes],
            Option<WorkerNumber>,
            &mut Symbols,
        ) -> Result<T, E>,
    ) -> ProjectBatch<Result<T, PhaseStop<E>>, E> {
        self.phase
            .evaluate_rules(phase, symbols, before, partials, guard, complete)
    }
    fn readiness_counts(&self) -> ReadinessCounts {
        self.phase.readiness_counts()
    }
}
impl ObservationExecution for ObservationStage<'_> {
    fn reserve(&mut self, rule: RuleIndex) -> Result<PublicationSlot, OperatingError> {
        self.publications.reserve(rule)
    }
    fn bindings(&self, rule: RuleIndex) -> Result<Vec<BoundInput>, OperatingError> {
        self.publications.bindings(rule)
    }
    fn binding_readiness(
        &self,
        rule: RuleIndex,
    ) -> Result<
        std::collections::BTreeMap<
            ryo_values::domain::plan_parts::types::ObservationBindingId,
            InputReadiness,
        >,
        OperatingError,
    > {
        self.publications.binding_readiness(rule)
    }
    fn frontier_readiness(&self, rule: RuleIndex) -> Result<ReadinessProof, OperatingError> {
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
    fn arenas(&self) -> &[ryo_values::domain::stored_facts::types::ArenaBytes] {
        self.publications.arenas()
    }
    fn finish(&mut self, rule: RuleIndex) -> Result<(), OperatingError> {
        self.publications.finish(rule)
    }
    fn finish_stage(self) -> Result<(), OperatingError> {
        drop(self.publications);
        self.phase.finish()
    }
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
        for (_, rule) in batches.occurrences {
            *selected
                .get_mut(rule.0)
                .ok_or(OperatingError::NotRun)? = RuleSelection::Selected;
        }
        for (rule, selected) in selected.iter().enumerate() {
            if *selected == RuleSelection::Unselected {
                self.publications.finish(RuleIndex(rule))?;
            }
        }
        self.restore(&batches, &selected)?;
        let mut start = 0;
        while let Some((batch, _)) = batches.occurrences.get(start) {
            // Each declared occurrence of the batch in declaration order, numbered by its place among all of them.
            let end = start
                + batches.occurrences[start..]
                    .iter()
                    .take_while(|(each, _)| each == batch)
                    .count();
            let declared = &batches.occurrences[start..end];
            let bindings = declared
                .iter()
                .map(|(_, rule)| self.bind(*rule))
                .collect::<Vec<_>>();
            let (results, refused) = {
                // The occurrences whose bindings hold reach the phase; a refused binding stays with its occurrence below.
                let submitted = (start..end)
                    .zip(declared)
                    .zip(&bindings)
                    .filter_map(|((number, (_, rule)), binding)| match binding {
                        Ok(binding) => Some((OccurrenceNumber(number), *rule, binding)),
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
                let arenas = self.publications.arenas();
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
                        complete(declared_number(at)?, outcome, arenas, reserved(slot), symbols)
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
            for ((number, (_, rule)), binding) in (start..end).zip(declared).zip(bindings) {
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
                    let delivered = deliver(OccurrenceNumber(number), outcome).and_then(|install| {
                        match (install, slot) {
                            (None, _) => Ok(()),
                            (Some(install), Some(slot)) => self
                                .publications
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
                if let Err(error) = self.publications.finish(*rule) {
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
    /// What binds one declared occurrence: the readiness of its frontiers, the publication slot it reserves and its bound inputs,
    /// in that order.
    pub(crate) fn bind(&mut self, rule: RuleIndex) -> Result<Binding, OperatingError> {
        let ready = self.frontier_readiness(rule)?;
        let slot = self.publications.reserve(rule)?;
        let bound = self.publications.bindings(rule)?;
        Ok(Binding { ready, slot, bound })
    }
    /// The frontiers of the selected rules restored over what the batches read.
    fn restore(
        &mut self,
        batches: &ObservationBatches<'_>,
        selected: &[RuleSelection],
    ) -> Result<(), OperatingError> {
        crate::application::file_dag::observations::restore::service::restore_frontiers(
            &mut self.publications,
            &self.phase.request.program,
            &self.phase.request.services,
            &self.phase.publication,
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
pub(crate) struct Binding {
    ready: ReadinessProof,
    slot: PublicationSlot,
    bound: Vec<BoundInput>,
}
impl ObservationPublication for ObservationStage<'_> {
    fn install(
        &mut self,
        rule: RuleIndex,
        slot: PublicationSlot,
        transfer: CrossPublicationTransfer,
        publications: Vec<AdmittedOutput>,
    ) -> Result<(), OperatingError> {
        self.publications
            .install(rule, slot, transfer, publications)
    }
    fn restore_frontiers(
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
}
