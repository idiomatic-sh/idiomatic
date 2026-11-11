//! The `admission` application operation.

use super::types::{
    AliasMember, AliasSlot, AvailableBody, Emptiness, Partition, PartitionSupply, ProducerRoot,
};
use ryo_execution_domain::domain::{
    coverage::types::RuleIndex,
    executable_program::types::{InputSource, RegionId},
    operating::errors::OperatingError,
    rule_intent::types::ApplicabilityReason,
};
use ryo_execution_store::application::file_dag::relations::interface::RelationsPort as _;
use ryo_execution_store::application::file_dag::{
    derived::population::service::PopulationId,
    relations::{handle::service::Publication, service::RelationOwner},
};
use ryo_relations::domain::relations::types::FileId;
use std::{rc::Rc, sync::Arc};

#[derive(Clone)]
pub(crate) struct FileGeneration {
    pub(super) file: FileId,
    pub(super) generation: ryo_execution_store::application::file_dag::relations::generation::service::ResidentGeneration,
}

/// Positive applicability is evidence, not a Complete-empty population.
#[derive(Clone)]
pub struct ApplicabilityProof {
    pub(super) producer: RuleIndex,
    pub(super) file: FileGeneration,
    pub(super) reason: ApplicabilityReason,
}

/// What admission knows of one read: the partition it sits in and how much of it was read. Only
/// admission builds one: a whole physical read is `complete`, coverage lowers it, and a
/// published result carries what its admitted contributors carried. Callers pass this proof to
/// `AdmittedInput::from_publication` and never name a readiness.
#[derive(Clone)]
pub(crate) struct ReadProof {
    pub(super) partition: Partition,
    pub(super) readiness: ryo_execution_domain::domain::completion::types::CoverageState,
}

#[derive(Clone)]
pub(super) struct PopulationProof {
    pub(super) owner: Rc<RelationOwner>,
    pub(super) population: PopulationId,
    pub(super) generation: ryo_execution_store::application::file_dag::relations::generation::service::ResidentGeneration,
}

#[derive(Clone)]
pub(crate) struct AdmittedInput {
    pub(super) proof: PopulationProof,
    pub(super) root: ProducerRoot,
    pub(super) partition: Partition,
    pub(super) readiness: ryo_execution_domain::domain::completion::types::CoverageState,
    pub(super) publication: Publication,
}

/// Source availability does not authorize a kernel or expose a deferred geometry refusal.
/// Pure inspection uses the first borrow; its owned result is committed at the
/// declared schema boundary. No provider borrow survives suspension or is reread at commit.
#[derive(Clone)]
pub(crate) struct AvailableInput {
    pub(super) body: AvailableBody,
}

/// Private to the common invocation. A semantic kernel cannot import this trait.
pub(crate) trait Inputs {
    fn population_scope(
        &self,
    ) -> ryo_execution_store::application::file_dag::relations::publication::types::ProducerScope;
    /// A dependency is polled without entering another invocation. The owner
    /// resumes this same admission action after its producer reaches a terminal state.
    fn dependency(&self, _input: InputSource) -> Result<Option<RegionId>, OperatingError> {
        Ok(None)
    }
    fn bind_rule(&mut self, _rule: RuleIndex) {}
    /// Whether a region the file's admitted roots reach can have a row or a row error there. It cannot fail: a scope that
    /// decided nothing answers `Present`, and the region runs in full.
    fn presence(
        &mut self,
        _region: RegionId,
    ) -> ryo_execution_domain::domain::executable_program::presence::types::RegionPresence {
        ryo_execution_domain::domain::executable_program::presence::types::RegionPresence::Present
    }

    fn supply(
        &mut self,
        input: InputSource,
        partition: &Partition,
        context: &mut ryo_execution_domain::domain::operator::types::OperatorContext<'_>,
    ) -> Result<AvailableInput, OperatingError>;
    fn admit(
        &mut self,
        input: &AvailableInput,
        partition: &Partition,
        context: &mut ryo_execution_domain::domain::operator::types::OperatorContext<'_>,
    ) -> Result<AdmittedInput, OperatingError>;
    fn is_empty(
        &self,
        input: &AdmittedInput,
        representation: ryo_execution_domain::domain::operator::contract::types::EmptyRepresentation,
    ) -> Result<crate::application::file_dag::program::admission::types::Emptiness, OperatingError>;
    /// The one publication of a region result, in the representation the producer states.
    fn publish(
        &mut self,
        root: ProducerRoot,
        partition: &Partition,
        inputs: &[AdmittedInput],
        representation: Representation,
    ) -> Result<AdmittedInput, OperatingError>;
    fn replay(
        &self,
        input: &AdmittedInput,
        context: &mut ryo_execution_domain::domain::operator::types::OperatorContext<'_>,
        output: &mut dyn ryo_execution_domain::domain::operator::types::Emit,
    ) -> Result<(), OperatingError>;
}

/// What a publication holds, stated by the producer and never inferred by the publisher.
pub(crate) enum Representation {
    /// Rows the operators computed, empty or not: the owner stores them as computed rows.
    Computed(ryo_values::domain::rows::types::Table),
}

impl Representation {
    /// No rows over `schema`; a refused schema stays the producer's fault.
    pub(crate) fn empty(
        schema: Result<Vec<ryo_relations::domain::relations::types::CellKind>, OperatingError>,
    ) -> Result<Self, OperatingError> {
        let mut rows = ryo_values::domain::rows::types::Table::default();
        ryo_values::domain::rows::service::reset(&mut rows, &schema?);
        Ok(Self::Computed(rows))
    }
}

