//! Population proofs, partition identity and readiness are minted by admission only.
use super::super::{
    derived::population::types::PopulationId,
    relations::{RelationOwner, handle::service::PublishedInput},
};
use crate::domain::{
    coverage::types::RuleIndex,
    executable_program::types::{InputSource, RegionId},
    operating::errors::OperatingError,
};
use ryo_relations::domain::relations::types::FileId;
use std::{rc::Rc, sync::Arc};

#[derive(Clone)]
pub(super) struct FileGeneration {
    file: FileId,
    generation: Arc<()>,
}
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct RequiredPartitions {
    pub(super) request: Arc<super::request_identity::RequestIdentity>,
    pub(super) files: Box<[FileId]>,
    pub(super) covered: Box<[crate::domain::coverage::types::Covered]>,
    pub(super) skipped: Box<[ryo_values::domain::runs::types::Skipped]>,
    pub(super) inapplicable: Box<[FileId]>,
}
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApplicabilityReason {
    SyntaxRejected,
    DiagnosticNotApplicable,
}
#[derive(Clone)]
pub(super) enum Readiness {
    Complete,
    Partial { files: Vec<Option<FileId>> },
    Inapplicable { files: Vec<FileId> },
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
pub(super) struct AdmittedInput {
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
impl AvailableInput {
    /// Attach the demand's physical evidence without re-reading the cached
    /// source or forcing its deferred admission/geometry refusal.
    pub(super) fn with_coverage(
        self,
        coverage: crate::domain::completion::types::CoverageState,
    ) -> Self {
        let body = match self.body {
            AvailableBody::Published(input) => {
                AvailableBody::Published(input.with_coverage(coverage))
            }
            AvailableBody::Source {
                owner,
                publication,
                partition,
                admission,
            } => AvailableBody::Source {
                owner,
                publication,
                partition,
                admission: admission.map(|input| input.with_coverage(coverage)),
            },
        };
        Self { body }
    }
    pub(super) fn published(input: AdmittedInput) -> Self {
        Self {
            body: AvailableBody::Published(input),
        }
    }
    pub(super) fn source(
        owner: Rc<RelationOwner>,
        publication: PublishedInput,
        root: ProducerRoot,
        partition: Partition,
        coverage: crate::domain::completion::types::CoverageState,
        rows: crate::domain::table_model::types::Rows<'_>,
    ) -> Result<Self, OperatingError> {
        // Inspect this exact first provider borrow now, not a later re-read.
        // Only geometry refusals are deferred; identity/generation/source failures
        // remain immediate. No borrowed rows survive this synchronous boundary.
        let inspected = owner.inspect_available(
            publication.as_ref(),
            crate::domain::operator::contract::views::BorrowedRows::from(&rows),
        )?;
        let admission = match inspected {
            Ok(physical) => {
                let physical = if physical.physical_shape() == crate::domain::table_model::collections::representations::types::PhysicalShape::ReadablePhysicalFallback {
                    // Freeze the readable prefix from this first supplied borrow in
                    // the existing generation-owned normalization record. Its refusal
                    // remains deferred to the declared admission boundary.
                    drop(physical);
                    owner.normalize(&publication, rows).and_then(|normalized| {
                        owner.admit(publication.as_ref(), crate::domain::operator::contract::views::BorrowedRows::from(&normalized))
                            .and_then(|proof| AdmittedInput::from_publication(Rc::clone(&owner), publication.clone(), &proof,
                                root, partition.clone(), Readiness::Complete))
                    })
                } else {
                    AdmittedInput::from_publication(Rc::clone(&owner), publication.clone(), &physical,
                        root, partition.clone(), Readiness::Complete)
                };
                physical.map(|input| input.with_coverage(coverage))
            }
            Err(geometry) => Err(geometry),
        };
        Ok(Self {
            body: AvailableBody::Source {
                owner,
                publication,
                partition,
                admission,
            },
        })
    }
    pub(super) fn admit(
        &self,
        owner: &Rc<RelationOwner>,
        partition: &Partition,
    ) -> Result<AdmittedInput, OperatingError> {
        match &self.body {
            AvailableBody::Published(input) => {
                input.publication(owner)?;
                input.partition.require_supply(partition)?;
                Ok(input.clone())
            }
            AvailableBody::Source {
                owner: original,
                publication,
                partition: actual,
                admission,
            } => {
                if !Rc::ptr_eq(original, owner) {
                    return Err(OperatingError::NotRead);
                }
                actual.require_supply(partition)?;
                // Revalidate owner generation without obtaining another provider population.
                let _kinds = owner.publication_kinds(publication)?;
                admission.clone()
            }
        }
    }
}

/// Private to the common invocation. A semantic kernel cannot import this trait.
pub(super) trait Inputs {
    fn population_scope(&self) -> super::super::relations::publication::types::ProducerScope;
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
    ) -> Result<bool, OperatingError>;
    fn publish(
        &mut self,
        root: ProducerRoot,
        partition: &Partition,
        inputs: &[AdmittedInput],
        rows: ryo_values::domain::rows::types::Table,
    ) -> Result<AdmittedInput, OperatingError>;
    fn publish_positions(
        &mut self,
        root: ProducerRoot,
        partition: &Partition,
        inputs: &[AdmittedInput],
        produced: super::super::relations::handle::service::PublishedInput,
    ) -> Result<AdmittedInput, OperatingError>;
    fn publish_selected(
        &mut self,
        root: ProducerRoot,
        partition: &Partition,
        inputs: &[AdmittedInput],
        produced: super::super::relations::handle::service::PublishedInput,
    ) -> Result<AdmittedInput, OperatingError>;
    fn publish_empty(
        &mut self,
        root: ProducerRoot,
        partition: &Partition,
        inputs: &[AdmittedInput],
        schema: Result<Vec<ryo_relations::domain::relations::types::CellKind>, OperatingError>,
    ) -> Result<AdmittedInput, OperatingError>;
    fn replay(
        &self,
        input: &AdmittedInput,
        context: &mut crate::domain::operating::types::OperatorContext<'_>,
        output: &mut dyn crate::domain::operating::types::Emit,
    ) -> Result<(), OperatingError>;
}
impl AdmittedInput {
    pub(in crate::application::file_dag::program) fn population(
        &self,
        owner: &Rc<RelationOwner>,
    ) -> Result<
        crate::application::file_dag::derived::population::types::PopulationId,
        OperatingError,
    > {
        self.publication(owner)?;
        Ok(self.proof.population)
    }
    pub(super) fn has_root(&self, root: ProducerRoot) -> bool {
        self.root == root
    }
    /// Every admitted consumer uses the same physical-layout contract. A readable
    /// malformed provider is normalized once by its owner; complete native and
    /// compact populations remain borrowed in their original representation.
    pub(super) fn rows<'a, 'facts>(
        &'a self,
        owner: &'a Rc<RelationOwner>,
        read: &dyn Fn(
            ryo_relations::domain::relations::types::Relation,
        )
            -> Result<crate::domain::table_model::types::Rows<'facts>, OperatingError>,
    ) -> Result<crate::domain::table_model::types::Rows<'a>, OperatingError>
    where
        'facts: 'a,
    {
        let publication = self.publication(owner)?;
        let rows = owner.resolve(publication.as_ref(), read)?;
        owner.normalize(publication, rows)
    }
    pub(super) fn observation_ready(&self) -> Result<bool, OperatingError> {
        match &self.readiness {
            Readiness::Complete => Ok(true),
            Readiness::Partial { .. } | Readiness::Inapplicable { .. } => Ok(false),
        }
    }

