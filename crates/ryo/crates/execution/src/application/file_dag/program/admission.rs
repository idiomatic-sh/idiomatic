//! Population proofs, partition identity and readiness are minted by admission only.
use super::super::{
    derived::population::types::PopulationId,
    relations::{RelationOwner, handle::service::PublishedInput},
};

mod admitted;
mod available;
mod partitions;
/// Whether two admitted identities are the same.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::application::file_dag) enum Agreement {
    Same,
    Different,
}
use crate::domain::{
    coverage::types::RuleIndex,
    executable_program::types::{InputSource, RegionId},
    operating::errors::OperatingError,
    rule_intent::types::ApplicabilityReason,
};
use ryo_relations::domain::relations::types::FileId;
use std::{rc::Rc, sync::Arc};

#[derive(Clone)]
pub(super) struct FileGeneration {
    file: FileId,
    generation: Arc<()>,
}
/// The project identity is the request and its files, coverage and skips. `inapplicable` is one
/// rule's own evidence beside that identity: equality never reads it, so every identity check
/// (open, supplies, materialize, the frontier cache key) agrees across rules.
#[derive(Clone)]
pub(crate) struct RequiredPartitions {
    pub(super) request: Arc<super::request_identity::RequestIdentity>,
    pub(super) files: Box<[FileId]>,
    pub(super) covered: Box<[crate::domain::coverage::types::Covered]>,
    pub(super) skipped: Box<[ryo_values::domain::runs::types::Skipped]>,
    pub(super) inapplicable: Box<[FileId]>,
}
impl PartialEq for RequiredPartitions {
    fn eq(&self, other: &Self) -> bool {
        self.request == other.request
            && self.files == other.files
            && self.covered == other.covered
            && self.skipped == other.skipped
    }
}
impl Eq for RequiredPartitions {}
#[derive(Clone)]
pub(super) enum Partition {
    RequestInvariant,
    File(FileGeneration),
    Project(RequiredPartitions),
}
/// Positive applicability is evidence, not a Complete-empty population.
#[derive(Clone)]
pub struct ApplicabilityProof {
    producer: RuleIndex,
    file: FileGeneration,
    reason: ApplicabilityReason,
}
/// Whether an admitted input has no rows in a given representation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::application::file_dag::program) enum Emptiness {
    Empty,
    Populated,
}
#[derive(Clone)]
enum Readiness {
    Complete,
    Partial {
        files: Vec<Option<FileId>>,
        lacks: Vec<crate::domain::completion::types::FileLacks>,
    },
    Inapplicable {
        files: Vec<FileId>,
    },
}
/// What admission knows of one read: the partition it sits in and how much of it was read. Only
/// admission builds one: a whole physical read is `complete`, coverage lowers it, and a
/// published result carries what its admitted contributors carried. Callers pass this proof to
/// `AdmittedInput::from_publication` and never name a readiness.
#[derive(Clone)]
pub(super) struct ReadProof {
    partition: Partition,
    readiness: Readiness,
}
/// Root provenance remains attached even when no rows can be supplied.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ProducerRoot {
    Input(InputSource),
    Region(RegionId),
}
#[derive(Clone)]
struct PopulationProof {
    owner: Rc<RelationOwner>,
    population: PopulationId,
    generation: Arc<()>,
}
#[derive(Clone)]
pub(crate) struct AdmittedInput {
    proof: PopulationProof,
    root: ProducerRoot,
    partition: Partition,
    readiness: Readiness,
    publication: PublishedInput,
}
/// Source availability does not authorize a kernel or expose a deferred geometry refusal.
/// Pure inspection uses the first borrow; its owned result is committed at the
/// declared schema boundary. No provider borrow survives suspension or is reread at commit.
#[derive(Clone)]
pub(super) struct AvailableInput {
    body: AvailableBody,
}
#[derive(Clone)]
enum AvailableBody {
    Published(AdmittedInput),
    Source {
        owner: Rc<RelationOwner>,
        publication: PublishedInput,
        partition: Partition,
        admission: Result<AdmittedInput, OperatingError>,
    },
}

/// Whether a partition covers the partition a consumer requires.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PartitionSupply {
    Supplies,
    Lacks,
}
/// Whether an admitted input is complete enough to feed a named observation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ObservationReadiness {
    Ready,
    Waiting,
}
/// The observation slot an alias is issued for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct AliasSlot(pub usize);
/// The member of a named observation an alias is issued for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct AliasMember(pub usize);

/// Private to the common invocation. A semantic kernel cannot import this trait.
pub(super) trait Inputs {
    fn population_scope(
        &self,
    ) -> crate::application::file_dag::relations::publication::types::ProducerScope;
    /// A dependency is polled without entering another invocation. The owner
    /// resumes this same admission action after its producer reaches a terminal state.
    fn dependency(
        &self,
        _input: InputSource,
    ) -> Result<Option<crate::domain::executable_program::types::RegionId>, OperatingError> {
        Ok(None)
    }
    fn bind_rule(&mut self, _rule: crate::domain::coverage::types::RuleIndex) {}

