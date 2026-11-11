//! Binding intents and preparing the shared walk of one file.
use crate::application::file_dag::program::admission::service::ApplicabilityProof;
use crate::application::file_dag::program::admission::service::Inputs;
use crate::application::file_dag::program::admission::service::Partition;
use crate::application::file_dag::program::constants::{EXECUTION_TARGET, PRESENCE_FILE};
use crate::application::file_dag::program::constants::{
    METADATA_UNAVAILABLE, PRODUCER_COVERAGE_INCOMPLETE, SOURCE_FACTS_INCOMPLETE,
    SYNTAX_UNDETERMINED,
};
use crate::application::file_dag::program::facade_runtime::file_execution::service::FileRun;
use crate::application::file_dag::program::facade_runtime::file_execution::service::classify;
use crate::application::file_dag::program::facade_runtime::file_execution::service::readiness_of;
use crate::application::file_dag::program::facade_runtime::source_admission::service::SourceInputs;
use crate::application::file_dag::program::facade_runtime::types::{
    DeclaredIntent, FileRoot, FrontierCommit, NamedReadiness, PendingActivation, PreparedNamed,
    PresenceGate, RootCandidate, WalkPreparation,
};
use crate::application::file_dag::program::native_walk::types::Deliveries;
use ryo_execution_domain::domain::completion::types::completion_policy::value::CompletionPolicy;
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::executable_program::presence::types::RegionPresence;
use ryo_execution_domain::domain::executable_program::types::absent_roots::value::AbsentRoots;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::rule_intent::types::CaptureIntent;
use ryo_execution_domain::domain::rule_intent::types::DiagnosticDelivery;
use ryo_execution_domain::domain::rule_intent::types::FileRuleIntent;
use ryo_execution_domain::domain::rule_intent::types::ObservationDemand;
use ryo_execution_domain::domain::table_model::types::Rows;
use ryo_execution_store::application::file_dag::service::ScopePartials;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::runs::types::ApplicabilityRefusal;
use std::rc::Rc;