/// Borrowed source access issued only after exact relation, schema and owner admission.
/// No source backing or generation proof can be replaced independently.
pub(crate) struct AdmittedWalkSource<'a> {
    pub(super) source: &'a AdmittedInput,
    pub(super) rows: &'a ryo_execution_domain::domain::table_model::types::Rows<'a>,
    pub(super) relation: ryo_relations::domain::relations::types::Relation,
    pub(super) population: PopulationId,
}

impl<'a> AdmittedWalkSource<'a> {
    pub(crate) fn require_input(&self, input: &AdmittedInput) -> Result<(), OperatingError> {
        std::ptr::eq(self.source, input)
            .then_some(())
            .ok_or(OperatingError::NotRead)
    }
    pub(crate) fn rows(&self) -> &ryo_execution_domain::domain::table_model::types::Rows<'a> {
        self.rows
    }
    pub(crate) fn relation(&self) -> ryo_relations::domain::relations::types::Relation {
        self.relation
    }
    pub(crate) fn population(&self) -> PopulationId {
        self.population
    }
}

/// Preserve physical coverage of every admitted contributor. This is only the
/// input side of publication validation; the caller must still hold the
/// invocation's output/root authority.
pub(crate) fn contributed_proof(
    partition: &Partition,
    inputs: &[AdmittedInput],
    owner: &Rc<RelationOwner>,
) -> Result<ReadProof, OperatingError> {
    let readiness = contributed_readiness(partition, inputs, owner)?;
    Ok(ReadProof {
        partition: partition.clone(),
        readiness,
    })
}

pub(super) fn contributed_readiness(
    partition: &Partition,
    inputs: &[AdmittedInput],
    owner: &Rc<RelationOwner>,
) -> Result<ryo_execution_domain::domain::completion::types::CoverageState, OperatingError> {
    for input in inputs {
        input.publication(owner)?;
        if input.partition.supplies(partition) == PartitionSupply::Lacks {
            return Err(OperatingError::NotRead);
        }
    }
    ryo_execution_domain::domain::completion::service::contributed(
        inputs.iter().map(|input| &input.readiness),
    )
}

/// Give a real producer publication its request-qualified member alias. The
/// binding is resolved from prepared metadata; callers cannot choose a producer.
pub(crate) fn alias_member(
    prepared: &crate::application::file_dag::program::preparation::types::PreparedProgram,
    slot: AliasSlot,
    member: AliasMember,
    input: &AdmittedInput,
    required: &Partition,
    owner: &Rc<RelationOwner>,
) -> Result<AdmittedInput, OperatingError> {
    let named = prepared
        .observation_metadata()
        .ok_or(OperatingError::NotRun)?;
    let declaration = named.slots.get(slot.0).ok_or(OperatingError::NotRun)?;
    let binding = named
        .bindings
        .get(declaration.rule.0)
        .and_then(|bindings| bindings.get(declaration.input.0 as usize))
        .ok_or(OperatingError::NotRun)?;
    let producer = binding
        .producers
        .get(member.0)
        .ok_or(OperatingError::NotRun)?;
    let ryo_execution_domain::domain::observation_schedule::types::ResolvedObservationProducer::Output {
        rule,
        output,
    } = producer
    else {
        return Err(OperatingError::ObservationIncomplete {
            reason: crate::application::file_dag::program::constants::NO_EXECUTABLE_QUERY.into(),
        });
    };
    let root = prepared
        .rule_metadata(*rule)?
        .observations
        .get(output.0 as usize)
        .ok_or(OperatingError::NotRun)?;
    let expected = match *root.source.as_ref().map_err(Clone::clone)? {
        InputSource::Region(id) => ProducerRoot::Region(id),
        other => ProducerRoot::Input(other),
    };
    if input.root != expected || input.partition.supplies(required) == PartitionSupply::Lacks {
        return Err(OperatingError::NotRead);
    }
    let publication = input.publication(owner)?;
    let kinds = owner.kinds(publication)?;
    if kinds.len() != binding.schema.columns.len()
        || kinds
            .iter()
            .zip(&binding.schema.columns)
            .any(|(actual, declared)| *actual != declared.kind)
    {
        return Err(OperatingError::ShapeMismatch);
    }
    let mut alias = input.clone();
    alias.root = ProducerRoot::Input(InputSource::ObservationMember {
        slot: ryo_execution_domain::domain::executable_program::types::ObservationPlace(slot.0),
        member: ryo_execution_domain::domain::executable_program::types::ProducerMember(member.0),
    });
    Ok(alias)
}

pub(crate) fn alias_observation(
    program: &ryo_execution_domain::domain::executable_program::types::executable_program::value::ExecutableProgram,
    slot: AliasSlot,
    input: &AdmittedInput,
    owner: &Rc<RelationOwner>,
) -> Result<AdmittedInput, OperatingError> {
    let root = *program
        .observation_regions()
        .get(slot.0)
        .ok_or(OperatingError::NotRun)?
        .as_ref()
        .map_err(Clone::clone)?;
    if input.root != ProducerRoot::Region(root) {
        return Err(OperatingError::NotRead);
    }
    input.publication(owner)?;
    let mut alias = input.clone();
    alias.root = ProducerRoot::Input(InputSource::Observation(
        ryo_execution_domain::domain::executable_program::types::ObservationPlace(slot.0),
    ));
    Ok(alias)
}
