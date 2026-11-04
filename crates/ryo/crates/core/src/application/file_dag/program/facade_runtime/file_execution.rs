//! File execution and named observation lifecycle.
use super::*;

pub(super) struct FileRoot {
    pub(super) readiness: Option<ryo_values::domain::observations::types::ProducerReadiness>,
    pub(super) committed_frontier: bool,
    pub(super) publication: Option<AdmittedInput>,
    pub(super) skipped: Option<ApplicabilityProof>,
}
pub(crate) struct FileRun<'a, 'service> {
    program: &'a Arc<PreparedProgram>,
    services: &'a ProgramServices<'service>,
    execution: ProgramExecution,
    owner: Rc<RelationOwner>,
    file: FileId,
    partials: ScopePartials<'a>,
    roots: Vec<Vec<FileRoot>>,
    observed: HashMap<usize, AdmittedInput>,
}
pub(crate) fn open_file<'a, 'service>(
    program: &'a Arc<PreparedProgram>,
    file: FileId,
    partials: ScopePartials<'a>,
    demanded: &[bool],
    services: &'a ProgramServices<'service>,
) -> Result<FileRun<'a, 'service>, OperatingError> {
    if !matches!(partials, ScopePartials::File { .. }) {
        return Err(OperatingError::WrongOperation);
    }
    let owner = open_program_owner(program.program(), Arc::clone(services.derived()));
    let ScopePartials::File { publication, .. } = partials else {
        return Err(OperatingError::WrongOperation);
    };
    owner
        .derived
        .prepare_program_file(program.derived(), demanded, publication)?;
    let generation = FileGeneration::from_owner(file, &owner);
    let execution = ProgramExecution::open(
        Arc::clone(program.program()),
        Partition::File(generation),
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
    Ok(FileRun {
        program,
        services,
        execution,
        owner,
        file,
        partials,
        roots,
        observed: HashMap::new(),
    })
}
impl FileRun<'_, '_> {
    pub(crate) fn skip_rule(
        &mut self,
        rule: RuleIndex,
        reason: ApplicabilityReason,
    ) -> Result<(), OperatingError> {
        let proof = ApplicabilityProof::for_file(rule, self.file, reason, &self.owner);
        for root in self.roots.get_mut(rule.0).ok_or(OperatingError::NotRun)? {
            root.readiness =
                Some(ryo_values::domain::observations::types::ProducerReadiness::Skipped);
            root.publication = None;
            root.skipped = Some(proof.clone());
        }
        Ok(())
    }
    pub(crate) fn rule<'facts>(
        &mut self,
        rule: RuleIndex,
        demand: RuleDemand,
        read: &dyn Fn(Relation) -> Result<Rows<'facts>, OperatingError>,
        context: &mut OperatorContext<'_>,
    ) -> Result<RuleOutcome, OperatingError> {
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
        );
        let mut sources = SourceInputs {
            execution: &self.execution,
            completion: completion.clone(),
            owner: Rc::clone(&self.owner),
            read,
            kernels: &kernels,
            observations: &mut self.observed,
            file_outputs: Some(&self.roots),
            program: self.program,
            rule,
            project: None,
        };
        let emit = match demand.emit {
            EmitDemand::Disabled => RootOutcome::NoQuery,
            EmitDemand::Enabled => match &metadata.emit_root {
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
                let result = if completion.observation_ready(&root.reads) {
                    root.source.clone().and_then(|source| {
                        sources.admit(
                            source,
                            &Partition::File(FileGeneration::from_owner(self.file, &self.owner)),
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
        context: &mut OperatorContext<'_>,
        capture: CaptureContext<'_>,
    ) -> Result<Vec<FrontierCapture>, OperatingError> {
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
            .rules
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
        );
        let mut inputs = SourceInputs {
            execution: &self.execution,
            completion,
            owner: Rc::clone(&self.owner),
            read,
            kernels: &kernels,
            observations: &mut self.observed,
            file_outputs: Some(&self.roots),
            program: self.program,
            rule,
            project: None,
        };
        let CaptureContext {
            file,
            worker,
            retained,
            words,
        } = capture;
        let crate::domain::finish::types::Retained { arena, places } = retained;
        for (index, source) in sources.into_iter().enumerate() {
            let input = InputNumber(index);
            let outcome = (|| {
                let admitted = inputs.admit(
                    source?,
                    &Partition::File(FileGeneration::from_owner(file, &self.owner)),
                    context,
                )?;
                let mut table = table_of(&self.owner, &admitted, read)?;
                let before = places.len();
                crate::domain::finish::service::escape(
                    &mut table,
                    crate::domain::finish::types::Leaving {
                        symbols: context.symbols.as_ref(),
                        source: context.source,
                        file,
                        worker,
                        text: crate::domain::finish::types::Leaves::WithText,
                    },
                    crate::domain::finish::types::Retained {
                        arena: crate::domain::finish::types::Arena(&mut *arena.0),
                        places: &mut *places,
                    },
                );
                words.0.clear();
                crate::domain::table_words::service::encode_table(&table, &places[before..], words);
                let packed = crate::domain::table_words::service::pack(
                    crate::domain::table_words::types::WordSlice(&words.0),
                );
                Ok(CapturedFrontier {
                    input,
                    bucket: crate::domain::reading::service::frontier_bucket(first, input),
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
                Ok(crate::application::file_dag::completion::CoverageState::Complete) => {
                    RootOutcome::Complete(table)
                }
                Ok(crate::application::file_dag::completion::CoverageState::Partial { files }) => {
                    RootOutcome::Partial {
                        table,
                        affected: files,
                    }
                }
                Err(error) => classify(error),
            },
            Err(error) => classify(error),
        },
        Ok(InvocationOutcome::Forwarded) => RootOutcome::Failed(OperatingError::NotRun),
        Err(error) => classify(error),
    }
}
