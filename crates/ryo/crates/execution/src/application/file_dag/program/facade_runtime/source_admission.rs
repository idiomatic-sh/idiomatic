//! Demand-triggered source admission, publication, and replay for one program generation.
use crate::application::file_dag::program::admission::AdmittedInput;
use crate::application::file_dag::program::admission::AvailableInput;
use crate::application::file_dag::program::admission::Emptiness;
use crate::application::file_dag::program::admission::Inputs;
use crate::application::file_dag::program::admission::Partition;
use crate::application::file_dag::program::admission::ProducerRoot;
use crate::application::file_dag::program::admission::Representation;
use crate::application::file_dag::program::constants::PRODUCER_COVERAGE_INCOMPLETE;
use crate::application::file_dag::program::facade_runtime::types::CachedInput;
use crate::application::file_dag::program::facade_runtime::types::FileRoot;
use crate::application::file_dag::program::facade_runtime::types::ObservationKey;
use crate::application::file_dag::program::facade_runtime::types::Provision;
use crate::application::file_dag::program::facade_runtime::types::PublicationScope;
use crate::application::file_dag::program::preparation::types::PreparedProgram;
use crate::application::file_dag::program::runtime::service::ProgramExecution;
use crate::application::file_dag::relations::RelationOwner;
use crate::domain::completion::types::completion_policy::value::CompletionPolicy;
use crate::domain::coverage::types::RuleIndex;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::OperatorContext;
use crate::domain::table_model::service::count_of;
use crate::domain::table_model::service::materialize;
use crate::domain::table_model::types::Rows;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::rows::types::Table;
use std::collections::HashMap;
use std::rc::Rc;

pub(super) struct ProjectSources<'a> {
    pub(super) program: &'a PreparedProgram,
    pub(super) rule: RuleIndex,
    pub(super) sources: &'a [crate::application::file_dag::frontiers::types::FrontierSource],
    pub(super) ordinary: &'a [Rc<
        super::super::project_population::types::ProjectFrontierPopulation<AvailableInput>,
    >],
    pub(super) refused: &'a [crate::domain::crossing::types::RefusedFrontier],
    pub(super) bound:
        &'a mut [Option<crate::application::file_dag::frontiers::types::BoundInput>],
    pub(super) cache: &'a mut Vec<CachedInput>,
    pub(super) transport: &'a ryo_language::domain::symbols::types::TransportPermit,
    pub(super) publication: &'a PublicationScope,
}
mod acquisition;
mod project;
mod publication;

/// The application admission port is the only bridge from an executable source
/// to a generation-checked publication. It has no second scheduler.
pub(super) struct SourceInputs<'a, 'facts> {
    pub(super) execution: &'a ProgramExecution,
    pub(super) state: &'a crate::application::file_dag::program::runtime::service::RuntimeState,
    pub(super) completion: CompletionPolicy,
    pub(super) owner: Rc<RelationOwner>,
    pub(super) read: &'a dyn Fn(Relation) -> Result<Rows<'facts>, OperatingError>,
    pub(super) observations: &'a mut HashMap<ObservationKey, AdmittedInput>,
    pub(super) file_outputs: Option<&'a HashMap<RuleIndex, Vec<FileRoot>>>,
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
    fn population_scope(
        &self,
    ) -> super::super::super::relations::publication::types::ProducerScope {
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
            .is_some_and(|project| project.provides(source) == Provision::Provided)
        {
            return Ok(None);
        }
        if let InputSource::Observation(slot) = source {
            if self.observations.contains_key(&ObservationKey(slot)) {
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
                            .and_then(|outputs| outputs.get(rule))
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
        self.supply_source(source, partition, context)
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
    ) -> Result<Emptiness, OperatingError> {
        let rows = input.rows(&self.owner, self.read)?;
        Ok(
            if count_of(&rows).0 == 0
                && (representation
                    == crate::domain::operator::contract::types::EmptyRepresentation::Any
                    || matches!(rows, Rows::Computed(_)))
            {
                Emptiness::Empty
            } else {
                Emptiness::Populated
            },
        )
    }
    fn publish(
        &mut self,
        root: ProducerRoot,
        partition: &Partition,
        inputs: &[AdmittedInput],
        representation: Representation,
    ) -> Result<AdmittedInput, OperatingError> {
        self.publish_source(root, partition, inputs, representation)
    }
    fn replay(
        &self,
        input: &AdmittedInput,
        context: &mut OperatorContext<'_>,
        output: &mut dyn crate::domain::operating::types::Emit,
    ) -> Result<(), OperatingError> {
        self.replay_source(input, context, output)
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
    if input.observation_ready()?
        == crate::application::file_dag::program::admission::ObservationReadiness::Waiting
    {
        return Err(OperatingError::ObservationIncomplete {
            reason: PRODUCER_COVERAGE_INCOMPLETE.into(),
        });
    }
    table_of(owner, input, read)
}

pub(super) fn observation_slot(
    program: &PreparedProgram,
    rule: RuleIndex,
    input: ryo_values::domain::plan_parts::types::ObservationBindingId,
) -> Result<ObservationKey, OperatingError> {
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
    Ok(ObservationKey(index))
}