    fn supply(
        &mut self,
        input: InputSource,
        partition: &Partition,
        context: &mut crate::domain::operating::types::OperatorContext<'_>,
    ) -> Result<AvailableInput, OperatingError>;
    fn admit(
        &mut self,
        input: &AvailableInput,
        partition: &Partition,
        context: &mut crate::domain::operating::types::OperatorContext<'_>,
    ) -> Result<AdmittedInput, OperatingError>;
    fn is_empty(
        &self,
        input: &AdmittedInput,
        representation: crate::domain::operator::contract::types::EmptyRepresentation,
    ) -> Result<crate::application::file_dag::program::admission::Emptiness, OperatingError>;
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
        context: &mut crate::domain::operating::types::OperatorContext<'_>,
        output: &mut dyn crate::domain::operating::types::Emit,
    ) -> Result<(), OperatingError>;
}

/// What a publication holds, stated by the producer and never inferred by the publisher.
pub(in crate::application::file_dag::program) enum Representation {
    /// Rows the operators computed, empty or not: the owner stores them as computed rows.
    Computed(ryo_values::domain::rows::types::Table),
}
impl Representation {
    /// No rows over `schema`; a refused schema stays the producer's fault.
    pub(in crate::application::file_dag::program) fn empty(
        schema: Result<Vec<ryo_relations::domain::relations::types::CellKind>, OperatingError>,
    ) -> Result<Self, OperatingError> {
        let mut rows = ryo_values::domain::rows::types::Table::default();
        ryo_values::domain::rows::service::reset(&mut rows, &schema?);
        Ok(Self::Computed(rows))
    }
}

/// Borrowed source access issued only after exact relation, schema and owner admission.
/// No source backing or generation proof can be replaced independently.
pub(super) struct AdmittedWalkSource<'a> {
    source: &'a AdmittedInput,
    rows: &'a crate::domain::table_model::types::Rows<'a>,
    relation: ryo_relations::domain::relations::types::Relation,
    population: PopulationId,
}
impl<'a> AdmittedWalkSource<'a> {
    pub(in crate::application::file_dag::program) fn require_input(
        &self,
        input: &AdmittedInput,
    ) -> Result<(), OperatingError> {
        std::ptr::eq(self.source, input)
            .then_some(())
            .ok_or(OperatingError::NotRead)
    }
    pub(super) fn rows(&self) -> &crate::domain::table_model::types::Rows<'a> {
        self.rows
    }
    pub(super) fn relation(&self) -> ryo_relations::domain::relations::types::Relation {
        self.relation
    }
    pub(super) fn population(&self) -> PopulationId {
        self.population
    }
}

/// Preserve physical coverage of every admitted contributor. This is only the
/// input side of publication validation; the caller must still hold the
/// invocation's output/root authority.
pub(super) fn contributed_proof(
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

fn contributed_readiness(
    partition: &Partition,
    inputs: &[AdmittedInput],
    owner: &Rc<RelationOwner>,
) -> Result<Readiness, OperatingError> {
    let mut affected = Vec::new();
    let mut inapplicable = Vec::new();
    for input in inputs {
        input.publication(owner)?;
        if input.partition.supplies(partition) == PartitionSupply::Lacks {
            return Err(OperatingError::NotRead);
        }
        match &input.readiness {
            Readiness::Complete => {}
            Readiness::Partial { files, lacks } => {
                affected.extend(crate::domain::completion::service::lacks_of(files, lacks));
            }
            Readiness::Inapplicable { files } => {
                for file in files {
                    if !inapplicable.contains(file) {
                        inapplicable.push(*file);
                    }
                }
            }
        }
    }
    if inputs
        .iter()
        .any(|input| matches!(input.readiness, Readiness::Partial { .. }))
    {
        affected.extend(inapplicable.into_iter().map(|file| {
            crate::domain::completion::types::FileLacks {
                file: Some(file),
                missing: crate::domain::completion::types::MissingRows::Any,
            }
        }));
        match crate::domain::completion::service::partial_of_lacks(affected) {
            crate::domain::completion::types::CoverageState::Partial { files, lacks } => {
                Ok(Readiness::Partial { files, lacks })
            }
            _ => Err(OperatingError::NotRun),
        }
    } else if inapplicable.is_empty() {
        Ok(Readiness::Complete)
    } else {
        Ok(Readiness::Inapplicable {
            files: inapplicable,
        })
    }
}

/// Give a real producer publication its request-qualified member alias. The
/// binding is resolved from prepared metadata; callers cannot choose a producer.
pub(super) fn alias_member(
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
    let crate::domain::file_program::types::ResolvedObservationProducer::Output { rule, output } =
        producer
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
    let kinds = owner.publication_kinds(publication)?;
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
        slot: crate::domain::executable_program::types::ObservationPlace(slot.0),
        member: crate::domain::executable_program::types::ProducerMember(member.0),
    });
    Ok(alias)
}

pub(super) fn alias_observation(
    program: &crate::domain::executable_program::types::executable_program::value::ExecutableProgram,
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
    alias.root = ProducerRoot::Input(InputSource::Observation(crate::domain::executable_program::types::ObservationPlace(slot.0)));
    Ok(alias)
}
