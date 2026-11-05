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
    roots: Vec<Vec<FileRoot>>,
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
    let demanded = intents
        .iter()
        .map(|intent| match intent {
            FileRuleIntent::Run {
                diagnostic,
                observations,
                capture,
            } => {
                *diagnostic != DiagnosticDelivery::Disabled
                    || *observations == ObservationDemand::Required
                    || *capture != CaptureIntent::Disabled
            }
            _ => false,
        })
        .collect::<Vec<_>>();
    owner
        .derived
        .prepare_program_file(program.derived(), &demanded, publication)?;
    let generation = FileGeneration::from_owner(file, &owner);
    let execution = ProgramExecution::open(
        Arc::clone(program.program()),
        Partition::File(generation.clone()),
        Rc::clone(&owner),
    );
    let roots = (0..program.rule_count())
        .map(|rule| {
            program.rule_metadata(RuleIndex(rule)).map(|metadata| {
                metadata
                    .observations
                    .iter()
                    .map(|_| FileRoot {
                        readiness: None,
                        committed_frontier: false,
                        publication: None,
                        skipped: None,
                    })
                    .collect()
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut run = FileRun {
        _routes: routes,
        program,
        services: Arc::clone(&services),
        execution,
        owner,
        file,
        partials,
        roots,
        intents: intents.into(),
        generation,
        observed: HashMap::new(),
        walk: super::super::native_walk::FileWalk::new(file),
        walk_prepared: false,
        context: super::super::context::RunContext::file(
            &services, source, symbols, semantic, kinds,
        ),
    };
    run.bind_intents()?;
    Ok(run)
}
impl FileRun<'_> {
    /// Cancel every undeclared external reader before a source or native walk is admitted.
    fn bind_intents(&mut self) -> Result<(), OperatingError> {
        use crate::domain::executable_program::types::InputSource;
        for (index, intent) in self.intents.iter().copied().enumerate() {
            let rule = RuleIndex(index);
            let prepared = &self.program.program().rules()[index];
            let metadata = self.program.rule_metadata(rule)?;
            let mut enabled = Vec::new();
            if let FileRuleIntent::Run {
                diagnostic,
                observations,
                capture,
            } = intent
            {
                if diagnostic != DiagnosticDelivery::Disabled {
                    enabled.extend(prepared.emit.iter().copied());
                }
                if observations == ObservationDemand::Required {
                    enabled.extend(metadata.observations.iter().filter_map(|root| {
                        match root.source.as_ref().ok()? {
                            InputSource::Region(id) => Some(*id),
                            _ => None,
                        }
                    }));
                }
                for (frontier, source) in prepared.frontier.iter().enumerate() {
                    let wanted = match capture {
                        CaptureIntent::Disabled => false,
                        CaptureIntent::Observations => metadata
                            .observation_frontiers
                            .get(frontier)
                            .copied()
                            .unwrap_or(false),
                        CaptureIntent::DiagnosticAndObservations => true,
                    };
                    if wanted {
                        match source {
                            Ok(InputSource::Region(id)) => enabled.push(*id),
                            Ok(InputSource::Observation(slot)) => {
                                if let Some(Ok(id)) =
                                    self.program.program().observation_regions().get(*slot)
                                {
                                    enabled.push(*id);
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            // Reader obligations are a multiset: repeated root aliases remain distinct.
            let mut cancelled = Vec::new();
            for root in prepared.root_readers.iter().copied() {
                if let Some(place) = enabled.iter().position(|enabled| *enabled == root) {
                    enabled.swap_remove(place);
                } else {
                    cancelled.push(root);
                }
            }
            if !cancelled.is_empty() {
                self.execution.cancel_roots(&cancelled)?;
            }
            match intent {
                FileRuleIntent::Undemanded => {}
                FileRuleIntent::Skipped(reason) => {
                    let proof = ApplicabilityProof::for_file(rule, self.file, reason, &self.owner);
                    for root in &mut self.roots[index] {
                        root.readiness = Some(
                            ryo_values::domain::observations::types::ProducerReadiness::Skipped,
                        );
                        root.skipped = Some(proof.clone());
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
                    for root in &mut self.roots[index] {
                        root.readiness = Some(
                            ryo_values::domain::observations::types::ProducerReadiness::Unknown(
                                ryo_values::domain::observations::types::ObservationReason(
                                    reason.into(),
                                ),
                            ),
                        );
                    }
                }
                FileRuleIntent::Run { .. } => {}
            }
        }
        Ok(())
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
        use crate::domain::operator::contract::types::EvaluationMode;
        let mut queued = Vec::new();
        for (index, intent) in self.intents.iter().enumerate() {
            let FileRuleIntent::Run {
                diagnostic,
                observations,
                capture,
            } = intent
            else {
                continue;
            };
            let metadata = self.program.rule_metadata(RuleIndex(index))?;
            let mut append = |source: &Result<InputSource, OperatingError>, mode| {
                let id = match source {
                    Ok(InputSource::Region(id)) => Some(*id),
                    Ok(InputSource::Observation(slot)) => self
                        .program
                        .program()
                        .observation_regions()
                        .get(*slot)
                        .and_then(|id| id.as_ref().ok())
                        .copied(),
                    _ => None,
                };
                if let Some(id) = id {
                    if self.program.program().regions()[id.0].phase
                        == ryo_values::domain::plans::types::StepScope::PerFile
                    {
                        if let Some((_, existing)) =
                            queued.iter_mut().find(|(found, _)| *found == id)
                        {
                            if mode == EvaluationMode::Streaming {
                                *existing = mode;
                            }
                        } else {
                            queued.push((id, mode));
                        }
                    }
                }
            };
            if *diagnostic != DiagnosticDelivery::Disabled {
                append(
                    &metadata.emit_root.clone().map(InputSource::Region),
                    match diagnostic {
                        DiagnosticDelivery::Forwarded => EvaluationMode::Streaming,
                        _ => EvaluationMode::Collected,
                    },
                );
            }
            if *observations == ObservationDemand::Required {
                let policy = match &self.partials {
                    ScopePartials::File {
                        evaluation,
                        publication,
                    } => CompletionPolicy::file_inputs(
                        RuleIndex(index),
                        self.file,
                        evaluation,
                        publication,
                    ),
                    ScopePartials::Project { .. } => return Err(OperatingError::WrongOperation),
                };
                for root in &metadata.observations {
                    if policy.observation_admission(&root.reads)
                        == crate::domain::completion::types::ObservationAdmission::Ready
                    {
                        append(&root.source, EvaluationMode::Collected);
                    }
                }
            }
            let prepared = &self.program.program().rules()[index];
            for (frontier, source) in prepared.frontier.iter().enumerate() {
                let wanted = match capture {
                    CaptureIntent::Disabled => false,
                    CaptureIntent::Observations => metadata
                        .observation_frontiers
                        .get(frontier)
                        .copied()
                        .unwrap_or(false),
                    CaptureIntent::DiagnosticAndObservations => true,
                };
                if wanted {
                    append(source, EvaluationMode::Collected);
                }
            }
        }
        let Some((first, _)) = queued.first() else {
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
        let kernels = super::super::kernel_session::KernelSession::new(
            self.services.kernels(),
            self.program,
            Rc::clone(&self.owner),
            read,
            Some(&self.walk),
        );
        let mut sources = SourceInputs {
            execution: &self.execution,
            completion: completion.clone(),
            owner: Rc::clone(&self.owner),
            read,
            observations: &mut self.observed,
            file_outputs: Some(&self.roots),
            program: self.program,
            rule,
            project: None,
        };
        self.execution
            .prepare_roots(&queued, &mut sources, &kernels, &completion, &mut context)?;
        // Named roots are independent demanded roots even when the diagnostic
        // has a forwarded delivery. Publish their readiness without requiring a
        // collected diagnostic API call.
        let mut named = Vec::new();
        for (index, intent) in self.intents.iter().enumerate() {
            let FileRuleIntent::Run {
                observations: ObservationDemand::Required,
                ..
            } = intent
            else {
                continue;
            };
            let rule = RuleIndex(index);
            let metadata = self.program.rule_metadata(rule)?;
            let policy = completion.for_rule(rule);
            sources.bind_rule(rule);
            for (output, root) in metadata.observations.iter().enumerate() {
                if root.phase != ryo_values::domain::plans::types::StepScope::PerFile {
                    continue;
                }
                let result = if policy.observation_admission(&root.reads)
                    == crate::domain::completion::types::ObservationAdmission::Ready
                {
                    root.source.clone().and_then(|source| {
                        sources.admit(
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
        }
        drop(sources);
        for (rule, output, result) in named {
            let root = &mut self.roots[rule][output];
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
        self.prepare_walk(read, symbols)?;
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
        let kernels = super::super::kernel_session::KernelSession::new(
            self.services.kernels(),
            self.program,
            Rc::clone(&self.owner),
            read,
            Some(&self.walk),
        );
        let mut sources = SourceInputs {
            execution: &self.execution,
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
                        *id,
                        &mut sources,
                        &kernels,
                        &completion,
                        context,
                        Delivery::Retain,
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
                        sources.admit(source, &Partition::File(self.generation.clone()), context)
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
            self.roots[rule.0][index].publication = Some(input);
        }
        for (output, outcome) in &observations {
            self.roots[rule.0][output.0 as usize].readiness = readiness_of(outcome);
        }
        Ok(RuleOutcome {
            emit,
            observations,
            counts: self.execution.logical_counts(rule)?,
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
        let kernels = super::super::kernel_session::KernelSession::new(
            self.services.kernels(),
            self.program,
            Rc::clone(&self.owner),
            read,
            Some(&self.walk),
        );
        let mut inputs = SourceInputs {
            execution: &self.execution,
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
                let admitted =
                    inputs.admit(source?, &Partition::File(self.generation.clone()), context)?;
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
        let metadata = self.program.observation_metadata();
        if let Some(metadata) = metadata {
            for root in metadata
                .frontiers
                .iter()
                .filter(|root| root.producer == rule && root.frontier == input)
            {
                self.roots
                    .get_mut(rule.0)
                    .and_then(|outputs| outputs.get_mut(root.output.0 as usize))
                    .ok_or(OperatingError::NotRun)?
                    .committed_frontier = true;
            }
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
    /// Read-only test telemetry from actual common-program dispatch and retirement.
    #[cfg(test)]
    pub(crate) fn route_observation(
        &self,
        rule: RuleIndex,
        step: ryo_values::domain::plan_parts::types::StepId,
    ) -> Result<crate::domain::operating::types::ExecutionObservation, OperatingError> {
        self.execution.route_observation(rule, step)
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
        self.execution.region_observation(*region)
    }
    pub(crate) fn walk_counts(&self) -> crate::domain::shared_walk::types::WalkCounts {
        self.walk.counts()
    }
    pub(crate) fn counts(&self, rule: RuleIndex) -> Result<LogicalCounts, OperatingError> {
        self.execution.logical_counts(rule)
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
            let state = self
                .roots
                .get(root.producer.0)?
                .get(root.output.0 as usize)?;
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
        Ok(
            InvocationOutcome::Forwarded(_)
            | InvocationOutcome::Ready
            | InvocationOutcome::Suspended(_),
        ) => RootOutcome::Failed(OperatingError::NotRun),
        Err(error) => classify(error),
    }
}

#[cfg(test)]
mod distinct_integration_test;

#[cfg(test)]
impl FileRun<'_> {
    pub(crate) fn source_reuse_eligible(&self) -> bool {
        self.owner.stable()
    }
}

impl FileRun<'_> {
    /// Stream through the same prepared slot, admission, completion and retry owner.
    pub(crate) fn emit_rows<'facts>(
        &mut self,
        rule: RuleIndex,
        read: &dyn Fn(Relation) -> Result<Rows<'facts>, OperatingError>,
        symbols: &mut ryo_language::domain::symbols::types::Symbols,
        each: &mut dyn FnMut(
            &[ryo_relations::domain::relations::types::Value],
        ) -> Result<(), OperatingError>,
    ) -> Result<super::super::facade::RowEmission, OperatingError> {
        if self.declared(rule)?.0 != DiagnosticDelivery::Forwarded {
            return Err(OperatingError::NotRun);
        }
        self.prepare_walk(read, symbols)?;
        let _rule_origin = crate::application::file_dag::route_counts::rule(
            rule.0,
            &self.program.rule_id(rule)?.0,
        );
        let _consumer_origin = crate::application::file_dag::route_counts::consumer("file_stream");
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
        let kernels = super::super::kernel_session::KernelSession::new(
            self.services.kernels(),
            self.program,
            Rc::clone(&self.owner),
            read,
            Some(&self.walk),
        );
        let mut sources = SourceInputs {
            execution: &self.execution,
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
        let id = metadata.emit_root.as_ref().map_err(Clone::clone)?;
        let phase = self.program.program().regions()[id.0].phase;
        if phase != ryo_values::domain::plans::types::StepScope::PerFile {
            return Ok(super::super::facade::RowEmission::Deferred { phase });
        }
        let mut output = ForwardRows(each);
        let coverage = match self.execution.invoke(
            *id,
            &mut sources,
            &kernels,
            &completion,
            context,
            Delivery::Forward(rule, &mut output),
        )? {
            InvocationOutcome::Published(input) => input.coverage()?,
            InvocationOutcome::Forwarded(coverage) => coverage,
            InvocationOutcome::Ready | InvocationOutcome::Suspended(_) => {
                return Err(OperatingError::NotRun);
            }
        };
        Ok(match coverage {
            crate::domain::completion::types::CoverageState::Complete => {
                super::super::facade::RowEmission::Complete
            }
            crate::domain::completion::types::CoverageState::Partial { files } => {
                super::super::facade::RowEmission::Partial { affected: files }
            }
        })
    }
}
