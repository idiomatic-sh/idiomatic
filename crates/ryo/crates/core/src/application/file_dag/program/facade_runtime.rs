//! The application owns one execution and its admitted populations for each file
//! or project phase. Kernel code receives borrowed rows only after admission.
use super::{
    admission::{
        AdmittedInput, ApplicabilityProof, ApplicabilityReason, FileGeneration, Inputs, Partition,
        ProducerRoot, Readiness, RequiredPartitions,
    },
    facade::{
        CaptureContext, CapturedFrontier, EmitDemand, FrontierCapture, LogicalCounts,
        ObservationDemand, PreparedProgram, ProjectFrontiers, RootOutcome, RuleDemand, RuleOutcome,
    },
    runtime::{Delivery, InvocationOutcome, KernelInvocation, ProgramExecution},
};
use crate::{
    application::file_dag::{
        relations::{PublishedInput, RelationOwner},
        scope::{ScopePartials, open_program_owner},
    },
    domain::{
        coverage::types::{Covered, RuleIndex},
        operating::{errors::OperatingError, types::OperatorContext},
        replay::service::scan,
        table_model::{
            service::{count_of, materialize, reset},
            types::Rows,
        },
    },
    outbound::derived::interface::DerivedPort,
};
use ryo_relations::domain::relations::types::{FileId, Relation, Value};
use ryo_values::domain::{
    plan_parts::types::{Input, InputNumber},
    rows::types::Table,
    runs::types::Skipped,
    stored_facts::types::Bucket,
};
use std::{collections::HashMap, rc::Rc, sync::Arc};

pub(crate) struct ProgramServices<'service> {
    kernels: &'service dyn super::super::executor::PrimitiveExecutor,
    derived: Arc<dyn DerivedPort>,
}
impl<'service> ProgramServices<'service> {
    pub(crate) fn new(
        kernels: &'service dyn super::super::executor::PrimitiveExecutor,
        derived: Arc<dyn DerivedPort>,
    ) -> Self {
        Self { kernels, derived }
    }
    pub(super) fn kernels(&self) -> &'service dyn super::super::executor::PrimitiveExecutor {
        self.kernels
    }
    pub(super) fn derived(&self) -> &Arc<dyn DerivedPort> {
        &self.derived
    }
}

/// A named project phase and its bound inputs must use the very same owner.
pub(crate) struct PublicationScope {
    owner: Rc<RelationOwner>,
    execution: std::cell::OnceCell<ProgramExecution>,
}
impl PublicationScope {
    pub(in crate::application::file_dag) fn owner(&self) -> &Rc<RelationOwner> {
        &self.owner
    }
}
pub(crate) fn publication_owner(
    program: &Arc<PreparedProgram>,
    services: &ProgramServices<'_>,
) -> Result<PublicationScope, OperatingError> {
    Ok(PublicationScope {
        owner: open_program_owner(program.program(), Arc::clone(services.derived())),
        execution: std::cell::OnceCell::new(),
    })
}

/// The finite physical files named by this project request. No absent frame is
/// synthesized from a missing index, and a skipped file is never complete-empty.
pub(crate) fn required_partitions(
    program: &Arc<PreparedProgram>,
    rule: RuleIndex,
    files: &[ryo_values::domain::files::types::CorpusInput],
    covered: &[Covered],
    skipped: &[Skipped],
) -> Result<RequiredPartitions, OperatingError> {
    program.rule_metadata(rule)?;
    RequiredPartitions::from_request(files, covered, skipped)
}

