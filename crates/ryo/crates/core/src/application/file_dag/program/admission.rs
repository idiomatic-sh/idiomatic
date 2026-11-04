//! Population proofs, partition identity and readiness are minted by admission only.
use super::super::{
    derived::owner::PopulationId,
    relations::{PublishedInput, RelationOwner},
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
    pub(super) files: Box<[FileId]>,
    pub(super) covered: Box<[crate::domain::coverage::types::Covered]>,
    pub(super) skipped: Box<[ryo_values::domain::runs::types::Skipped]>,
}
#[derive(Clone)]
pub(super) enum Partition {
    RequestInvariant,
    File(FileGeneration),
    Project(RequiredPartitions),
}
/// Positive applicability is evidence, not a Complete-empty population.
#[derive(Clone)]
pub(super) struct ApplicabilityProof {
    producer: RuleIndex,
    file: FileGeneration,
    reason: ApplicabilityReason,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ApplicabilityReason {
    SyntaxRejected,
    DiagnosticNotApplicable,
}
#[derive(Clone)]
pub(super) enum Readiness {
    Complete,
    Partial { files: Vec<Option<FileId>> },
    Skipped(ApplicabilityProof),
    Unknown(OperatingError),
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
/// Private to the common invocation. A semantic kernel cannot import this trait.
pub(super) trait Inputs {
    fn admit(
        &mut self,
        input: InputSource,
        partition: &Partition,
        context: &mut crate::domain::operating::types::OperatorContext<'_>,
    ) -> Result<AdmittedInput, OperatingError>;
    fn is_empty(&self, input: &AdmittedInput) -> Result<bool, OperatingError>;
    fn publish(
        &mut self,
        root: ProducerRoot,
        partition: &Partition,
        inputs: &[AdmittedInput],
        rows: ryo_values::domain::rows::types::Table,
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
    pub(super) fn observation_ready(&self) -> Result<bool, OperatingError> {
        match &self.readiness {
            Readiness::Complete => Ok(true),
            Readiness::Partial { .. } | Readiness::Skipped(_) => Ok(false),
            Readiness::Unknown(error) => Err(error.clone()),
        }
    }

    pub(super) fn coverage(
        &self,
    ) -> Result<super::super::completion::CoverageState, OperatingError> {
        match &self.readiness {
            Readiness::Complete => Ok(super::super::completion::CoverageState::Complete),
            Readiness::Partial { files } => Ok(super::super::completion::CoverageState::Partial {
                files: files.clone(),
            }),
            Readiness::Unknown(error) => Err(error.clone()),
            Readiness::Skipped(_) => Err(OperatingError::ObservationIncomplete {
                reason: "producer partition was skipped".into(),
            }),
        }
    }
    pub(super) fn with_coverage(
        mut self,
        coverage: super::super::completion::CoverageState,
    ) -> Self {
        if let super::super::completion::CoverageState::Partial { mut files } = coverage {
            match &mut self.readiness {
                Readiness::Complete => self.readiness = Readiness::Partial { files },
                Readiness::Partial { files: existing } => {
                    for file in files.drain(..) {
                        if !existing.contains(&file) {
                            existing.push(file);
                        }
                    }
                }
                Readiness::Skipped(_) | Readiness::Unknown(_) => {}
            }
        }
        self
    }

    /// Only a real generation-checked admission can issue this required input.
    pub(in crate::application::file_dag::program) fn from_publication(
        owner: Rc<RelationOwner>,
        publication: PublishedInput,
        admitted: &super::super::relations::AdmittedInput<'_>,
        root: ProducerRoot,
        partition: Partition,
        readiness: Readiness,
    ) -> Result<Self, OperatingError> {
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
            Readiness::Complete | Readiness::Partial { .. } => Ok(&self.publication),
            Readiness::Skipped(_) => Err(OperatingError::ObservationIncomplete {
                reason: "admitted producer partition is skipped, not complete".into(),
            }),
            Readiness::Unknown(error) => Err(error.clone()),
        }
    }
}
impl FileGeneration {
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

/// A borrowed semantic call can be made only under an admitted program invocation.
/// Its fields/issuance are inaccessible to raw evaluator or provider callers.
pub(crate) struct KernelPermit<'scope> {
    program: &'scope crate::domain::executable_program::types::ExecutableProgram,
    region: RegionId,
    instruction: usize,
    generation: &'scope Arc<()>,
}
impl<'scope> KernelPermit<'scope> {
    pub(in crate::application::file_dag::program) fn admitted(
        execution: &'scope super::runtime::ProgramExecution,
        region: RegionId,
        inputs: &[AdmittedInput],
    ) -> Result<Self, OperatingError> {
        let code = execution
            .program
            .regions
            .get(region.0)
            .ok_or(OperatingError::NotRun)?;
        if code.instructions.is_empty() || inputs.len() != code.inputs.len() {
            return Err(OperatingError::WrongOperation);
        }
        if !execution.owner.matches_generation(&execution.generation) {
            return Err(OperatingError::NotRead);
        }
        for (source, input) in code.inputs.iter().zip(inputs) {
            let expected = match source {
                InputSource::Region(id) => ProducerRoot::Region(*id),
                other => ProducerRoot::Input(*other),
            };
            if input.root != expected || !input.partition.supplies(execution.partition()) {
                return Err(OperatingError::NotRead);
            }
            input.publication(&execution.owner)?;
            if !Rc::ptr_eq(&execution.owner, &input.proof.owner)
                || !Arc::ptr_eq(&input.proof.generation, &execution.generation)
            {
                return Err(OperatingError::NotRead);
            }
        }
        Ok(Self {
            program: &execution.program,
            region,
            instruction: 0,
            generation: &execution.generation,
        })
    }
    /// A continuation borrows the already admitted invocation; no refcount,
    /// population walk, room or ledger operation occurs here.
    pub(in crate::application::file_dag::program) fn instruction(
        &self,
        at: usize,
    ) -> Result<Self, OperatingError> {
        self.program.regions[self.region.0]
            .instructions
            .get(at)
            .ok_or(OperatingError::NotRun)?;
        Ok(Self {
            program: self.program,
            region: self.region,
            instruction: at,
            generation: self.generation,
        })
    }
    pub(crate) fn kind(&self) -> ryo_values::domain::plans::types::OperationKind {
        self.program.regions[self.region.0].instructions[self.instruction]
            .contract
            .kind
    }
    pub(crate) fn operation(&self) -> &ryo_values::domain::plans::types::Operation {
        &self.program.regions[self.region.0].instructions[self.instruction].operation
    }
}

impl RequiredPartitions {
    pub(in crate::application::file_dag::program) fn from_request(
        files: &[ryo_values::domain::files::types::CorpusInput],
        covered: &[crate::domain::coverage::types::Covered],
        skipped: &[ryo_values::domain::runs::types::Skipped],
    ) -> Result<Self, OperatingError> {
        let mut ids = Vec::with_capacity(files.len());
        for (at, _) in files.iter().enumerate() {
            ids.push(FileId(
                u32::try_from(at).map_err(|_| OperatingError::ShapeMismatch)?,
            ));
        }
        if covered
            .iter()
            .any(|entry| entry.file.0 as usize >= files.len())
        {
            return Err(OperatingError::NotRead);
        }
        Ok(Self {
            files: ids.into_boxed_slice(),
            covered: covered.to_vec().into_boxed_slice(),
            skipped: skipped.to_vec().into_boxed_slice(),
        })
    }
    pub(in crate::application::file_dag::program) fn files(&self) -> &[FileId] {
        &self.files
    }
}

impl Partition {
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
    pub(super) fn same_request(&self, other: &Self) -> bool {
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
    for input in inputs {
        input.publication(owner)?;
        if !input.partition.supplies(partition) {
            return Err(OperatingError::NotRead);
        }
        if let Readiness::Partial { files } = &input.readiness {
            for file in files {
                if !affected.contains(file) {
                    affected.push(*file);
                }
            }
        }
    }
    if affected.is_empty() {
        Ok(Readiness::Complete)
    } else {
        Ok(Readiness::Partial { files: affected })
    }
}

/// Give a real producer publication its request-qualified member alias. The
/// binding is resolved from prepared metadata; callers cannot choose a producer.
pub(super) fn alias_member(
    prepared: &super::facade::PreparedProgram,
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
    input.publication(owner)?;
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
        .observation_regions
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