    pub(super) fn coverage(
        &self,
    ) -> Result<crate::domain::completion::types::CoverageState, OperatingError> {
        match &self.readiness {
            Readiness::Complete => Ok(crate::domain::completion::types::CoverageState::Complete),
            Readiness::Partial { files } => {
                Ok(crate::domain::completion::types::CoverageState::Partial {
                    files: files.clone(),
                })
            }
            Readiness::Inapplicable { files } => Ok(
                crate::domain::completion::types::CoverageState::Inapplicable {
                    files: files.clone(),
                },
            ),
        }
    }
    pub(super) fn with_coverage(
        mut self,
        coverage: crate::domain::completion::types::CoverageState,
    ) -> Self {
        use crate::domain::completion::types::CoverageState;
        match coverage {
            CoverageState::Complete => {}
            CoverageState::Partial { mut files } => match &mut self.readiness {
                Readiness::Complete => self.readiness = Readiness::Partial { files },
                Readiness::Partial { files: existing } => {
                    for file in files.drain(..) {
                        if !existing.contains(&file) {
                            existing.push(file);
                        }
                    }
                }
                Readiness::Inapplicable { files: existing } => {
                    files.extend(existing.iter().copied().map(Some));
                    files.sort_unstable();
                    files.dedup();
                    self.readiness = Readiness::Partial { files };
                }
            },
            CoverageState::Inapplicable { files } => match &mut self.readiness {
                Readiness::Complete => self.readiness = Readiness::Inapplicable { files },
                Readiness::Partial { files: existing } => {
                    for file in files {
                        if !existing.contains(&Some(file)) {
                            existing.push(Some(file));
                        }
                    }
                }
                Readiness::Inapplicable { files: existing } => {
                    for file in files {
                        if !existing.contains(&file) {
                            existing.push(file);
                        }
                    }
                }
            },
        }
        self
    }