struct ProjectSources<'a> {
    program: &'a PreparedProgram,
    rule: RuleIndex,
    ordinary: &'a mut [Option<crate::application::cross_file::types::FrontierInput>],
    refused: &'a [crate::domain::crossing::types::RefusedFrontier],
    bound: &'a mut [Option<crate::domain::crossing::types::BoundInputWords>],
    cache: &'a mut Vec<(
        crate::domain::executable_program::types::InputSource,
        AdmittedInput,
    )>,
}
impl ProjectSources<'_> {
    /// Called only by an actual DemandAction::Admit. Raw source handles and
    /// errors remain untouched when a branch does not demand them.
    fn admit(
        &mut self,
        source: crate::domain::executable_program::types::InputSource,
        partition: &Partition,
        context: &mut OperatorContext<'_>,
        owner: &Rc<RelationOwner>,
        kernels: &dyn KernelInvocation,
    ) -> Option<Result<AdmittedInput, OperatingError>> {
        use crate::{
            application::cross_file::types::FrontierInput,
            domain::executable_program::types::InputSource,
        };
        if let Some((_, admitted)) = self.cache.iter().find(|(key, _)| *key == source) {
            return Some(Ok(admitted.clone()));
        }
        let program = self.program;
        let rule = self.rule;
        let mut pending = None;
        for (at, input) in self.ordinary.iter().enumerate() {
            let Some(input) = input else { continue };
            let original = match input {
                FrontierInput::Decoded(frontier) => frontier.input,
                FrontierInput::Compact { input, .. } => *input,
            };
            if source_for(program, rule, original).ok() == Some(source) {
                pending = Some(at);
                break;
            }
        }
        if let Some(at) = pending {
            let input = self.ordinary[at]
                .take()
                .expect("demanded source was present");
            let publication = match input {
                FrontierInput::Decoded(frontier) => owner.computed(frontier.table),
                FrontierInput::Compact { table, .. } => owner.compact(Arc::new(table)),
            };
            let result = publication
                .and_then(|publication| admit_published(owner, publication, source, partition));
            if let Ok(input) = &result {
                self.cache.push((source, input.clone()));
            }
            return Some(result);
        }
        for refused in self.refused {
            if source_for(program, rule, refused.input).ok() == Some(source) {
                return Some(Err(refused.error.clone()));
            }
        }
        if let InputSource::Observation(slot) = source {
            for bound in &mut *self.bound {
                let Some(words) = bound.as_ref() else {
                    continue;
                };
                if program.observation_slot(rule, words.input).ok() != Some(slot) {
                    continue;
                }
                let words = bound.take().expect("demanded bound input was present");
                let result = (|| {
                    let kept = words.words?;
                    let tables = kept
                        .iter()
                        .map(|words| {
                            crate::domain::table_words::service::decode_complete(
                                &words.0,
                                ryo_language::domain::symbols::service::query_places_mut(
                                    &mut context.symbols,
                                ),
                            )
                            .map_err(|error| {
                                OperatingError::ObservationIncomplete {
                                    reason: error.to_string(),
                                }
                            })
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    let contract = words.union?;
                    let publication = super::kernel_session::combine_bound(
                        kernels,
                        owner,
                        &tables,
                        &words.kinds,
                        &contract,
                        context,
                    )?;
                    admit_published(owner, publication, source, partition)
                })();
                if let Ok(input) = &result {
                    self.cache.push((source, input.clone()));
                }
                return Some(result);
            }
            return Some(Err(OperatingError::ObservationIncomplete {
                reason: "declared bound producer is unavailable".into(),
            }));
        }
        None
    }
}
fn admit_published(
    owner: &Rc<RelationOwner>,
    publication: PublishedInput,
    source: crate::domain::executable_program::types::InputSource,
    partition: &Partition,
) -> Result<AdmittedInput, OperatingError> {
    let unavailable = |_| Err(OperatingError::NotRead);
    let rows = owner.resolve(publication.as_ref(), &unavailable)?;
    let checked = owner.admit(
        publication.as_ref(),
        crate::domain::operator::contract::views::BorrowedRows::from(&rows),
    )?;
    AdmittedInput::from_publication(
        Rc::clone(owner),
        publication,
        &checked,
        ProducerRoot::Input(source),
        partition.clone(),
        Readiness::Complete,
    )
}

/// The application admission port is the only bridge from an executable source
/// to a generation-checked publication. It has no second scheduler.
struct SourceInputs<'a, 'facts> {
    execution: &'a ProgramExecution,
    owner: Rc<RelationOwner>,
    read: &'a dyn Fn(Relation) -> Result<Rows<'facts>, OperatingError>,
    kernels: &'a dyn KernelInvocation,
    observations: &'a HashMap<usize, AdmittedInput>,
    project: Option<ProjectSources<'a>>,
}
impl Inputs for SourceInputs<'_, '_> {
    fn admit(
        &mut self,
        source: crate::domain::executable_program::types::InputSource,
        partition: &Partition,
        context: &mut OperatorContext<'_>,
    ) -> Result<AdmittedInput, OperatingError> {
        use crate::domain::executable_program::types::InputSource;
        if let Some(project) = &mut self.project {
            if let Some(provided) =
                project.admit(source, partition, context, &self.owner, self.kernels)
            {
                return provided;
            }
        }
        match source {
            InputSource::Relation(relation) => {
                let rows = (self.read)(relation)?;
                let publication = self.owner.source(relation, &rows)?;
                let checked = self.owner.admit(
                    publication.as_ref(),
                    crate::domain::operator::contract::views::BorrowedRows::from(&rows),
                )?;
                AdmittedInput::from_publication(
                    Rc::clone(&self.owner),
                    publication,
                    &checked,
                    ProducerRoot::Input(source),
                    partition.clone(),
                    Readiness::Complete,
                )
            }
            InputSource::Region(id) => {
                match self
                    .execution
                    .invoke(id, self, self.kernels, context, Delivery::Retain)?
                {
                    InvocationOutcome::Published(input) => Ok(input),
                    InvocationOutcome::Forwarded => Err(OperatingError::NotRun),
                }
            }
            InputSource::Observation(slot) => {
                self.observations.get(&slot).cloned().ok_or_else(|| {
                    OperatingError::ObservationIncomplete {
                        reason: "declared observation publication is unavailable".into(),
                    }
                })
            }
        }
    }
    fn is_empty(&self, input: &AdmittedInput) -> Result<bool, OperatingError> {
        let rows = self
            .owner
            .resolve(input.publication(&self.owner)?.as_ref(), self.read)?;
        Ok(count_of(&rows).0 == 0)
    }
    fn publish(
        &mut self,
        root: ProducerRoot,
        partition: &Partition,
        _inputs: &[AdmittedInput],
        rows: Table,
    ) -> Result<AdmittedInput, OperatingError> {
        let publication = self.owner.computed(rows)?;
        let resolved = self.owner.resolve(publication.as_ref(), self.read)?;
        let checked = self.owner.admit(
            publication.as_ref(),
            crate::domain::operator::contract::views::BorrowedRows::from(&resolved),
        )?;
        AdmittedInput::from_publication(
            Rc::clone(&self.owner),
            publication,
            &checked,
            root,
            partition.clone(),
            Readiness::Complete,
        )
    }
    fn publish_empty(
        &mut self,
        root: ProducerRoot,
        partition: &Partition,
        inputs: &[AdmittedInput],
        schema: Result<Vec<ryo_relations::domain::relations::types::CellKind>, OperatingError>,
    ) -> Result<AdmittedInput, OperatingError> {
        let mut rows = Table::default();
        reset(&mut rows, &schema?);
        self.publish(root, partition, inputs, rows)
    }
    fn replay(
        &self,
        input: &AdmittedInput,
        context: &mut OperatorContext<'_>,
        output: &mut dyn crate::domain::operating::types::Emit,
    ) -> Result<(), OperatingError> {
        let rows = self
            .owner
            .resolve(input.publication(&self.owner)?.as_ref(), self.read)?;
        scan(&rows, &mut Vec::new(), context, output)
    }
}

fn table_of(
    owner: &Rc<RelationOwner>,
    input: &AdmittedInput,
    read: &dyn Fn(Relation) -> Result<Rows<'_>, OperatingError>,
) -> Result<Table, OperatingError> {
    let rows = owner.resolve(input.publication(owner)?.as_ref(), read)?;
    let mut table = Table::default();
    materialize(&rows, &mut table, &mut Vec::new());
    Ok(table)
}

struct FileRoot {
    readiness: Option<ryo_values::domain::observations::types::ProducerReadiness>,
    committed_frontier: bool,
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
    services: &'a ProgramServices<'service>,
) -> Result<FileRun<'a, 'service>, OperatingError> {
    if !matches!(partials, ScopePartials::File { .. }) {
        return Err(OperatingError::WrongOperation);
    }
    let owner = open_program_owner(program.program(), Arc::clone(services.derived()));
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
        }
        let _ = proof;
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
        let kernels = super::kernel_session::KernelSession::new(
            self.services.kernels(),
            Rc::clone(&self.owner),
            read,
        );
        let mut sources = SourceInputs {
            execution: &self.execution,
            owner: Rc::clone(&self.owner),
            read,
            kernels: &kernels,
            observations: &self.observed,
            project: None,
        };
        let emit = match demand.emit {
            EmitDemand::Disabled => RootOutcome::NoQuery,
            EmitDemand::Enabled => match &metadata.emit_root {
                Ok(id) => outcome_of(
                    self.execution
                        .invoke(*id, &mut sources, &kernels, context, Delivery::Retain),
                    &self.owner,
                    read,
                ),
                Err(error) => RootOutcome::Failed(error.clone()),
            },
        };
        let mut observations = Vec::new();
        if matches!(demand.observations, ObservationDemand::Required) {
            for (index, root) in metadata.observations.iter().enumerate() {
                if root.phase != ryo_values::domain::plans::types::StepScope::PerFile {
                    continue;
                }
                let result = root.source.clone().and_then(|source| {
                    sources.admit(
                        source,
                        &Partition::File(FileGeneration::from_owner(self.file, &self.owner)),
                        context,
                    )
                });
                let outcome = match result {
                    Ok(input) => {
                        let table = table_of(&self.owner, &input, read);
                        if let Ok(table) = table {
                            self.observed
                                .insert(self.program.observation_slot(rule, index)?, input);
                            RootOutcome::Complete(table)
                        } else {
                            RootOutcome::Unknown(table.err().expect("checked error"))
                        }
                    }
                    Err(error) => classify(error),
                };
                self.roots[rule.0][index].readiness = readiness_of(&outcome);
                observations.push((
                    ryo_values::domain::observations::types::ObservationIndex(
                        u32::try_from(index).map_err(|_| OperatingError::ShapeMismatch)?,
                    ),
                    outcome,
                ));
            }
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
        let kernels = super::kernel_session::KernelSession::new(
            self.services.kernels(),
            Rc::clone(&self.owner),
            read,
        );
        let mut inputs = SourceInputs {
            execution: &self.execution,
            owner: Rc::clone(&self.owner),
            read,
            kernels: &kernels,
            observations: &self.observed,
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
        RootOutcome::Skipped(_) => ProducerReadiness::Skipped,
        RootOutcome::Unknown(error) | RootOutcome::Failed(error) => {
            ProducerReadiness::Unknown(ObservationReason(error.to_string().into()))
        }
        RootOutcome::NoQuery => ProducerReadiness::NoQuery,
    })
}

