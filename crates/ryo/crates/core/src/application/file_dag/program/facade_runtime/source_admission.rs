//! Demand-triggered source admission, publication, and replay for one program generation.
use super::*;

pub(super) struct ProjectSources<'a> {
    pub(super) program: &'a PreparedProgram,
    pub(super) rule: RuleIndex,
    pub(super) ordinary: &'a mut [Option<crate::application::cross_file::types::FrontierInput>],
    pub(super) refused: &'a [crate::domain::crossing::types::RefusedFrontier],
    pub(super) bound: &'a mut [Option<crate::domain::crossing::types::BoundInputWords>],
    pub(super) cache: &'a mut Vec<(
        crate::domain::executable_program::types::InputSource,
        AdmittedInput,
    )>,
    pub(super) transport: &'a ryo_language::domain::symbols::types::TransportPermit,
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
                                    self.transport,
                                )
                                .map_err(|_| OperatingError::NotRead)?,
                            )
                            .map_err(|error| {
                                OperatingError::ObservationIncomplete {
                                    reason: error.to_string(),
                                }
                            })
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    let contract = words.union?;
                    let publication = super::super::kernel_session::combine_bound(
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
        match source {
            crate::domain::executable_program::types::InputSource::Region(id) => {
                ProducerRoot::Region(id)
            }
            _ => ProducerRoot::Input(source),
        },
        partition.clone(),
        Readiness::Complete,
    )
}

/// The application admission port is the only bridge from an executable source
/// to a generation-checked publication. It has no second scheduler.
pub(super) struct SourceInputs<'a, 'facts> {
    pub(super) execution: &'a ProgramExecution,
    pub(super) completion: CompletionPolicy,
    pub(super) owner: Rc<RelationOwner>,
    pub(super) read: &'a dyn Fn(Relation) -> Result<Rows<'facts>, OperatingError>,
    pub(super) kernels: &'a dyn KernelInvocation,
    pub(super) observations: &'a mut HashMap<usize, AdmittedInput>,
    pub(super) file_outputs: Option<&'a [Vec<FileRoot>]>,
    pub(super) program: &'a PreparedProgram,
    pub(super) rule: RuleIndex,
    pub(super) project: Option<ProjectSources<'a>>,
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
                let producer = self
                    .execution
                    .program
                    .regions
                    .get(id.0)
                    .ok_or(OperatingError::NotRun)?
                    .producer;
                let completion = self.completion.for_rule(producer);
                match self.execution.invoke(
                    id,
                    self,
                    self.kernels,
                    &completion,
                    context,
                    Delivery::Retain,
                )? {
                    InvocationOutcome::Published(input) => Ok(input),
                    InvocationOutcome::Forwarded => Err(OperatingError::NotRun),
                }
            }
            InputSource::Observation(slot) => {
                if let Some(input) = self.observations.get(&slot) {
                    return Ok(input.clone());
                }
                let outputs =
                    self.file_outputs
                        .ok_or_else(|| OperatingError::ObservationIncomplete {
                            reason: "declared observation publication is unavailable".into(),
                        })?;
                let named = self
                    .program
                    .observation_metadata()
                    .ok_or(OperatingError::NotRun)?;
                let declared = named.slots.get(slot).ok_or(OperatingError::NotRun)?;
                if declared.rule != self.rule.0 {
                    return Err(OperatingError::NotRead);
                }
                let binding = named
                    .bindings
                    .get(self.rule.0)
                    .and_then(|bindings| {
                        bindings
                            .iter()
                            .find(|binding| binding.input == declared.input)
                    })
                    .ok_or(OperatingError::NotRun)?;
                let mut tables = Vec::with_capacity(binding.producers.len());
                for producer in &binding.producers {
                    let crate::domain::file_program::types::ResolvedObservationProducer::Output {
                        rule,
                        output,
                    } = producer
                    else {
                        return Err(OperatingError::ObservationIncomplete {
                            reason: "named producer has no executable query".into(),
                        });
                    };
                    let admitted = outputs
                        .get(rule.0)
                        .and_then(|roots| roots.get(output.0 as usize))
                        .and_then(|root| root.publication.as_ref())
                        .ok_or_else(|| OperatingError::ObservationIncomplete {
                            reason: "named producer root is not complete".into(),
                        })?;
                    tables.push(table_of(&self.owner, admitted, self.read)?);
                }
                let contract = named
                    .unions
                    .get(self.rule.0)
                    .and_then(|contracts| contracts.get(declared.input.0 as usize))
                    .ok_or(OperatingError::NotRun)?
                    .as_ref()
                    .map_err(Clone::clone)?;
                let publication = super::super::kernel_session::combine_bound(
                    self.kernels,
                    &self.owner,
                    &tables,
                    &binding
                        .schema
                        .columns
                        .iter()
                        .map(|column| column.kind)
                        .collect::<Vec<_>>(),
                    contract,
                    context,
                )?;
                let admitted = admit_published(&self.owner, publication, source, partition)?;
                self.observations.insert(slot, admitted.clone());
                Ok(admitted)
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

pub(super) fn table_of<'facts>(
    owner: &Rc<RelationOwner>,
    input: &AdmittedInput,
    read: &dyn Fn(Relation) -> Result<Rows<'facts>, OperatingError>,
) -> Result<Table, OperatingError> {
    let rows = owner.resolve(input.publication(owner)?.as_ref(), read)?;
    let mut table = Table::default();
    materialize(&rows, &mut table, &mut Vec::new());
    Ok(table)
}

pub(super) fn observation_table<'facts>(
    owner: &Rc<RelationOwner>,
    input: &AdmittedInput,
    read: &dyn Fn(Relation) -> Result<Rows<'facts>, OperatingError>,
) -> Result<Table, OperatingError> {
    if !input.observation_ready()? {
        return Err(OperatingError::ObservationIncomplete {
            reason: "producer publication has incomplete physical coverage".into(),
        });
    }
    table_of(owner, input, read)
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
