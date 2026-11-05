//! Demand-triggered source admission, publication, and replay for one program generation.
use super::*;

pub(super) struct ProjectSources<'a> {
    pub(super) program: &'a PreparedProgram,
    pub(super) rule: RuleIndex,
    pub(super) sources: &'a [crate::application::file_dag::frontiers::types::FrontierSource],
    pub(super) ordinary: &'a [Rc<super::super::project_population::service::Population>],
    pub(super) refused: &'a [crate::domain::crossing::types::RefusedFrontier],
    pub(super) bound:
        &'a mut [Option<crate::application::file_dag::frontiers::types::BoundInput>],
    pub(super) cache: &'a mut Vec<(
        crate::domain::executable_program::types::InputSource,
        AvailableInput,
    )>,
    pub(super) transport: &'a ryo_language::domain::symbols::types::TransportPermit,
    pub(super) publication: &'a PublicationScope,
}
impl ProjectSources<'_> {
    fn ordinary_place(
        &self,
        source: crate::domain::executable_program::types::InputSource,
    ) -> Option<usize> {
        self.ordinary
            .iter()
            .position(|input| input.provides(source))
    }

    fn provides(&self, source: crate::domain::executable_program::types::InputSource) -> bool {
        matches!(
            source,
            crate::domain::executable_program::types::InputSource::ObservationMember { .. }
        ) || self.cache.iter().any(|(root, _)| *root == source)
            || self.ordinary_place(source).is_some()
            || self.refused.iter().any(|refused| {
                self.sources.iter().any(|binding| {
                    binding.input == refused.input
                        && matches!(&binding.root, Ok(actual) if *actual == source)
                })
            })
    }
    /// Called only by an actual DemandAction::Admit. Raw source handles and
    /// errors remain untouched when a branch does not demand them.
    fn supply(
        &mut self,
        source: crate::domain::executable_program::types::InputSource,
        partition: &Partition,
        context: &mut OperatorContext<'_>,
        owner: &Rc<RelationOwner>,
    ) -> Option<Result<AvailableInput, OperatingError>> {
        use crate::domain::executable_program::types::InputSource;
        if let Some((_, admitted)) = self.cache.iter().find(|(key, _)| *key == source) {
            return Some(Ok(admitted.clone()));
        }
        let program = self.program;
        let rule = self.rule;
        if let Some(at) = self.ordinary_place(source) {
            let actual_partition = match self.publication.execution.get() {
                Some(execution) => execution.partition(),
                None => return Some(Err(OperatingError::NotRead)),
            };
            if let Err(error) = actual_partition.require_supply(partition) {
                return Some(Err(error));
            }
            let result = self.ordinary[at].supply(owner, actual_partition);
            if let Ok(input) = &result {
                self.cache.push((source, input.clone()));
            }
            return Some(result);
        }
        for refused in self.refused {
            if self.sources.iter().any(|binding| {
                binding.input == refused.input
                    && matches!(&binding.root, Ok(actual) if *actual == source)
            }) {
                return Some(Err(refused.error.clone()));
            }
        }
        if let InputSource::ObservationMember { slot, member } = source {
            for bound in &mut *self.bound {
                let Some(binding) = bound.as_mut() else {
                    continue;
                };
                if observation_slot(program, rule, binding.input).ok() != Some(slot) {
                    continue;
                }
                let result = (|| {
                    let proof = binding
                        .members
                        .as_ref()
                        .map_err(Clone::clone)?
                        .get(member)
                        .ok_or(OperatingError::NotRun)?;
                    let producer = match proof {
                        crate::application::file_dag::frontiers::types::BoundMember::Current(
                            actual,
                        ) => actual.clone(),
                        crate::application::file_dag::frontiers::types::BoundMember::Restored(
                            token,
                        ) => {
                            let Partition::Project(required) = partition else {
                                return Err(OperatingError::WrongOperation);
                            };
                            token.materialize(
                                self.publication,
                                required,
                                self.transport,
                                context,
                            )?
                        }
                    };
                    super::super::admission::alias_member(
                        program,
                        slot,
                        member,
                        producer.admitted(),
                        partition,
                        owner,
                    )
                })();
                let result = result.map(AvailableInput::published);
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
        publication.clone(),
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
    pub(super) state: &'a super::super::runtime::RuntimeState,
    pub(super) completion: CompletionPolicy,
    pub(super) owner: Rc<RelationOwner>,
    pub(super) read: &'a dyn Fn(Relation) -> Result<Rows<'facts>, OperatingError>,
    pub(super) observations: &'a mut HashMap<usize, AdmittedInput>,
    pub(super) file_outputs: Option<&'a [Vec<FileRoot>]>,
    pub(super) program: &'a PreparedProgram,
    pub(super) rule: RuleIndex,
    pub(super) project: Option<ProjectSources<'a>>,
}
impl SourceInputs<'_, '_> {
    pub(super) fn admit_source(
        &mut self,
        source: crate::domain::executable_program::types::InputSource,
        partition: &Partition,
        context: &mut OperatorContext<'_>,
    ) -> Result<AdmittedInput, OperatingError> {
        let available = self.supply(source, partition, context)?;
        self.admit(&available, partition, context)
    }
}
impl Inputs for SourceInputs<'_, '_> {
    fn population_scope(&self) -> super::super::super::relations::ProducerScope {
        self.state.population_scope()
    }
    fn bind_rule(&mut self, rule: RuleIndex) {
        self.completion = self.completion.for_rule(rule);
        self.rule = rule;
    }
    fn dependency(
        &self,
        source: crate::domain::executable_program::types::InputSource,
    ) -> Result<Option<crate::domain::executable_program::types::RegionId>, OperatingError> {
        use crate::domain::executable_program::types::InputSource;
        if self
            .project
            .as_ref()
            .is_some_and(|project| project.provides(source))
        {
            return Ok(None);
        }
        if let InputSource::Observation(slot) = source {
            if self.observations.contains_key(&slot) {
                return Ok(None);
            }
        }
        let region = match source {
            InputSource::Region(id) => Some(id),
            InputSource::Observation(slot) => Some(
                *self
                    .program
                    .program()
                    .observation_regions()
                    .get(slot)
                    .ok_or(OperatingError::NotRun)?
                    .as_ref()
                    .map_err(Clone::clone)?,
            ),
            InputSource::ObservationMember { slot, member } => {
                let named = self
                    .program
                    .observation_metadata()
                    .ok_or(OperatingError::NotRun)?;
                let declaration = named.slots.get(slot).ok_or(OperatingError::NotRun)?;
                if declaration.rule != self.rule.0 {
                    return Err(OperatingError::NotRead);
                }
                let producer = named
                    .bindings
                    .get(declaration.rule)
                    .and_then(|bindings| bindings.get(declaration.input.0 as usize))
                    .and_then(|binding| binding.producers.get(member))
                    .ok_or(OperatingError::NotRun)?;
                match producer {
                    crate::domain::file_program::types::ResolvedObservationProducer::Output {
                        rule,
                        output,
                    } => {
                        if self
                            .file_outputs
                            .and_then(|outputs| outputs.get(rule.0))
                            .and_then(|roots| roots.get(output.0 as usize))
                            .is_some_and(|root| {
                                root.publication.is_some() || root.readiness.is_some()
                            })
                        {
                            return Ok(None);
                        }
                        let metadata = self.program.rule_metadata(*rule)?;
                        let root = metadata
                            .observations
                            .get(output.0 as usize)
                            .ok_or(OperatingError::NotRun)?;
                        if self
                            .completion
                            .for_rule(*rule)
                            .observation_admission(&root.reads)
                            != crate::domain::completion::types::ObservationAdmission::Ready
                        {
                            return Ok(None);
                        }
                        match metadata
                            .observations
                            .get(output.0 as usize)
                            .ok_or(OperatingError::NotRun)?
                            .source
                            .as_ref()
                            .map_err(Clone::clone)?
                        {
                            InputSource::Region(id) => Some(*id),
                            InputSource::Observation(slot) => Some(
                                *self
                                    .program
                                    .program()
                                    .observation_regions()
                                    .get(*slot)
                                    .ok_or(OperatingError::NotRun)?
                                    .as_ref()
                                    .map_err(Clone::clone)?,
                            ),
                            _ => None,
                        }
                    }
                    _ => None,
                }
            }
            _ => None,
        };
        match region {
            Some(id) if self.execution.published(self.state, id)?.is_none() => Ok(Some(id)),
            _ => Ok(None),
        }
    }

    fn supply(
        &mut self,
        source: crate::domain::executable_program::types::InputSource,
        partition: &Partition,
        context: &mut OperatorContext<'_>,
    ) -> Result<AvailableInput, OperatingError> {
        use crate::domain::executable_program::types::InputSource;
        if let Some(project) = &mut self.project {
            if let Some(provided) = project.supply(source, partition, context, &self.owner) {
                return provided;
            }
        }
        let admitted = match source {
            InputSource::Relation(relation) => {
                let actual_partition = self
                    .execution
                    .source_partition(ProducerRoot::Input(source))?;
                actual_partition.require_supply(partition)?;
                let rows = (self.read)(relation)?;
                let publication = self.owner.source(relation, &rows)?;
                return AvailableInput::source(
                    Rc::clone(&self.owner),
                    publication,
                    ProducerRoot::Input(source),
                    actual_partition,
                    self.completion
                        .coverage(&[crate::domain::trust::types::RelationRead {
                            relation,
                            kinds: vec![],
                        }]),
                    rows,
                );
            }
            InputSource::Hosted { rule, relation } => {
                let actual_partition = self
                    .execution
                    .source_partition(ProducerRoot::Input(source))?;
                actual_partition.require_supply(partition)?;
                // The compiler owns the request-source declaration. Admission uses
                // the immutable prepared snapshot and the current numbered corpus.
                self.program.rule_metadata(rule)?;
                let hosted = self
                    .program
                    .hosted()
                    .get(relation.0 as usize)
                    .ok_or(OperatingError::NotRun)?;
                let table = crate::domain::hosted::service::table_of_query(
                    hosted,
                    crate::domain::hosted::types::NumberedFiles(context.paths),
                    &mut context.symbols,
                    context.phase,
                );
                let publication = self.owner.computed(table)?;
                admit_published(&self.owner, publication, source, &actual_partition)
            }
            InputSource::Region(id) => self
                .execution
                .published(self.state, id)?
                .ok_or(OperatingError::NotRun),
            InputSource::Observation(slot) => {
                if let Some(input) = self.observations.get(&slot) {
                    return Ok(AvailableInput::published(input.clone()));
                }
                let union = *self
                    .program
                    .program()
                    .observation_regions()
                    .get(slot)
                    .ok_or(OperatingError::NotRun)?
                    .as_ref()
                    .map_err(Clone::clone)?;
                let input = self
                    .execution
                    .published(self.state, union)?
                    .ok_or(OperatingError::NotRun)?;
                let admitted = super::super::admission::alias_observation(
                    self.program.program(),
                    slot,
                    &input,
                    &self.owner,
                )?;
                self.observations.insert(slot, admitted.clone());
                Ok(admitted)
            }
            InputSource::EmptyObservation(slot) => {
                let named = self
                    .program
                    .observation_metadata()
                    .ok_or(OperatingError::NotRun)?;
                let declaration = named.slots.get(slot).ok_or(OperatingError::NotRun)?;
                if declaration.rule != self.rule.0 {
                    return Err(OperatingError::NotRead);
                }
                let binding = named
                    .bindings
                    .get(declaration.rule)
                    .and_then(|bindings| bindings.get(declaration.input.0 as usize))
                    .ok_or(OperatingError::NotRun)?;
                if !binding.producers.is_empty() {
                    return Err(OperatingError::NotRead);
                }
                let actual_partition = self
                    .execution
                    .source_partition(ProducerRoot::Input(source))?;
                actual_partition.require_supply(partition)?;
                let publication = self
                    .owner
                    .computed(ryo_values::domain::rows::types::Table {
                        kinds: binding
                            .schema
                            .columns
                            .iter()
                            .map(|column| column.kind)
                            .collect(),
                        count: ryo_values::domain::rows::types::RowCount(0),
                        cells: vec![],
                    })?;
                admit_published(&self.owner, publication, source, &actual_partition)
            }
            InputSource::ObservationMember { slot, member } => {
                let outputs =
                    self.file_outputs
                        .ok_or_else(|| OperatingError::ObservationIncomplete {
                            reason: "declared observation publication is unavailable".into(),
                        })?;
                let named = self
                    .program
                    .observation_metadata()
                    .ok_or(OperatingError::NotRun)?;
                let declaration = named.slots.get(slot).ok_or(OperatingError::NotRun)?;
                if declaration.rule != self.rule.0 {
                    return Err(OperatingError::NotRead);
                }
                let producer = named
                    .bindings
                    .get(declaration.rule)
                    .and_then(|bindings| bindings.get(declaration.input.0 as usize))
                    .and_then(|binding| binding.producers.get(member))
                    .ok_or(OperatingError::NotRun)?;
                let crate::domain::file_program::types::ResolvedObservationProducer::Output {
                    rule,
                    output,
                } = producer
                else {
                    return Err(OperatingError::ObservationIncomplete {
                        reason: "named producer has no executable query".into(),
                    });
                };
                let published = outputs
                    .get(rule.0)
                    .and_then(|roots| roots.get(output.0 as usize))
                    .and_then(|root| root.publication.clone());
                if published.is_none()
                    && outputs
                        .get(rule.0)
                        .and_then(|roots| roots.get(output.0 as usize))
                        .is_some_and(|root| root.readiness.is_some())
                {
                    return Err(OperatingError::ObservationIncomplete {
                        reason: "named producer root is not complete".into(),
                    });
                }
                let admitted = match published {
                    Some(input) => input,
                    None => {
                        let metadata = self.program.rule_metadata(*rule)?;
                        let root = metadata
                            .observations
                            .get(output.0 as usize)
                            .ok_or(OperatingError::NotRun)?;
                        if self
                            .completion
                            .for_rule(*rule)
                            .observation_admission(&root.reads)
                            != crate::domain::completion::types::ObservationAdmission::Ready
                        {
                            return Err(OperatingError::ObservationIncomplete {
                                reason: "physical source facts are incomplete".into(),
                            });
                        }
                        let source = *root.source.as_ref().map_err(Clone::clone)?;
                        let previous = self.rule;
                        self.bind_rule(*rule);
                        let admitted = self.admit_source(source, partition, context);
                        self.bind_rule(previous);
                        admitted?
                    }
                };
                if !admitted.observation_ready()? {
                    return Err(OperatingError::ObservationIncomplete {
                        reason: "producer publication has incomplete physical coverage".into(),
                    });
                }
                super::super::admission::alias_member(
                    self.program,
                    slot,
                    member,
                    &admitted,
                    partition,
                    &self.owner,
                )
            }
        }?;
        Ok(AvailableInput::published(admitted))
    }
    fn admit(
        &mut self,
        input: &AvailableInput,
        partition: &Partition,
        _context: &mut OperatorContext<'_>,
    ) -> Result<AdmittedInput, OperatingError> {
        input.admit(&self.owner, partition)
    }
    fn is_empty(
        &self,
        input: &AdmittedInput,
        representation: crate::domain::operator::contract::types::EmptyRepresentation,
    ) -> Result<bool, OperatingError> {
        let rows = input.rows(&self.owner, self.read)?;
        Ok(count_of(&rows).0 == 0
            && (representation
                == crate::domain::operator::contract::types::EmptyRepresentation::Any
                || matches!(rows, Rows::Computed(_))))
    }
    fn publish(
        &mut self,
        root: ProducerRoot,
        partition: &Partition,
        inputs: &[AdmittedInput],
        rows: Table,
    ) -> Result<AdmittedInput, OperatingError> {
        let actual_partition = self.execution.source_partition(root)?;
        actual_partition.require_supply(partition)?;
        let readiness =
            super::super::admission::contributor_readiness(&actual_partition, inputs, &self.owner)?;
        let publication = self.owner.computed(rows)?;
        self.state.bind(&self.owner, root, &publication)?;
        let resolved = self.owner.resolve(publication.as_ref(), self.read)?;
        let checked = self.owner.admit(
            publication.as_ref(),
            crate::domain::operator::contract::views::BorrowedRows::from(&resolved),
        )?;
        AdmittedInput::from_publication(
            Rc::clone(&self.owner),
            publication.clone(),
            &checked,
            root,
            actual_partition,
            readiness,
        )
    }
    fn publish_positions(
        &mut self,
        root: ProducerRoot,
        partition: &Partition,
        inputs: &[AdmittedInput],
        publication: PublishedInput,
    ) -> Result<AdmittedInput, OperatingError> {
        let actual_partition = self.execution.source_partition(root)?;
        actual_partition.require_supply(partition)?;
        let readiness =
            super::super::admission::contributor_readiness(&actual_partition, inputs, &self.owner)?;
        self.state.bind(&self.owner, root, &publication)?;
        let rows = self.owner.resolve(publication.as_ref(), self.read)?;
        let checked = self.owner.admit(
            publication.as_ref(),
            crate::domain::operator::contract::views::BorrowedRows::from(&rows),
        )?;
        AdmittedInput::from_publication(
            Rc::clone(&self.owner),
            publication.clone(),
            &checked,
            root,
            actual_partition,
            readiness,
        )
    }
    fn publish_selected(
        &mut self,
        root: ProducerRoot,
        partition: &Partition,
        inputs: &[AdmittedInput],
        publication: PublishedInput,
    ) -> Result<AdmittedInput, OperatingError> {
        let actual_partition = self.execution.source_partition(root)?;
        actual_partition.require_supply(partition)?;
        let readiness =
            super::super::admission::contributor_readiness(&actual_partition, inputs, &self.owner)?;
        self.state.bind(&self.owner, root, &publication)?;
        let rows = self.owner.resolve(publication.as_ref(), self.read)?;
        let checked = self.owner.admit(
            publication.as_ref(),
            crate::domain::operator::contract::views::BorrowedRows::from(&rows),
        )?;
        AdmittedInput::from_publication(
            Rc::clone(&self.owner),
            publication.clone(),
            &checked,
            root,
            actual_partition,
            readiness,
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
        let rows = input.rows(&self.owner, self.read)?;
        crate::application::file_dag::execution::output::replay(&rows, context, output)
    }
}

pub(super) fn table_of<'facts>(
    owner: &Rc<RelationOwner>,
    input: &AdmittedInput,
    read: &dyn Fn(Relation) -> Result<Rows<'facts>, OperatingError>,
) -> Result<Table, OperatingError> {
    let rows = input.rows(owner, read)?;
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

fn observation_slot(
    program: &PreparedProgram,
    rule: RuleIndex,
    input: ryo_values::domain::observations::types::ObservationBindingId,
) -> Result<usize, OperatingError> {
    let slots = &program
        .observation_metadata()
        .ok_or(OperatingError::NotRun)?
        .slots;
    let mut matching = slots
        .iter()
        .enumerate()
        .filter(|(_, slot)| slot.rule == rule.0 && slot.input == input);
    let (index, _) = matching.next().ok_or(OperatingError::NotRun)?;
    if matching.next().is_some() {
        return Err(OperatingError::WrongOperation);
    }
    Ok(index)
}