impl FileRun<'_> {
    /// Bind selected external readers before a source or native walk is admitted.
    pub(crate) fn bind_intents(&mut self) -> Result<Vec<RuleIndex>, OperatingError> {
        use ryo_execution_domain::domain::executable_program::file_demand::types::{
            FileDemand, FileRootDemand as DemandRoot, FrontierSelection, RootSelection,
        };
        let ScopePartials::File {
            evaluation,
            publication,
        } = &self.partials
        else {
            return Err(OperatingError::WrongOperation);
        };
        // The presence gate: off on any file with a partial read, where every root runs in full.
        let gate = if evaluation.is_empty() && publication.is_empty() {
            PresenceGate::On
        } else {
            PresenceGate::Off
        };
        let mut seen = rustc_hash::FxHashSet::default();
        let mut readers = Vec::new();
        let mut demanded = Vec::new();
        // Each run rule's prepared roots, which the presence gate may find absent.
        let mut candidates: Vec<RootCandidate> = Vec::new();
        for (index, intent) in self.intents.iter().copied().enumerate() {
            let rule = RuleIndex(index);
            let compiled = self.program.file_demand(rule)?;
            let selected = match intent {
                FileRuleIntent::Run {
                    diagnostic,
                    observations,
                    capture,
                } => FileDemand {
                    diagnostic: match diagnostic {
                        DiagnosticDelivery::Disabled => RootSelection::Undemanded,
                        DiagnosticDelivery::Collected | DiagnosticDelivery::Forwarded => {
                            RootSelection::Required
                        }
                    },
                    observations: match observations {
                        ObservationDemand::NotRequired => RootSelection::Undemanded,
                        ObservationDemand::Required => RootSelection::Required,
                    },
                    capture: match capture {
                        CaptureIntent::Disabled => FrontierSelection::Disabled,
                        CaptureIntent::Observations => FrontierSelection::Observations,
                        CaptureIntent::DiagnosticAndObservations => FrontierSelection::All,
                    },
                },
                _ => ryo_execution_domain::domain::executable_program::file_demand::constants::NO_FILE_DEMAND,
            };
            if selected != ryo_execution_domain::domain::executable_program::file_demand::constants::NO_FILE_DEMAND {
                demanded.push(rule);
            }
            if let FileRuleIntent::Run {
                diagnostic,
                observations,
                capture,
            } = intent
            {
                // One owed delivery per entry point the caller will use: rows (or
                // collected rule) and frontier capture. The walk keeps shared
                // batches until every owed delivery has finished.
                let rows = diagnostic != DiagnosticDelivery::Disabled
                    || observations == ObservationDemand::Required;
                let deliveries =
                    Deliveries(usize::from(rows) + usize::from(capture != CaptureIntent::Disabled));
                self.walk.owe(self.program, rule, deliveries);
            }
            let plan = compiled.selected(selected);
            readers.extend_from_slice(&plan.readers);
            if (selected != ryo_execution_domain::domain::executable_program::file_demand::constants::NO_FILE_DEMAND
                || matches!(
                    intent,
                    FileRuleIntent::Skipped(_) | FileRuleIntent::Refused(_)
                ))
                && compiled.outputs.0 != 0
            {
                self.roots.insert(
                    rule,
                    (0..compiled.outputs.0)
                        .map(|_| FileRoot {
                            readiness: None,
                            commit: FrontierCommit::Pending,
                            publication: None,
                            skipped: None,
                        })
                        .collect(),
                );
            }
            for &output in &plan.observations {
                let root = &self.program.rule_metadata(rule)?.observations[output.0 as usize];
                let readiness = if ryo_execution_domain::domain::trust::service::blocked(
                    &root.reads,
                    publication,
                ) == ryo_execution_domain::domain::trust::types::Blocking::Free
                {
                    NamedReadiness::Ready
                } else {
                    NamedReadiness::Blocked
                };
                self.prepared_named.push(PreparedNamed {
                    rule,
                    output,
                    readiness,
                });
            }
            for root in &plan.roots {
                let id = match root {
                    // Deferred source faults stay in compiled metadata; admission
                    // of the diagnostic/named/frontier root reports them on demand.
                    DemandRoot::Deferred(_error) => continue,
                    DemandRoot::Ordinary(id) => *id,
                    DemandRoot::Observation { region, output } => {
                        let root =
                            &self.program.rule_metadata(rule)?.observations[output.0 as usize];
                        if ryo_execution_domain::domain::trust::service::blocked(
                            &root.reads,
                            publication,
                        ) != ryo_execution_domain::domain::trust::types::Blocking::Free
                        {
                            continue;
                        }
                        *region
                    }
                };
                if seen.insert(id) {
                    self.prepared_roots.push(id);
                }
                if matches!(intent, FileRuleIntent::Run { .. }) {
                    candidates.push(RootCandidate { rule, region: id });
                }
            }
            match intent {
                FileRuleIntent::Skipped(reason) => {
                    let proof = ApplicabilityProof::for_file(rule, self.file, reason, &self.owner);
                    if let Some(roots) = self.roots.get_mut(&rule) {
                        for root in roots {
                            root.readiness = Some(
                                ryo_values::domain::records::types::ProducerReadiness::Skipped,
                            );
                            root.skipped = Some(proof.clone());
                        }
                    }
                }
                FileRuleIntent::Refused(reason) => {
                    let reason = match reason {
                        ApplicabilityRefusal::SyntaxUndetermined => SYNTAX_UNDETERMINED,
                        ApplicabilityRefusal::MetadataUnavailable => METADATA_UNAVAILABLE,
                    };
                    if let Some(roots) = self.roots.get_mut(&rule) {
                        for root in roots {
                            root.readiness = Some(
                                ryo_values::domain::records::types::ProducerReadiness::Unknown(
                                    ryo_values::domain::records::types::ObservationReason(
                                        reason.into(),
                                    ),
                                ),
                            );
                        }
                    }
                }
                _ => {}
            }
        }
        self.execution
            .activate_file_roots(&readers, AbsentRoots::default())?;
        self.activation = Some(PendingActivation {
            readers,
            candidates,
            gate,
        });
        Ok(demanded)
    }
    /// Binds the file's selected roots before its first region is touched: the roots presence finds absent on the file's
    /// rows, the reader counts of one walk from the admitted roots, and the reached regions that are absent too.
    pub(crate) fn activate<'facts>(
        &mut self,
        read: &dyn Fn(Relation) -> Result<Rows<'facts>, OperatingError>,
        symbols: &ryo_language::domain::symbols::types::Symbols,
    ) -> Result<(), OperatingError> {
        let Some(PendingActivation {
            readers,
            candidates,
            gate,
        }) = self.activation.take()
        else {
            return Ok(());
        };
        // Presence reads the file's rows, so it binds when the first entry point reads the file; the counts bound at open
        // stand until then, and stand for good once a region was touched.
        if self.execution.touched()
            == crate::application::file_dag::program::runtime::types::Touch::Touched
        {
            return Ok(());
        }
        let source = self.source;
        let started = tracing::enabled!(target: EXECUTION_TARGET, tracing::Level::TRACE)
            .then(std::time::Instant::now);
        let decided = self.decide_presence(read, symbols, source, gate, &candidates)?;
        let presence_elapsed = started.map(|started| started.elapsed());
        if decided.count().0 > 0 {
            self.execution
                .activate_file_roots(&readers, AbsentRoots::decided(decided.clone()))?;
        }
        let walk_elapsed = started.map(|started| started.elapsed());
        let interior = self.decide_interior(read, symbols, source, gate, &decided)?;
        if let (Some(started), Some(presence_elapsed), Some(walk_elapsed)) =
            (started, presence_elapsed, walk_elapsed)
        {
            let mut skipped = Vec::new();
            for (at, candidate) in candidates.iter().enumerate() {
                let rule = candidate.rule;
                if candidates[..at].iter().any(|earlier| earlier.rule == rule) {
                    continue;
                }
                if candidates
                    .iter()
                    .filter(|each| each.rule == rule)
                    .all(|each| decided.presence(each.region) == RegionPresence::Absent)
                {
                    skipped.push(self.program.rule_id(rule)?.0.clone());
                }
            }
            tracing::trace!(target: EXECUTION_TARGET, event = PRESENCE_FILE, file = self.file.0,
                gate = ?gate, absent_roots = decided.count().0, interior = interior.count().0, skipped = ?skipped,
                presence_ns = presence_elapsed.as_nanos() as u64,
                walk_ns = walk_elapsed.saturating_sub(presence_elapsed).as_nanos() as u64,
                interior_ns = started.elapsed().saturating_sub(walk_elapsed).as_nanos() as u64);
        }
        Ok(())
    }
    pub(crate) fn declared(&self, rule: RuleIndex) -> Result<DeclaredIntent, OperatingError> {
        match self.intents.get(rule.0) {
            Some(FileRuleIntent::Run {
                diagnostic,
                observations,
                capture,
            }) => Ok(DeclaredIntent {
                delivery: *diagnostic,
                demand: *observations,
                capture_intent: *capture,
            }),
            _ => Err(OperatingError::NotRun),
        }
    }
    pub(crate) fn prepare_walk<'facts>(
        &mut self,
        read: &dyn Fn(Relation) -> Result<Rows<'facts>, OperatingError>,
        symbols: &mut ryo_language::domain::symbols::types::Symbols,
    ) -> Result<(), OperatingError> {
        if self.walk_prepared == WalkPreparation::Prepared {
            return Ok(());
        }
        self.context.validate(symbols)?;
        self.activate(read, symbols)?;
        let Some(first) = self.prepared_roots.first() else {
            self.walk_prepared = WalkPreparation::Prepared;
            return Ok(());
        };
        let rule = self.program.program().regions()[first.0].producer();
        let completion = match &self.partials {
            ScopePartials::File {
                evaluation,
                publication,
            } => CompletionPolicy::file_inputs(rule, self.file, evaluation, publication),
            ScopePartials::Project { .. } => return Err(OperatingError::WrongOperation),
        };
        let mut context = self.context.borrow(symbols, self.program, rule)?;
        let kernels =
            crate::application::file_dag::program::kernel_session::service::KernelSession::file(
                self.services.kernels(),
                self.program,
                Rc::clone(&self.owner),
                read,
                &self.walk,
            );
        let mut sources = SourceInputs {
            execution: &self.execution,
            state: self.execution.file_state(),
            completion: completion.clone(),
            owner: Rc::clone(&self.owner),
            read,
            observations: &mut self.observed,
            file_outputs: Some(&self.roots),
            program: self.program,
            rule,
            project: None,
        };
        self.execution.prepare_roots(
            self.execution.file_state(),
            &self.prepared_roots,
            &mut sources,
            &kernels,
            &completion,
            &mut context,
        )?;
        // Named roots have their own declared demand. Publish their readiness
        // through the same evaluation contract before delivering any diagnostic.
        let mut named = Vec::new();
        for &PreparedNamed {
            rule,
            output,
            readiness,
        } in &self.prepared_named
        {
            let root = &self.program.rule_metadata(rule)?.observations[output.0 as usize];
            sources.bind_rule(rule);
            let result = if readiness == NamedReadiness::Ready {
                root.source.clone().and_then(|source| {
                    sources.admit_source(
                        source,
                        &Partition::File(self.generation.clone()),
                        &mut context,
                    )
                })
            } else {
                Err(OperatingError::ObservationIncomplete {
                    reason: SOURCE_FACTS_INCOMPLETE.into(),
                })
            };
            let result = result.and_then(|input| {
                if input.observation_ready()?
                    == crate::application::file_dag::program::admission::types::ObservationReadiness::Ready
                {
                    Ok(input)
                } else {
                    Err(OperatingError::ObservationIncomplete {
                        reason: PRODUCER_COVERAGE_INCOMPLETE.into(),
                    })
                }
            });
            named.push((rule, output, result));
        }
        drop(sources);
        for (rule, output, result) in named {
            let root =
                &mut self.roots.get_mut(&rule).ok_or(OperatingError::NotRun)?[output.0 as usize];
            match result {
                Ok(input) => {
                    root.publication = Some(input);
                    root.readiness =
                        Some(ryo_values::domain::records::types::ProducerReadiness::Complete);
                }
                Err(error) => {
                    root.readiness = readiness_of(&classify(error));
                }
            }
        }
        self.walk_prepared = WalkPreparation::Prepared;
        Ok(())
    }
}