fn classify(error: OperatingError) -> RootOutcome {
    match error {
        OperatingError::ObservationIncomplete { .. }
        | OperatingError::FrontierUnavailable { .. }
        | OperatingError::SemanticIncomplete { .. } => RootOutcome::Unknown(error),
        other => RootOutcome::Failed(other),
    }
}
fn outcome_of(
    result: Result<InvocationOutcome, OperatingError>,
    owner: &Rc<RelationOwner>,
    read: &dyn Fn(Relation) -> Result<Rows<'_>, OperatingError>,
) -> RootOutcome {
    match result {
        Ok(InvocationOutcome::Published(input)) => match table_of(owner, &input, read) {
            Ok(table) => RootOutcome::Complete(table),
            Err(error) => classify(error),
        },
        Ok(InvocationOutcome::Forwarded) => RootOutcome::Failed(OperatingError::NotRun),
        Err(error) => classify(error),
    }
}

/// One project phase borrows the coordinator's shared executable generation.
/// Raw frontier and bound handles remain deferred until the demand interpreter
/// calls SourceInputs::admit for their exact source.
pub(crate) struct ProjectRun<'a, 'service> {
    program: &'a Arc<PreparedProgram>,
    services: &'a ProgramServices<'service>,
    execution: &'a ProgramExecution,
    owner: Rc<RelationOwner>,
    required: RequiredPartitions,
    partials: ScopePartials<'a>,
    ordinary: Vec<Option<crate::application::cross_file::types::FrontierInput>>,
    refused: Vec<crate::domain::crossing::types::RefusedFrontier>,
    bound: Vec<Option<crate::domain::crossing::types::BoundInputWords>>,
    cache: Vec<(
        crate::domain::executable_program::types::InputSource,
        AdmittedInput,
    )>,
}
pub(crate) fn open_project<'a, 'service>(
    program: &'a Arc<PreparedProgram>,
    required: RequiredPartitions,
    partials: ScopePartials<'a>,
    frontiers: ProjectFrontiers<'a>,
    _symbols: &mut ryo_language::domain::symbols::types::Symbols,
    services: &'a ProgramServices<'service>,
) -> Result<ProjectRun<'a, 'service>, OperatingError> {
    if !matches!(partials, ScopePartials::Project { .. }) {
        return Err(OperatingError::WrongOperation);
    }
    let scope = frontiers.publication;
    let owner = Rc::clone(scope.owner());
    if let Some(existing) = scope.execution.get() {
        if !Arc::ptr_eq(&existing.program, program.program())
            || !existing.project_partition_matches(&required)
        {
            return Err(OperatingError::WrongOperation);
        }
    } else {
        scope
            .execution
            .set(ProgramExecution::open(
                Arc::clone(program.program()),
                Partition::Project(required.clone()),
                Rc::clone(&owner),
            ))
            .map_err(|_| OperatingError::WrongOperation)?;
    }
    let execution = scope.execution.get().ok_or(OperatingError::NotRun)?;
    Ok(ProjectRun {
        program,
        services,
        execution,
        owner,
        required,
        partials,
        ordinary: frontiers.ordinary.rows.into_iter().map(Some).collect(),
        refused: frontiers.ordinary.refused,
        bound: frontiers.bound_inputs.iter().cloned().map(Some).collect(),
        cache: Vec::new(),
    })
}
impl ProjectRun<'_, '_> {
    pub(crate) fn rule(
        &mut self,
        rule: RuleIndex,
        demand: RuleDemand,
        context: &mut OperatorContext<'_>,
    ) -> Result<RuleOutcome, OperatingError> {
        let metadata = self.program.rule_metadata(rule)?;
        let unavailable = |_relation| Err(OperatingError::NotRead);
        let kernels = super::kernel_session::KernelSession::new(
            self.services.kernels(),
            Rc::clone(&self.owner),
            &unavailable,
        );
        let partition = Partition::Project(self.required.clone());
        let absent = HashMap::new();
        let mut sources = SourceInputs {
            execution: self.execution,
            owner: Rc::clone(&self.owner),
            read: &unavailable,
            kernels: &kernels,
            observations: &absent,
            project: Some(ProjectSources {
                program: self.program,
                rule,
                ordinary: &mut self.ordinary,
                refused: &self.refused,
                bound: &mut self.bound,
                cache: &mut self.cache,
            }),
        };
        let emit = match demand.emit {
            EmitDemand::Disabled => RootOutcome::NoQuery,
            EmitDemand::Enabled => match &metadata.emit_root {
                Ok(id) => outcome_of(
                    self.execution
                        .invoke(*id, &mut sources, &kernels, context, Delivery::Retain),
                    &self.owner,
                    &unavailable,
                ),
                Err(error) => RootOutcome::Failed(error.clone()),
            },
        };
        let mut observations = Vec::new();
        if matches!(demand.observations, ObservationDemand::Required) {
            for (index, root) in metadata.observations.iter().enumerate() {
                if root.phase != ryo_values::domain::plans::types::StepScope::CrossFile {
                    continue;
                }
                let result = root
                    .source
                    .clone()
                    .and_then(|source| sources.admit(source, &partition, context));
                observations.push((
                    ryo_values::domain::observations::types::ObservationIndex(
                        u32::try_from(index).map_err(|_| OperatingError::ShapeMismatch)?,
                    ),
                    match result {
                        Ok(input) => outcome_of(
                            Ok(InvocationOutcome::Published(input)),
                            &self.owner,
                            &unavailable,
                        ),
                        Err(error) => classify(error),
                    },
                ));
            }
        }
        Ok(RuleOutcome {
            emit,
            observations,
            counts: self.execution.logical_counts(rule)?,
        })
    }
}
fn source_for(
    program: &PreparedProgram,
    rule: RuleIndex,
    input: Input,
) -> Result<crate::domain::executable_program::types::InputSource, OperatingError> {
    let index = program
        .frontiers(rule)?
        .iter()
        .position(|each| *each == input)
        .ok_or(OperatingError::NotRun)?;
    program
        .program()
        .rules
        .get(rule.0)
        .and_then(|rule| rule.frontier.get(index))
        .ok_or(OperatingError::NotRun)?
        .as_ref()
        .copied()
        .map_err(Clone::clone)
}
