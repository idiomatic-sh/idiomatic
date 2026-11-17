//! An observation stage: its declared batches run behind one call through the project pass, with the stage's publications.
use super::types::ObservationStage;
use crate::application::file_dag::frontiers::types::{
    InputReadiness, PhaseScoping, ReadinessProof,
};
use crate::application::file_dag::program::observation_stage::interface::ProgramObservationStagePort;
use crate::application::file_dag::program::observation_stage::types::{
    BatchOccurrence, Install, ObservationBatches,
};
use crate::application::file_dag::program::project_phase::interface::ProgramProjectPhasePort;
use crate::application::file_dag::program::project_phase::service::reserved;
use crate::application::file_dag::program::project_phase::types::{
    OccurrenceNumber, PhasePartials, PhaseStop,
};
use crate::application::file_dag::program::rule_outcome::types::ProjectBatch;
use crate::application::file_dag::program::rule_outcome::types::ProjectOutputOutcome;
use crate::application::file_dag::program::rule_outcome::types::RuleOutcome;
use crate::application::file_dag::project_pass::service::{project_reads, run_batch, uses_of};
use crate::application::file_dag::project_pass::types::PhaseBatch;
use crate::application::file_dag::project_pass::types::{PhaseOccurrence, PhaseState};
use crate::application::file_dag::project_source::service::opened;
use crate::application::file_dag::project_source::types::ProjectWritten;
use crate::application::file_dag::publications::types::RuleSelection;
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::WorkerNumber;
use ryo_values::domain::stored_facts::types::ArenaBytes;

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
                self.publications.finish(RuleIndex(rule))?;
            }
        }
        let request = self.phase.request;
        let scoping = PhaseScoping {
            corpus: batches.corpus,
            covered: batches.covered,
            skipped: batches.skipped,
            inapplicable: batches.inapplicable,
            refused: batches.refused,
        };
        self.publications.restore(
            request,
            batches.facts,
            scoping,
            batches.readiness,
            batches.first,
            &selected,
        )?;
        // The project's relations are read once for the stage, at its first admitted occurrence, every one its declared rules
        // read after the files.
        let declared_rules = batches
            .occurrences
            .iter()
            .map(|occurrence| occurrence.rule)
            .collect::<Vec<_>>();
        let project = opened(
            ProjectWritten {
                rows: batches.project,
                symbols: batches.project_symbols,
            },
            project_reads(&request.pass, &declared_rules),
        );
        let mut state = PhaseState {
            uses: uses_of(batches.occurrences.iter().map(|occurrence| occurrence.rule)),
            ..PhaseState::default()
        };
        let mut start = 0;
        while let Some(BatchOccurrence { batch, .. }) = batches.occurrences.get(start) {
            // Each declared occurrence of the batch in declaration order, numbered by its place among all of them.
            let end = start
                + batches.occurrences[start..]
                    .iter()
                    .take_while(|each| each.batch == *batch)
                    .count();
            let declared = &batches.occurrences[start..end];
            let mut bindings = Vec::with_capacity(declared.len());
            for occurrence in declared {
                bindings.push(self.bind(occurrence.rule, symbols, &mut state));
            }
            let (results, refused, slots) = {
                // The occurrences whose bindings hold reach the pass; a refused binding stays with its occurrence below.
                let mut numbers = Vec::new();
                let mut occurrences = Vec::new();
                let mut slots = Vec::with_capacity(bindings.len());
                for ((number, occurrence), binding) in (start..end).zip(declared).zip(bindings) {
                    match binding {
                        Ok(bound) => {
                            slots.push(Ok(bound.slot));
                            numbers.push(OccurrenceNumber(number));
                            occurrences.push(PhaseOccurrence {
                                rule: occurrence.rule,
                                ..bound
                            });
                        }
                        Err(error) => slots.push(Err(error)),
                    }
                }
                // The pass numbers the submitted occurrences among themselves; each callback is given the declared number.
                let declared_number = |submitted: OccurrenceNumber| {
                    numbers
                        .get(submitted.0)
                        .copied()
                        .ok_or(OperatingError::NotRun)
                };
                let arenas = self.publications.arenas();
                let batch = PhaseBatch {
                    program: &request.program,
                    pass: &request.pass,
                    execution: request,
                    scoping,
                    first: batches.first,
                    facts: batches.facts,
                    project: &project,
                    arenas,
                    observer: request.services.observer(),
                    stage: &self.publications.stage,
                };
                let ran = run_batch(
                    &batch,
                    &mut state,
                    &occurrences,
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
                let (results, refused) = match ran {
                    ProjectBatch::Complete(completed) => (completed, None),
                    ProjectBatch::Refused { completed, error } => (completed, Some(error)),
                };
                (results, refused, slots)
            };
            let mut first_error = None;
            let mut outcomes = results.into_iter();
            // Past the guard's refusal no occurrence has an outcome to deliver; every one is still retired.
            let mut delivering = true;
            for ((number, BatchOccurrence { rule, .. }), slot) in
                (start..end).zip(declared).zip(slots)
            {
                let (outcome, slot) = match slot {
                    Err(error) if delivering => (Some(Err(PhaseStop::Unbound(error))), None),
                    Err(_) => (None, None),
                    Ok(slot) => {
                        let outcome = outcomes.next();
                        delivering = outcome.is_some();
                        (outcome, Some(slot))
                    }
                };
                if let Some(outcome) = outcome {
                    let delivered =
                        deliver(OccurrenceNumber(number), outcome).and_then(|install| {
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
                        let _: &mut E = first_error.get_or_insert(error);
                    }
                }
                if let Err(error) = self.publications.finish(*rule) {
                    let _: &mut E = first_error.get_or_insert(E::from(error));
                }
            }
            if let Some(error) = refused {
                let _: &mut E = first_error.get_or_insert(error);
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
    /// What binds one declared occurrence before it reaches the pass, in this order: the readiness of its rule's frontier, the
    /// publication slot it reserves and the tables its bound observations are bound to.
    pub(crate) fn bind(
        &mut self,
        rule: RuleIndex,
        symbols: &mut Symbols,
        state: &mut PhaseState,
    ) -> Result<PhaseOccurrence, OperatingError> {
        let ready = self.frontier_readiness(rule)?;
        let slot = self.reserve(rule)?;
        let bound = self.publications.bindings(
            &self.phase.request.pass,
            rule,
            symbols,
            &mut state.withholdings,
        )?;
        Ok(PhaseOccurrence {
            rule,
            slot,
            ready: Some(ready),
            bound,
        })
    }

    /// The readiness of each retained frontier of `rule`, from the bindings each frontier depends on: a frontier is ready only
    /// when every one of them is published in this stage.
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

    /// Reserves the arena an occurrence of `rule` publishes into after the files, when it publishes anything there.
    pub(crate) fn reserve(
        &mut self,
        rule: RuleIndex,
    ) -> Result<crate::application::file_dag::publications::types::PublicationSlot, OperatingError>
    {
        self.publications.reserve(rule)
    }

    /// Ends the stage: its publications go back as found, then its phase finishes.
    pub(crate) fn finish_stage(self) -> Result<(), OperatingError> {
        drop(self.publications);
        self.phase.finish()
    }
}