    pub(in crate::application::file_dag::program) fn validate_for(
        &self,
        owner: &Rc<RelationOwner>,
        generation: &Arc<()>,
        required: &Partition,
        expected: ProducerRoot,
    ) -> Result<(), OperatingError> {
        if !owner.matches_generation(generation)
            || !Rc::ptr_eq(owner, &self.proof.owner)
            || !Arc::ptr_eq(&self.proof.generation, generation)
            || self.root != expected
            || !self.partition.supplies(required)
        {
            return Err(OperatingError::NotRead);
        }
        self.publication(owner)?;
        Ok(())
    }
    pub(in crate::application::file_dag::program) fn validate_publication(
        &self,
    ) -> Result<(), OperatingError> {
        self.publication(&self.proof.owner).map(|_| ())
    }
    pub(in crate::application::file_dag::program) fn walk_source<'call>(
        &'call self,
        relation: ryo_relations::domain::relations::types::Relation,
        rows: &'call crate::domain::table_model::types::Rows<'call>,
        generation: &Arc<()>,
    ) -> Result<Option<AdmittedWalkSource<'call>>, OperatingError> {
        use crate::domain::{
            operator::contract::views::BorrowedRows,
            table_model::{service::kinds_of, types::Rows},
        };
        if self.root != ProducerRoot::Input(InputSource::Relation(relation)) {
            return Err(OperatingError::WrongOperation);
        }
        if !Arc::ptr_eq(&self.proof.generation, generation) {
            return Err(OperatingError::NotRead);
        }
        let publication = self.publication(&self.proof.owner)?;
        let admitted = self
            .proof
            .owner
            .admit(publication.as_ref(), BorrowedRows::from(rows))?;
        let (population, actual_relation) = admitted.identity(&self.proof.owner)?;
        if population != self.proof.population || actual_relation != Some(relation) {
            return Err(OperatingError::NotRead);
        }
        if kinds_of(rows) != ryo_relations::domain::relations::service::kinds(relation)
            || matches!(rows,Rows::Tree(tree) if crate::domain::table_cells::service::native_form(relation) != Some(tree.form))
        {
            return Ok(None);
        }
        Ok(Some(AdmittedWalkSource {
            source: self,
            rows,
            relation,
            population,
        }))
    }
    /// Only a real generation-checked admission can issue this required input.
    pub(in crate::application::file_dag::program) fn from_publication(
        owner: Rc<RelationOwner>,
        publication: PublishedInput,
        admitted: &super::super::relations::admission::service::AdmittedInput<'_>,
        root: ProducerRoot,
        partition: Partition,
        readiness: Readiness,
    ) -> Result<Self, OperatingError> {
        admitted.require_publication(owner.as_ref(), publication.as_ref())?;
        let (population, _) = admitted.identity(&owner)?;
        let generation = admitted.generation_guard();

        Ok(Self {
            proof: PopulationProof {
                owner,
                population,
                generation,
            },
            root,
            partition,
            readiness,
            publication,
        })
    }
    pub(in crate::application::file_dag::program) fn publication(
        &self,
        owner: &Rc<RelationOwner>,
    ) -> Result<&PublishedInput, OperatingError> {
        if !Rc::ptr_eq(owner, &self.proof.owner)
            || !owner.matches_generation(&self.proof.generation)
        {
            return Err(OperatingError::NotRead);
        }
        match &self.readiness {
            Readiness::Complete | Readiness::Partial { .. } | Readiness::Inapplicable { .. } => {
                Ok(&self.publication)
            }
        }
    }
}
impl FileGeneration {
    pub(in crate::application::file_dag::program) fn matches(
        &self,
        expected: &Self,
        generation: &Arc<()>,
    ) -> bool {
        self.file == expected.file
            && Arc::ptr_eq(&self.generation, &expected.generation)
            && Arc::ptr_eq(generation, &expected.generation)
    }
    pub(super) fn file(&self) -> FileId {
        self.file
    }
    pub(in crate::application::file_dag::program) fn from_owner(
        file: FileId,
        owner: &RelationOwner,
    ) -> Self {
        Self {
            file,
            generation: owner.generation_guard(),
        }
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
/// A borrowed semantic call can be made only under an admitted program invocation.
/// Its fields/issuance are inaccessible to raw evaluator or provider callers.

impl RequiredPartitions {
    pub(in crate::application::file_dag::program) fn from_request(
        request: &Arc<super::request_identity::RequestIdentity>,
        files: &[ryo_values::domain::files::types::CorpusInput],
        covered: &[crate::domain::coverage::types::Covered],
        skipped: &[ryo_values::domain::runs::types::Skipped],
        inapplicable: &[FileId],
    ) -> Result<Self, OperatingError> {
        if !request.matches_corpus(files) {
            return Err(OperatingError::NotRead);
        }
        let mut ids = Vec::with_capacity(files.len());
        for (at, _) in files.iter().enumerate() {
            ids.push(FileId(
                u32::try_from(at).map_err(|_| OperatingError::ShapeMismatch)?,
            ));
        }
        if covered
            .iter()
            .any(|entry| entry.file.0 as usize >= files.len())
            || inapplicable
                .iter()
                .any(|file| file.0 as usize >= files.len())
        {
            return Err(OperatingError::NotRead);
        }
        Ok(Self {
            request: Arc::clone(request),
            files: ids.into_boxed_slice(),
            covered: covered.to_vec().into_boxed_slice(),
            skipped: skipped.to_vec().into_boxed_slice(),
            inapplicable: inapplicable.to_vec().into_boxed_slice(),
        })
    }
    /// An explicit syntax rejection discharges transport, never completeness.
    /// Missing, unreadable, or unknown files cannot acquire this evidence.
    pub(in crate::application::file_dag) fn skipped_files(
        &self,
        program: &crate::application::file_dag::program::preparation::types::PreparedProgram,
        rule: RuleIndex,
    ) -> Result<std::collections::BTreeSet<FileId>, OperatingError> {
        use ryo_values::domain::runs::types::{RuleApplicability, SkipReason};
        let id = program.rule_id(rule)?;
        let mut files = std::collections::BTreeSet::new();
        for skipped in self.skipped.iter().filter(|skipped| &skipped.rule == id) {
            if program.applicability(rule)? != RuleApplicability::AcceptedSyntax
                || skipped.reason != SkipReason::SyntaxRejected
            {
                return Err(OperatingError::NotRead);
            }
            let file = self
                .request
                .available_file(&skipped.path)
                .ok_or(OperatingError::NotRead)?;
            if !files.insert(file) {
                return Err(OperatingError::NotRead);
            }
        }
        Ok(files)
    }
    pub(in crate::application) fn files(&self) -> &[FileId] {
        &self.files
    }
    /// The files the edge named as having no frame for the rule: ungoverned, or read by no language.
    pub(in crate::application) fn inapplicable(&self) -> &[FileId] {
        &self.inapplicable
    }
}

impl Partition {
    pub(super) fn require_supply(&self, required: &Self) -> Result<(), OperatingError> {
        self.supplies(required)
            .then_some(())
            .ok_or(OperatingError::NotRead)
    }
    fn supplies(&self, required: &Self) -> bool {
        match (self, required) {
            (Self::RequestInvariant, _) => true,
            (Self::File(actual), Self::File(required)) => {
                actual.file == required.file
                    && Arc::ptr_eq(&actual.generation, &required.generation)
            }
            (Self::Project(actual), Self::Project(required)) => actual == required,
            _ => false,
        }
    }
}
impl RequiredPartitions {
    pub(in crate::application::file_dag) fn same_request(&self, other: &Self) -> bool {
        self == other
    }
}

/// Preserve physical coverage of every admitted contributor. This is only the
/// input side of publication validation; the caller must still hold the
/// invocation's output/root authority.
pub(super) fn contributor_readiness(
    partition: &Partition,
    inputs: &[AdmittedInput],
    owner: &Rc<RelationOwner>,
) -> Result<Readiness, OperatingError> {
    let mut affected = Vec::new();
    let mut inapplicable = Vec::new();
    for input in inputs {
        input.publication(owner)?;
        if !input.partition.supplies(partition) {
            return Err(OperatingError::NotRead);
        }
        match &input.readiness {
            Readiness::Complete => {}
            Readiness::Partial { files } => {
                for file in files {
                    if !affected.contains(file) {
                        affected.push(*file);
                    }
                }
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
        for file in inapplicable {
            if !affected.contains(&Some(file)) {
                affected.push(Some(file));
            }
        }
        Ok(Readiness::Partial { files: affected })
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
    slot: usize,
    member: usize,
    input: &AdmittedInput,
    required: &Partition,
    owner: &Rc<RelationOwner>,
) -> Result<AdmittedInput, OperatingError> {
    let named = prepared
        .observation_metadata()
        .ok_or(OperatingError::NotRun)?;
    let declaration = named.slots.get(slot).ok_or(OperatingError::NotRun)?;
    let binding = named
        .bindings
        .get(declaration.rule)
        .and_then(|bindings| bindings.get(declaration.input.0 as usize))
        .ok_or(OperatingError::NotRun)?;
    let producer = binding
        .producers
        .get(member)
        .ok_or(OperatingError::NotRun)?;
    let crate::domain::file_program::types::ResolvedObservationProducer::Output { rule, output } =
        producer
    else {
        return Err(OperatingError::ObservationIncomplete {
            reason: "named producer has no executable query".into(),
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
    if input.root != expected || !input.partition.supplies(required) {
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
    alias.root = ProducerRoot::Input(InputSource::ObservationMember { slot, member });
    Ok(alias)
}

pub(super) fn alias_observation(
    program: &crate::domain::executable_program::types::ExecutableProgram,
    slot: usize,
    input: &AdmittedInput,
    owner: &Rc<RelationOwner>,
) -> Result<AdmittedInput, OperatingError> {
    let root = *program
        .observation_regions()
        .get(slot)
        .ok_or(OperatingError::NotRun)?
        .as_ref()
        .map_err(Clone::clone)?;
    if input.root != ProducerRoot::Region(root) {
        return Err(OperatingError::NotRead);
    }
    input.publication(owner)?;
    let mut alias = input.clone();
    alias.root = ProducerRoot::Input(InputSource::Observation(slot));
    Ok(alias)
}

impl ApplicabilityProof {
    pub(super) fn for_file(
        producer: RuleIndex,
        file: FileId,
        reason: ApplicabilityReason,
        owner: &RelationOwner,
    ) -> Self {
        Self {
            producer,
            file: FileGeneration::from_owner(file, owner),
            reason,
        }
    }
}

impl ApplicabilityProof {
    /// The declared producer for this positive applicability result.
    pub fn producer(&self) -> RuleIndex {
        self.producer
    }
    /// The file whose generation issued this fact; this cannot mint a new capability.
    pub fn file(&self) -> FileId {
        self.file.file
    }
    pub fn reason(&self) -> ApplicabilityReason {
        self.reason
    }
}
