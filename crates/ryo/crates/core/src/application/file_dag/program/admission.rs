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
pub(super) enum ApplicabilityReason {
    SyntaxRejected,
    DiagnosticNotApplicable,
}
#[derive(Clone)]
pub(super) enum Readiness {
    Complete,
    Partial,
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
    pub(super) fn partial(mut self) -> Self {
        self.readiness = Readiness::Partial;
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
            || !Arc::ptr_eq(&self.proof.generation, &owner.generation.borrow())
        {
            return Err(OperatingError::NotRead);
        }
        match &self.readiness {
            Readiness::Complete | Readiness::Partial => Ok(&self.publication),
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
            generation: Arc::clone(&owner.generation.borrow()),
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
        if !Arc::ptr_eq(&execution.generation, &execution.owner.generation.borrow()) {
            return Err(OperatingError::NotRead);
        }
        for (source, input) in code.inputs.iter().zip(inputs) {
            let expected = match source {
                InputSource::Region(id) => ProducerRoot::Region(*id),
                other => ProducerRoot::Input(*other),
            };
            if input.root != expected || !input.partition.supplies(&execution.partition) {
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
