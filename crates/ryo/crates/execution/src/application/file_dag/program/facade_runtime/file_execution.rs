//! File execution and named observation lifecycle.
use super::*;
use crate::domain::executable_program::types::InputSource;

pub(super) struct FileRoot {
    pub(super) readiness: Option<ryo_values::domain::observations::types::ProducerReadiness>,
    pub(super) committed_frontier: bool,
    pub(super) publication: Option<AdmittedInput>,
    pub(super) skipped: Option<ApplicabilityProof>,
}
pub(crate) struct FileRun<'a> {
    program: &'a Arc<PreparedProgram>,
    services: Arc<ProgramServices>,
    execution: ProgramExecution,
    owner: Rc<RelationOwner>,
    file: FileId,
    partials: ScopePartials<'a>,
    roots: HashMap<usize, Vec<FileRoot>>,
    prepared_roots: Vec<crate::domain::executable_program::types::RegionId>,
    prepared_named: Vec<(usize, usize, bool)>,
    /// Immutable intent and identity share the same private file owner lifetime.
    intents: Box<[FileRuleIntent]>,
    generation: FileGeneration,
    observed: HashMap<usize, AdmittedInput>,
    context: super::super::context::RunContext<'a>,
    walk: super::super::native_walk::FileWalk,
    walk_prepared: bool,
    _routes: crate::application::file_dag::route_counts::FileScope,
}
pub(crate) fn open_file<'a>(
    program: &'a Arc<PreparedProgram>,
    file: FileId,
    partials: ScopePartials<'a>,
    intents: &[FileRuleIntent],
    services: Arc<ProgramServices>,
    source: &'a [u8],
    reuse_eligible: bool,
    symbols: &mut ryo_language::domain::symbols::types::Symbols,
    semantic: Option<&'a dyn crate::domain::operating::types::SemanticLookup>,
    kinds: Option<Arc<crate::domain::kind_sets::types::PreparedVocabulary>>,
) -> Result<FileRun<'a>, OperatingError> {
    if intents.len() != program.rule_count() {
        return Err(OperatingError::ShapeMismatch);
    }
    if !services.request.contains(file) {
        return Err(OperatingError::NotRead);
    }
    if !matches!(partials, ScopePartials::File { .. }) {
        return Err(OperatingError::WrongOperation);
    }
    let routes = crate::application::file_dag::route_counts::FileScope::begin(file.0 as usize);
    #[cfg(feature = "diagnostic-row-progress")]
    if let Some(path) = services.request.path(file) {
        crate::application::file_dag::route_counts::file_name(&path.0);
    }
    let owner = open_program_owner(
        program.program(),
        Arc::clone(services.derived()),
        reuse_eligible,
    );
    let ScopePartials::File { publication, .. } = partials else {
        return Err(OperatingError::WrongOperation);
    };
    let generation = FileGeneration::from_owner(file, &owner);
    let execution = ProgramExecution::open(
        Arc::clone(program.program()),
        Partition::File(generation.clone()),
        Rc::clone(&owner),
    );
    let mut run = FileRun {
        _routes: routes,
        program,
        services: Arc::clone(&services),
        execution,
        owner,
        file,
        partials,
        roots: HashMap::new(),
        prepared_roots: Vec::new(),
        prepared_named: Vec::new(),
        intents: intents.into(),
        generation: generation.clone(),
        observed: HashMap::new(),
        walk: super::super::native_walk::FileWalk::new(generation),
        walk_prepared: false,
        context: super::super::context::RunContext::file(
            &services, source, symbols, semantic, kinds,
        ),
    };
    let demanded = run.bind_intents()?;
    run.owner
        .derived
        .prepare_program_file(program.derived(), &demanded, publication)?;
    Ok(run)
}
impl FileRun<'_> {
    /// Bind selected external readers before a source or native walk is admitted.
    fn bind_intents(&mut self) -> Result<Vec<RuleIndex>, OperatingError> {
        use crate::domain::executable_program::file_demand::types::{
            FileDemand, FileRootDemand as DemandRoot, FrontierSelection, RootSelection,
        };
        let ScopePartials::File { publication, .. } = &self.partials else {
            return Err(OperatingError::WrongOperation);
        };
        let mut seen = std::collections::HashSet::new();
        let mut readers = Vec::new();
        let mut demanded = Vec::new();
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
                _ => FileDemand::NONE,
            };
            if selected != FileDemand::NONE {
                demanded.push(rule);
            }
            let plan = compiled.selected(selected);
            readers.extend_from_slice(&plan.readers);
            if (selected != FileDemand::NONE
                || matches!(
                    intent,
                    FileRuleIntent::Skipped(_) | FileRuleIntent::Refused(_)
                ))
                && compiled.outputs.0 != 0
            {
                self.roots.insert(
                    index,
                    (0..compiled.outputs.0)
                        .map(|_| FileRoot {
                            readiness: None,
                            committed_frontier: false,
                            publication: None,
                            skipped: None,
                        })
                        .collect(),
                );
            }
            for &output in &plan.observations {
                let root = &self.program.rule_metadata(rule)?.observations[output.0 as usize];
                let ready = crate::domain::trust::service::blocked(&root.reads, publication)
                    == crate::domain::trust::types::Blocking::Free;
                self.prepared_named.push((index, output.0 as usize, ready));
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
                        if crate::domain::trust::service::blocked(&root.reads, publication)
                            != crate::domain::trust::types::Blocking::Free
                        {
                            continue;
                        }
                        *region
                    }
                };
                if seen.insert(id) {
                    self.prepared_roots.push(id);
                }
            }
            match intent {
                FileRuleIntent::Skipped(reason) => {
                    let proof = ApplicabilityProof::for_file(rule, self.file, reason, &self.owner);
                    if let Some(roots) = self.roots.get_mut(&index) {
                        for root in roots {
                            root.readiness = Some(
                                ryo_values::domain::observations::types::ProducerReadiness::Skipped,
                            );
                            root.skipped = Some(proof.clone());
                        }
                    }
                }
                FileRuleIntent::Refused(reason) => {
                    let reason = match reason {
                        ApplicabilityRefusal::SyntaxUndetermined => {
                            "syntax acceptance undetermined"
                        }
                        ApplicabilityRefusal::MetadataUnavailable => {
                            "applicability metadata unavailable"
                        }
                    };
                    if let Some(roots) = self.roots.get_mut(&index) {
                        for root in roots {
                            root.readiness = Some(
                                ryo_values::domain::observations::types::ProducerReadiness::Unknown(
                                    ryo_values::domain::observations::types::ObservationReason(
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
        self.execution.activate_file_roots(&readers)?;
        Ok(demanded)
    }
    fn declared(
        &self,
        rule: RuleIndex,
    ) -> Result<(DiagnosticDelivery, ObservationDemand, CaptureIntent), OperatingError> {
        match self.intents.get(rule.0) {
            Some(FileRuleIntent::Run {
                diagnostic,
                observations,
                capture,
            }) => Ok((*diagnostic, *observations, *capture)),
            _ => Err(OperatingError::NotRun),
        }
    }
    fn prepare_walk<'facts>(
        &mut self,
        read: &dyn Fn(Relation) -> Result<Rows<'facts>, OperatingError>,
        symbols: &mut ryo_language::domain::symbols::types::Symbols,
    ) -> Result<(), OperatingError> {
        if self.walk_prepared {
            return Ok(());
        }
        let Some(first) = self.prepared_roots.first() else {
            self.walk_prepared = true;
            return Ok(());
        };
        let rule = self.program.program().regions()[first.0].producer;
        let completion = match &self.partials {
            ScopePartials::File {
                evaluation,
                publication,
            } => CompletionPolicy::file_inputs(rule, self.file, evaluation, publication),
            ScopePartials::Project { .. } => return Err(OperatingError::WrongOperation),
        };
        let mut context = self.context.borrow(symbols, self.program, rule)?;
        let kernels = super::super::kernel_session::KernelSession::file(
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
        for &(index, output, ready) in &self.prepared_named {
            let rule = RuleIndex(index);
            let root = &self.program.rule_metadata(rule)?.observations[output];
            sources.bind_rule(rule);
            let result = if ready {
                root.source.clone().and_then(|source| {
                    sources.admit_source(
                        source,
                        &Partition::File(self.generation.clone()),
                        &mut context,
                    )
                })
            } else {
                Err(OperatingError::ObservationIncomplete {
                    reason: "physical source facts are incomplete".into(),
                })
            };
            let result = result.and_then(|input| {
                if input.observation_ready()? {
                    Ok(input)
                } else {
                    Err(OperatingError::ObservationIncomplete {
                        reason: "producer publication has incomplete physical coverage".into(),
                    })
                }
            });
            named.push((index, output, result));
        }
        drop(sources);
        for (rule, output, result) in named {
            let root = &mut self.roots.get_mut(&rule).ok_or(OperatingError::NotRun)?[output];
            match result {
                Ok(input) => {
                    root.publication = Some(input);
                    root.readiness =
                        Some(ryo_values::domain::observations::types::ProducerReadiness::Complete);
                }
                Err(error) => {
                    root.readiness = readiness_of(&classify(error));
                }
            }
        }
        self.walk_prepared = true;
        Ok(())
    }
    pub(crate) fn rule<'facts>(
        &mut self,
        rule: RuleIndex,
        demand: RuleDemand,
        read: &dyn Fn(Relation) -> Result<Rows<'facts>, OperatingError>,
        symbols: &mut ryo_language::domain::symbols::types::Symbols,
    ) -> Result<RuleOutcome, OperatingError> {
        let (diagnostic, observations, _) = self.declared(rule)?;
        let emit = match diagnostic {
            DiagnosticDelivery::Disabled => EmitDemand::Disabled,
            DiagnosticDelivery::Collected => EmitDemand::Enabled,
            DiagnosticDelivery::Forwarded => return Err(OperatingError::NotRun),
        };
        if (RuleDemand { emit, observations }) != demand {
            return Err(OperatingError::NotRun);
        }
        let metadata = self.program.rule_metadata(rule)?;
        let owns_file_root = (demand.emit == EmitDemand::Enabled
            && metadata.emit_root.as_ref().is_ok_and(|id| {
                self.program.program().regions()[id.0].phase
                    == ryo_values::domain::plans::types::StepScope::PerFile
            }))
            || (demand.observations == ObservationDemand::Required
                && metadata.observations.iter().any(|root| {
                    root.phase == ryo_values::domain::plans::types::StepScope::PerFile
                }));
        if owns_file_root {
            self.prepare_walk(read, symbols)?;
        }
        let _rule_origin = crate::application::file_dag::route_counts::rule(
            rule.0,
            &self.program.rule_id(rule)?.0,
        );
        let _consumer_origin = crate::application::file_dag::route_counts::consumer("file_rule");
        let mut context = self.context.borrow(symbols, self.program, rule)?;
        let context = &mut context;
        let metadata = self.program.rule_metadata(rule)?;
        let completion = match &self.partials {
            ScopePartials::File {
                evaluation,
                publication,
            } => CompletionPolicy::file_inputs(rule, self.file, evaluation, publication),
            ScopePartials::Project { .. } => return Err(OperatingError::WrongOperation),
        };
        let kernels = super::super::kernel_session::KernelSession::file(
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
        let emit = match demand.emit {
            EmitDemand::Disabled => RootOutcome::NoQuery,
            EmitDemand::Enabled => match &metadata.emit_root {
                Ok(id)
                    if self.program.program().regions()[id.0].phase
                        != ryo_values::domain::plans::types::StepScope::PerFile =>
                {
                    RootOutcome::Deferred {
                        phase: self.program.program().regions()[id.0].phase,
                    }
                }
                Ok(id) => outcome_of(
                    self.execution.invoke(
                        self.execution.file_state(),
                        *id,
                        &mut sources,
                        &kernels,
                        &completion,
                        context,
                    ),
                    &self.owner,
                    read,
                ),
                Err(error) => RootOutcome::Failed(error.clone()),
            },
        };
        let mut observations = Vec::new();
        let mut published = Vec::new();
        if matches!(demand.observations, ObservationDemand::Required) {
            for (index, root) in metadata.observations.iter().enumerate() {
                if root.phase != ryo_values::domain::plans::types::StepScope::PerFile {
                    continue;
                }
                let result = if completion.observation_admission(&root.reads)
                    == crate::domain::completion::types::ObservationAdmission::Ready
                {
                    root.source.clone().and_then(|source| {
                        sources.admit_source(
                            source,
                            &Partition::File(self.generation.clone()),
                            context,
                        )
                    })
                } else {
                    Err(OperatingError::ObservationIncomplete {
                        reason: "physical source facts are incomplete".into(),
                    })
                };
                let outcome = match result {
                    Ok(input) => match observation_table(&self.owner, &input, read) {
                        Ok(table) => {
                            published.push((index, input));
                            RootOutcome::Complete(table)
                        }
                        Err(error) => classify(error),
                    },
                    Err(error) => classify(error),
                };
                observations.push((
                    ryo_values::domain::observations::types::ObservationIndex(
                        u32::try_from(index).map_err(|_| OperatingError::ShapeMismatch)?,
                    ),
                    outcome,
                ));
            }
        }
        drop(sources);
        for (index, input) in published {
            self.roots.get_mut(&rule.0).ok_or(OperatingError::NotRun)?[index].publication =
                Some(input);
        }
        for (output, outcome) in &observations {
            self.roots.get_mut(&rule.0).ok_or(OperatingError::NotRun)?[output.0 as usize]
                .readiness = readiness_of(outcome);
        }
        Ok(RuleOutcome {
            emit,
            observations,
            counts: self
                .execution
                .logical_counts(self.execution.file_state(), rule)?,
        })
    }
    pub(crate) fn capture_frontiers<'facts>(
        &mut self,
        rule: RuleIndex,
        first: Bucket,
        read: &dyn Fn(Relation) -> Result<Rows<'facts>, OperatingError>,
        symbols: &mut ryo_language::domain::symbols::types::Symbols,
        capture: CaptureContext<'_>,
    ) -> Result<Vec<FrontierCapture>, OperatingError> {
        let (_, _, capture_intent) = self.declared(rule)?;
        if capture_intent == CaptureIntent::Disabled {
            return Err(OperatingError::NotRun);
        }
        self.prepare_walk(read, symbols)?;
        let mut context = self.context.borrow(symbols, self.program, rule)?;
        let context = &mut context;
        let completion = match &self.partials {
            ScopePartials::File {
                evaluation,
                publication,
            } => CompletionPolicy::file_inputs(rule, self.file, evaluation, publication),
            ScopePartials::Project { .. } => return Err(OperatingError::WrongOperation),
        };
        let mut captures = Vec::new();
        let sources = self
            .program
            .program()
            .rules()
            .get(rule.0)
            .ok_or(OperatingError::NotRun)?
            .frontier
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        let kernels = super::super::kernel_session::KernelSession::file(
            self.services.kernels(),
            self.program,
            Rc::clone(&self.owner),
            read,
            &self.walk,
        );
        let mut inputs = SourceInputs {
            execution: &self.execution,
            state: self.execution.file_state(),
            completion,
            owner: Rc::clone(&self.owner),
            read,
            observations: &mut self.observed,
            file_outputs: Some(&self.roots),
            program: self.program,
            rule,
            project: None,
        };
        let CaptureContext {
            file,
            rule_digest,
            worker,
            retained,
            words,
        } = capture;
        let crate::domain::finish::types::Retained { arena, places: _ } = retained;
        for (index, source) in sources.into_iter().enumerate() {
            if capture_intent == CaptureIntent::Observations
                && !self.program.captures_observation(rule, index)?
            {
                continue;
            }
            if let Ok(source) = source.as_ref()
                && matches!(
                    self.program.program().frontier_phase(*source),
                    Ok(ryo_values::domain::plans::types::StepScope::CrossFile)
                )
            {
                continue;
            }
            let input = InputNumber(index);
            let outcome = (|| {
                let admitted = inputs.admit_source(
                    source?,
                    &Partition::File(self.generation.clone()),
                    context,
                )?;
                let coverage = crate::domain::completion::service::file_frontier_coverage(
                    admitted.coverage()?,
                    file,
                )?;
                let mut table = table_of(&self.owner, &admitted, read)?;
                let frame_places = crate::domain::finish::service::escape_frame(
                    &mut table,
                    crate::domain::finish::types::Leaving {
                        symbols: context.symbols.as_ref(),
                        source: context.source,
                        file,
                        worker,
                        text: crate::domain::finish::types::Leaves::WithText,
                    },
                    crate::domain::finish::types::Arena(&mut *arena.0),
                )
                .map_err(|reason| OperatingError::ObservationIncomplete {
                    reason: reason.to_string(),
                })?;
                words.0.clear();
                ryo_values::domain::table_words::service::encode_table(
                    &table,
                    &frame_places,
                    words,
                );
                let packed = ryo_values::domain::table_words::service::pack(
                    ryo_values::domain::table_words::types::WordSlice(&words.0),
                );
                Ok(CapturedFrontier {
                    input,
                    bucket: ryo_values::domain::stored_facts::service::frontier_bucket(
                        first, input,
                    ),
                    proof: ryo_values::domain::records::types::FrontierProof {
                        version: 1,
                        rule: u32::try_from(rule.0).map_err(|_| OperatingError::ShapeMismatch)?,
                        input: u32::try_from(index).map_err(|_| OperatingError::ShapeMismatch)?,
                        rule_digest,
                        coverage,
                    },
                    words: packed,
                })
            })();
            captures.push(FrontierCapture { input, outcome });
        }
        Ok(captures)
    }
    pub(crate) fn commit_frontier(
        &mut self,
        rule: RuleIndex,
        input: InputNumber,
    ) -> Result<(), OperatingError> {
        let (_, _, capture) = self.declared(rule)?;
        if capture == CaptureIntent::Disabled
            || (capture == CaptureIntent::Observations
                && !self.program.captures_observation(rule, input.0)?)
            || input.0 >= self.program.frontiers(rule)?.len()
        {
            return Err(OperatingError::NotRun);
        }
        for &output in self.program.file_frontier_outputs(rule, input.0)? {
            self.roots
                .get_mut(&rule.0)
                .and_then(|outputs| outputs.get_mut(output))
                .ok_or(OperatingError::NotRun)?
                .committed_frontier = true;
        }
        Ok(())
    }
    pub(crate) fn finish(
        mut self,
    ) -> Result<Option<Arc<ryo_values::domain::records::types::FrontierReadiness>>, OperatingError>
    {
        let readiness = self.observation_snapshot();
        self.roots.clear();
        self.observed.clear();
        self.owner.derived.report(self.file);
        let (builds, hits, refusals) = self.owner.derived.counts();
        tracing::trace!(target: "ryo::diagnostic::resource_routes", file = self.file.0, builds, hits, refusals, event = "file_owner_finish");
        self.execution.finish()?;
        Ok(readiness)
    }
    /// Weak lifetime observation only; the test never retains a workspace or owner.
    #[cfg(test)]
    pub(crate) fn owner_lifetime(&self) -> std::rc::Weak<RelationOwner> {
        Rc::downgrade(&self.owner)
    }
    /// Read-only test telemetry from actual common-program dispatch and retirement.
    #[cfg(test)]
    pub(crate) fn route_observation(
        &self,
        rule: RuleIndex,
        step: ryo_values::domain::plan_parts::types::StepId,
    ) -> Result<crate::domain::operating::types::ExecutionObservation, OperatingError> {
        self.execution
            .route_observation(self.execution.file_state(), rule, step)
    }
    #[cfg(test)]
    pub(crate) fn resource_fixture_counts(
        &self,
    ) -> crate::application::file_dag::derived::owner::fixtures::resources::Snapshot {
        self.owner.derived.resource_fixture_counts()
    }
    #[cfg(test)]
    pub(crate) fn observation_routes(
        &self,
        slot: usize,
    ) -> Result<crate::domain::operating::types::ExecutionObservation, OperatingError> {
        let region = self
            .program
            .program()
            .observation_regions()
            .get(slot)
            .ok_or(OperatingError::NotRun)?
            .as_ref()
            .map_err(Clone::clone)?;
        self.execution
            .region_observation(self.execution.file_state(), *region)
    }
    pub(crate) fn readiness_counts(&self) -> crate::domain::readiness::types::ReadinessCounts {
        self.execution.readiness_counts()
    }
    pub(crate) fn walk_counts(&self) -> crate::domain::shared_walk::types::WalkCounts {
        self.walk.counts()
    }
    pub(crate) fn counts(&self, rule: RuleIndex) -> Result<LogicalCounts, OperatingError> {
        self.execution
            .logical_counts(self.execution.file_state(), rule)
    }
    pub(crate) fn observation_snapshot(
        &self,
    ) -> Option<Arc<ryo_values::domain::records::types::FrontierReadiness>> {
        use ryo_values::domain::observations::{
            service::{new_frontiers, set_frontier},
            types::{ObservationReadinessCount, ObservationReadinessSlot, ProducerReadiness},
        };
        let metadata = self.program.observation_metadata()?;
        if metadata.frontiers.is_empty() {
            return None;
        }
        let count = u32::try_from(metadata.frontiers.len()).ok()?;
        let mut map = new_frontiers(metadata.layout, ObservationReadinessCount(count));
        for (slot, root) in metadata.frontiers.iter().enumerate() {
            let Some(state) = self
                .roots
                .get(&root.producer.0)
                .and_then(|outputs| outputs.get(root.output.0 as usize))
            else {
                continue;
            };
            let readiness = match &state.readiness {
                Some(ProducerReadiness::Complete) if state.committed_frontier => {
                    ProducerReadiness::Complete
                }
                Some(ProducerReadiness::Complete) => continue,
                Some(other) => other.clone(),
                None => continue,
            };
            set_frontier(
                &mut map,
                ObservationReadinessSlot(u32::try_from(slot).ok()?),
                readiness,
            )
            .ok()?;
        }
        Some(Arc::new(map))
    }
}

fn readiness_of(
    outcome: &RootOutcome,
) -> Option<ryo_values::domain::observations::types::ProducerReadiness> {
    use ryo_values::domain::observations::types::{ObservationReason, ProducerReadiness};
    Some(match outcome {
        RootOutcome::Complete(_) => ProducerReadiness::Complete,
        RootOutcome::Partial { .. } => ProducerReadiness::Unknown(ObservationReason(
            "producer publication has incomplete physical coverage".into(),
        )),
        RootOutcome::Skipped(_) => ProducerReadiness::Skipped,
        RootOutcome::Unknown(error) | RootOutcome::Failed(error) => {
            ProducerReadiness::Unknown(ObservationReason(error.to_string().into()))
        }
        RootOutcome::NoQuery => ProducerReadiness::NoQuery,
        RootOutcome::Deferred { .. } => return None,
    })
}

pub(super) fn classify(error: OperatingError) -> RootOutcome {
    match error {
        OperatingError::ObservationIncomplete { .. }
        | OperatingError::FrontierUnavailable { .. }
        | OperatingError::SemanticIncomplete { .. } => RootOutcome::Unknown(error),
        other => RootOutcome::Failed(other),
    }
}
pub(super) fn outcome_of<'facts>(
    result: Result<InvocationOutcome, OperatingError>,
    owner: &Rc<RelationOwner>,
    read: &dyn Fn(Relation) -> Result<Rows<'facts>, OperatingError>,
) -> RootOutcome {
    match result {
        Ok(InvocationOutcome::Published(input)) => match table_of(owner, &input, read) {
            Ok(table) => match input.coverage() {
                Ok(crate::domain::completion::types::CoverageState::Complete) => {
                    RootOutcome::Complete(table)
                }
                Ok(crate::domain::completion::types::CoverageState::Partial { files }) => {
                    RootOutcome::Partial {
                        table,
                        affected: files,
                    }
                }
                Err(error) => classify(error),
            },
            Err(error) => classify(error),
        },
        Ok(InvocationOutcome::Ready | InvocationOutcome::Suspended(_)) => {
            RootOutcome::Failed(OperatingError::NotRun)
        }
        Err(error) => classify(error),
    }
}

#[cfg(test)]
mod distinct_integration;

#[cfg(test)]
impl FileRun<'_> {
    pub(crate) fn source_reuse_eligible(&self) -> bool {
        self.owner.stable()
    }
}

impl FileRun<'_> {
    /// Deliver the canonical publication; callbacks never participate in evaluation.
    pub(crate) fn emit_rows<'facts>(
        &mut self,
        rule: RuleIndex,
        read: &dyn Fn(Relation) -> Result<Rows<'facts>, OperatingError>,
        symbols: &mut ryo_language::domain::symbols::types::Symbols,
        each: &mut dyn FnMut(
            &[ryo_relations::domain::relations::types::Value],
        ) -> Result<(), OperatingError>,
    ) -> Result<crate::application::file_dag::program::protocol::types::RowEmission, OperatingError>
    {
        if self.declared(rule)?.0 != DiagnosticDelivery::Forwarded {
            return Err(OperatingError::NotRun);
        }
        let id = *self
            .program
            .program()
            .rules()
            .get(rule.0)
            .ok_or(OperatingError::NotRun)?
            .emit
            .as_ref()
            .map_err(Clone::clone)?;
        let phase = self.program.program().regions()[id.0].phase;
        if phase != ryo_values::domain::plans::types::StepScope::PerFile {
            return Ok(
                crate::application::file_dag::program::protocol::types::RowEmission::Deferred {
                    phase,
                },
            );
        }
        self.prepare_walk(read, symbols)?;
        let _rule_origin = crate::application::file_dag::route_counts::rule(
            rule.0,
            &self.program.rule_id(rule)?.0,
        );
        let _consumer_origin = crate::application::file_dag::route_counts::consumer("file_stream");
        let mut context = self.context.borrow(symbols, self.program, rule)?;
        let context = &mut context;
        let completion = match &self.partials {
            ScopePartials::File {
                evaluation,
                publication,
            } => CompletionPolicy::file_inputs(rule, self.file, evaluation, publication),
            ScopePartials::Project { .. } => return Err(OperatingError::WrongOperation),
        };
        let kernels = super::super::kernel_session::KernelSession::file(
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
        struct ForwardRows<'a>(
            &'a mut dyn FnMut(
                &[ryo_relations::domain::relations::types::Value],
            ) -> Result<(), OperatingError>,
        );
        impl crate::domain::operating::types::Emit for ForwardRows<'_> {
            fn row(
                &mut self,
                _context: &mut OperatorContext<'_>,
                row: &[ryo_relations::domain::relations::types::Value],
            ) -> Result<(), OperatingError> {
                (self.0)(row)
            }
        }
        let published = match self.execution.invoke(
            self.execution.file_state(),
            id,
            &mut sources,
            &kernels,
            &completion,
            context,
        )? {
            InvocationOutcome::Published(input) => input,
            InvocationOutcome::Ready | InvocationOutcome::Suspended(_) => {
                return Err(OperatingError::NotRun);
            }
        };
        let coverage = published.coverage()?;
        // Delivery begins only after semantic evaluation has committed. A sink
        // refusal leaves that publication intact for replay without reevaluation.
        sources.replay(&published, context, &mut ForwardRows(each))?;
        Ok(match coverage {
            crate::domain::completion::types::CoverageState::Complete => {
                crate::application::file_dag::program::protocol::types::RowEmission::Complete
            }
            crate::domain::completion::types::CoverageState::Partial { files } => {
                crate::application::file_dag::program::protocol::types::RowEmission::Partial {
                    affected: files,
                }
            }
        })
    }
}
