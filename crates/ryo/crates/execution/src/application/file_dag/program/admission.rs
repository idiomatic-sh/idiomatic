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
    pub(super) request: Arc<super::request_identity::RequestIdentity>,
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
        produced: super::super::relations::PublishedInput,
    ) -> Result<AdmittedInput, OperatingError>;
    fn publish_selected(
        &mut self,
        root: ProducerRoot,
        partition: &Partition,
        inputs: &[AdmittedInput],
        produced: super::super::relations::PublishedInput,
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
            Readiness::Partial { .. } => Ok(false),
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
        }
    }
    pub(super) fn with_coverage(
        mut self,
        coverage: crate::domain::completion::types::CoverageState,
    ) -> Self {
        if let crate::domain::completion::types::CoverageState::Partial { mut files } = coverage {
            match &mut self.readiness {
                Readiness::Complete => self.readiness = Readiness::Partial { files },
                Readiness::Partial { files: existing } => {
                    for file in files.drain(..) {
                        if !existing.contains(&file) {
                            existing.push(file);
                        }
                    }
                }
            }
        }
        self
    }

    fn validate_for(
        &self,
        execution: &super::runtime::ProgramExecution,
        expected: ProducerRoot,
    ) -> Result<(), OperatingError> {
        if !execution.owner.matches_generation(&execution.generation)
            || !Rc::ptr_eq(&execution.owner, &self.proof.owner)
            || !Arc::ptr_eq(&self.proof.generation, &execution.generation)
            || self.root != expected
            || !self.partition.supplies(execution.partition())
        {
            return Err(OperatingError::NotRead);
        }
        self.publication(&execution.owner)?;
        Ok(())
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
        if let ProducerRoot::Region(region) = root {
            owner.bind_producer(region, publication.as_ref())?;
        }

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

/// Borrowed native access issued by the same population admission as every representation.
/// Private fields prevent pairing a valid generation with an unrelated tree.
pub(super) struct AdmittedTree<'a> {
    source: &'a AdmittedInput,
    tree: crate::domain::table_cells::types::TreeRows<'a>,
    population: PopulationId,
}
impl AdmittedTree<'_> {
    pub(super) fn rows(&self) -> crate::domain::table_cells::types::TreeRows<'_> {
        self.tree
    }
    pub(super) fn population(&self) -> PopulationId {
        self.population
    }
}
/// A borrowed semantic call can be made only under an admitted program invocation.
/// Its fields/issuance are inaccessible to raw evaluator or provider callers.
#[derive(Clone, Copy)]
enum KernelStrategy {
    Registered,
    Positions,
    Ordered,
    Column,
}
#[derive(Clone, Copy)]
enum BoundInputs<'a> {
    Ordinary(&'a [AdmittedInput]),
    Ordered(&'a AdmittedInput),
    Column {
        left: &'a AdmittedInput,
        right: &'a AdmittedInput,
    },
}
pub(crate) struct KernelPermit<'scope> {
    program: &'scope crate::domain::executable_program::types::ExecutableProgram,
    region: RegionId,
    instruction: usize,
    generation: &'scope Arc<()>,
    live: &'scope Arc<super::super::relations::GenerationLease>,
    bindings: BoundInputs<'scope>,
    strategy: KernelStrategy,
    evaluation: crate::domain::operator::contract::types::EvaluationMode,
}
impl<'scope> KernelPermit<'scope> {
    fn code(
        execution: &super::runtime::ProgramExecution,
        region: RegionId,
    ) -> Result<&crate::domain::executable_program::types::Region, OperatingError> {
        if !execution.owner.matches_generation(&execution.generation) {
            return Err(OperatingError::NotRead);
        }
        let code = execution
            .program
            .regions()
            .get(region.0)
            .ok_or(OperatingError::NotRun)?;
        if code.instructions.is_empty() {
            return Err(OperatingError::NotRun);
        }
        Ok(code)
    }
    pub(in crate::application::file_dag::program) fn admitted(
        execution: &'scope super::runtime::ProgramExecution,
        region: RegionId,
        inputs: &'scope [AdmittedInput],
        evaluation: crate::domain::operator::contract::types::EvaluationMode,
    ) -> Result<Self, OperatingError> {
        let code = Self::code(execution, region)?;
        if code.instructions.is_empty() || inputs.len() != code.inputs.len() {
            return Err(OperatingError::WrongOperation);
        }
        for (source, input) in code.inputs.iter().zip(inputs) {
            let expected = match source {
                InputSource::Region(id) => ProducerRoot::Region(*id),
                other => ProducerRoot::Input(*other),
            };
            input.validate_for(execution, expected)?;
        }
        Ok(Self {
            program: &execution.program,
            region,
            instruction: 0,
            generation: &execution.generation,
            live: &execution.generation_live,
            bindings: BoundInputs::Ordinary(inputs),
            strategy: KernelStrategy::Registered,
            evaluation,
        })
    }
    pub(super) fn ordered(
        execution: &'scope super::runtime::ProgramExecution,
        region: RegionId,
        input: &'scope AdmittedInput,
        evaluation: crate::domain::operator::contract::types::EvaluationMode,
    ) -> Result<Self, OperatingError> {
        let code = Self::code(execution, region)?;
        let shape = code
            .ordered_existence
            .as_ref()
            .ok_or(OperatingError::WrongOperation)?;
        input.validate_for(execution, ProducerRoot::Region(shape.source))?;
        Ok(Self {
            program: &execution.program,
            region,
            instruction: 0,
            generation: &execution.generation,
            live: &execution.generation_live,
            bindings: BoundInputs::Ordered(input),
            strategy: KernelStrategy::Ordered,
            evaluation,
        })
    }
    pub(super) fn column(
        execution: &'scope super::runtime::ProgramExecution,
        region: RegionId,
        left: &'scope AdmittedInput,
        right: &'scope AdmittedInput,
    ) -> Result<Self, OperatingError> {
        let code = Self::code(execution, region)?;
        let shape = code.column.as_ref().ok_or(OperatingError::WrongOperation)?;
        left.validate_for(execution, ProducerRoot::Region(shape.left()))?;
        right.validate_for(
            execution,
            ProducerRoot::Input(InputSource::Relation(shape.chain().right)),
        )?;
        Ok(Self {
            program: &execution.program,
            region,
            instruction: 0,
            generation: &execution.generation,
            live: &execution.generation_live,
            bindings: BoundInputs::Column { left, right },
            strategy: KernelStrategy::Column,
            evaluation: crate::domain::operator::contract::types::EvaluationMode::Collected,
        })
    }
    /// A continuation borrows the already admitted invocation; no refcount,
    /// population walk, room or ledger operation occurs here.
    pub(in crate::application::file_dag::program) fn instruction(
        &self,
        at: usize,
    ) -> Result<Self, OperatingError> {
        self.program.regions()[self.region.0]
            .instructions
            .get(at)
            .ok_or(OperatingError::NotRun)?;
        Ok(Self {
            program: self.program,
            region: self.region,
            instruction: at,
            generation: self.generation,
            live: self.live,
            bindings: self.bindings,
            strategy: self.strategy,
            evaluation: self.evaluation,
        })
    }
    pub(super) fn native_shape(
        &self,
    ) -> Result<&Arc<crate::domain::lowering::types::LoweredShape>, OperatingError> {
        self.program.regions()[self.region.0]
            .native_positions
            .as_ref()
            .ok_or(OperatingError::WrongOperation)
    }
    pub(super) fn region(&self) -> RegionId {
        self.region
    }
    pub(super) fn validate_program(
        &self,
        program: &crate::domain::executable_program::types::ExecutableProgram,
    ) -> Result<(), OperatingError> {
        self.validate_live()?;
        if std::ptr::eq(self.program, program) {
            Ok(())
        } else {
            Err(OperatingError::WrongOperation)
        }
    }
    pub(super) fn descriptor(&self) -> &crate::domain::executable_program::types::Instruction {
        &self.program.regions()[self.region.0].instructions[self.instruction]
    }
    pub(super) fn position_semantics(
        &self,
        shape: &Arc<crate::domain::lowering::types::LoweredShape>,
    ) -> Result<SemanticPermit<'_>, OperatingError> {
        self.validate_registered(self.operation())?;
        let actual = self.program.regions()[self.region.0]
            .native_positions
            .as_ref()
            .ok_or(OperatingError::WrongOperation)?;
        if !Arc::ptr_eq(actual, shape)
            || self.evaluation
                != crate::domain::operator::contract::types::EvaluationMode::Collected
        {
            return Err(OperatingError::WrongOperation);
        }
        let mut permit = self.semantics();
        permit.strategy = KernelStrategy::Positions;
        Ok(permit)
    }
    pub(in crate::application::file_dag) fn semantics(&self) -> SemanticPermit<'_> {
        SemanticPermit {
            program: self.program,
            region: self.region,
            instruction: self.instruction,
            live: self.live,
            strategy: self.strategy,
        }
    }
    pub(super) fn bound_input(&self, at: usize) -> Result<&AdmittedInput, OperatingError> {
        match self.bindings {
            BoundInputs::Ordinary(inputs) => inputs.get(at),
            BoundInputs::Ordered(input) => (at == 0).then_some(input),
            BoundInputs::Column { left, right } => match at {
                0 => Some(left),
                1 => Some(right),
                _ => None,
            },
        }
        .ok_or(OperatingError::MissingInput { position: at + 1 })
    }
    pub(super) fn validate_live(&self) -> Result<(), OperatingError> {
        if self.live.is_live() {
            Ok(())
        } else {
            Err(OperatingError::NotRead)
        }
    }
    pub(super) fn admit_tree<'call>(
        &'call self,
        tree: crate::domain::table_cells::types::TreeRows<'call>,
    ) -> Result<AdmittedTree<'call>, OperatingError> {
        self.validate_live()?;
        let source = self.bound_input(0)?;
        if source.root
            != ProducerRoot::Input(InputSource::Relation(
                ryo_relations::domain::relations::types::Relation::SyntaxTree,
            ))
        {
            return Err(OperatingError::WrongOperation);
        }
        let publication = source.publication(&source.proof.owner)?;
        source.proof.owner.admit(
            publication.as_ref(),
            crate::domain::operator::contract::views::BorrowedRows::Tree(tree),
        )?;
        Ok(AdmittedTree {
            source,
            tree,
            population: source.proof.population,
        })
    }
    pub(super) fn walk<'call, 'context>(
        &'call self,
        prepared: &'call super::facade::PreparedProgram,
        source: AdmittedTree<'call>,
        context: &'call mut crate::domain::operating::types::OperatorContext<'context>,
        active: &'call [crate::domain::shared_walk::types::RuleActivity],
        work: &'call mut crate::domain::shared_walk::types::WalkWork,
    ) -> Result<super::native_walk::PermittedWalk<'call, 'context, 'scope>, OperatingError> {
        self.validate_live()?;
        if !std::ptr::eq(source.source, self.bound_input(0)?) {
            return Err(OperatingError::NotRead);
        }
        if !std::ptr::eq(self.program, prepared.program().as_ref()) {
            return Err(OperatingError::WrongOperation);
        }
        let instruction = &self.program.regions()[self.region.0].instructions[self.instruction];
        if prepared.walk_class(instruction).is_none() {
            return Err(OperatingError::WrongOperation);
        }
        Ok(super::native_walk::PermittedWalk {
            permit: self,
            prepared,
            source,
            context,
            active,
            work,
        })
    }
    pub(super) fn selection_input<'a>(
        &'a self,
        shape: &crate::domain::executable_program::selection::SelectionAlternative,
    ) -> Result<&'a AdmittedInput, OperatingError> {
        self.validate_live()?;
        if self.evaluation != crate::domain::operator::contract::types::EvaluationMode::Collected {
            return Err(OperatingError::WrongOperation);
        }
        let region = &self.program.regions()[self.region.0];
        let declared = region
            .selection
            .as_ref()
            .ok_or(OperatingError::WrongOperation)?;
        if shape.target() != self.region
            || shape.logical() != region.instructions[self.instruction].logical
            || !Arc::ptr_eq(shape.selector(), declared.selector())
        {
            return Err(OperatingError::NotRead);
        }
        let source = self.bound_input(0)?;
        source.publication(&source.proof.owner)?;
        Ok(source)
    }
    pub(super) fn collected_kinds(
        &self,
    ) -> Result<&[ryo_relations::domain::relations::types::CellKind], OperatingError> {
        self.program.regions()[self.region.0].instructions[self.instruction]
            .contract
            .collected_kinds
            .as_deref()
            .map_err(Clone::clone)
    }
    pub(super) fn validate_native_join(
        &self,
        native: &super::super::invocation::NativeJoinLease<'_>,
        owner: &Rc<RelationOwner>,
        left: &crate::domain::table_model::types::Rows<'_>,
    ) -> Result<(), OperatingError> {
        self.validate_registered(self.operation())?;
        if !matches!(
            self.native_join_eligibility(),
            super::native_join::Eligibility::Native
        ) {
            return Err(OperatingError::WrongOperation);
        }
        let region = &self.program.regions()[self.region.0];
        if region.instructions.len() != 1 || region.inputs.len() != 2 {
            return Err(OperatingError::WrongOperation);
        }
        let left_input = self.bound_input(0)?;
        let right_input = self.bound_input(1)?;
        let expected = region.instructions[self.instruction]
            .contract
            .derived
            .collected[ryo_derived::domain::contract::types::DerivedKind::NativeLookup.ordinal()]
        .as_ref()
        .ok_or(OperatingError::WrongOperation)?;
        if !std::ptr::eq(expected.primary.plan.as_ref(), native.contract()) {
            return Err(OperatingError::NotRead);
        }
        owner.admit(
            left_input.publication(owner)?.as_ref(),
            crate::domain::operator::contract::views::BorrowedRows::from(left),
        )?;
        owner.admit(
            right_input.publication(owner)?.as_ref(),
            crate::domain::operator::contract::views::BorrowedRows::Tree(native.tree()),
        )?;
        native.finding()?;
        Ok(())
    }
    pub(super) fn native_join_eligibility(&self) -> super::native_join::Eligibility {
        let region = &self.program.regions()[self.region.0];
        if self.evaluation == crate::domain::operator::contract::types::EvaluationMode::Collected
            && region.instructions.len() == 1
            && matches!(
                self.operation(),
                ryo_values::domain::plans::types::Operation::Join { reach: None, filter, .. } if filter.is_empty()
            )
            && region.instructions[self.instruction]
                .contract
                .schema
                .is_ok()
            && region.instructions[self.instruction]
                .contract
                .row_error
                .is_none()
        {
            super::native_join::Eligibility::Native
        } else {
            super::native_join::Eligibility::Ordinary
        }
    }
    pub(in crate::application::file_dag) fn validate_registered(
        &self,
        operation: &ryo_values::domain::plans::types::Operation,
    ) -> Result<(), OperatingError> {
        self.validate_live()?;
        if !matches!(self.strategy, KernelStrategy::Registered) || self.operation() != operation {
            return Err(OperatingError::WrongOperation);
        }
        Ok(())
    }
    pub(crate) fn kind(&self) -> ryo_values::domain::plans::types::OperationKind {
        self.program.regions()[self.region.0].instructions[self.instruction]
            .contract
            .kind
    }
    pub(crate) fn operation(&self) -> &ryo_values::domain::plans::types::Operation {
        &self.program.regions()[self.region.0].instructions[self.instruction].operation
    }
}

impl RequiredPartitions {
    pub(in crate::application::file_dag::program) fn from_request(
        request: &Arc<super::request_identity::RequestIdentity>,
        files: &[ryo_values::domain::files::types::CorpusInput],
        covered: &[crate::domain::coverage::types::Covered],
        skipped: &[ryo_values::domain::runs::types::Skipped],
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
        {
            return Err(OperatingError::NotRead);
        }
        Ok(Self {
            request: Arc::clone(request),
            files: ids.into_boxed_slice(),
            covered: covered.to_vec().into_boxed_slice(),
            skipped: skipped.to_vec().into_boxed_slice(),
        })
    }
    pub(in crate::application) fn files(&self) -> &[FileId] {
        &self.files
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
    if affected.is_empty()
        && !inputs
            .iter()
            .any(|input| matches!(input.readiness, Readiness::Partial { .. }))
    {
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

/// Pure borrowed semantic authority. It exposes neither worker ownership nor
/// dependency acquisition, and cannot outlive its admitted invocation permit.
pub(in crate::application::file_dag) struct SemanticPermit<'scope> {
    program: &'scope crate::domain::executable_program::types::ExecutableProgram,
    region: RegionId,
    instruction: usize,
    live: &'scope Arc<super::super::relations::GenerationLease>,
    strategy: KernelStrategy,
}
impl<'scope> SemanticPermit<'scope> {
    fn operation(&self) -> &ryo_values::domain::plans::types::Operation {
        &self.program.regions()[self.region.0].instructions[self.instruction].operation
    }
    fn validate_live(&self) -> Result<(), OperatingError> {
        if self.live.is_live() {
            Ok(())
        } else {
            Err(OperatingError::NotRead)
        }
    }
    pub(in crate::application::file_dag) fn physical<'call, 'meaning>(
        &'call self,
        binding: &crate::domain::executable_program::physical::PhysicalBinding<'_>,
        meaning: ryo_operators::inbound::use_cases::operators::interface::OperatorCall<'meaning>,
    ) -> Result<PermittedPhysical<'call, 'meaning, 'scope>, OperatingError> {
        self.validate_live()?;
        use crate::domain::executable_program::physical::{self, PhysicalRoute};
        let route = match self.strategy {
            KernelStrategy::Registered => PhysicalRoute::Registered,
            KernelStrategy::Positions => PhysicalRoute::Positions(
                self.program.regions()[self.region.0]
                    .native_positions
                    .as_ref()
                    .ok_or(OperatingError::WrongOperation)?,
            ),
            KernelStrategy::Ordered => PhysicalRoute::Ordered(
                self.program.regions()[self.region.0]
                    .ordered_existence
                    .as_ref()
                    .ok_or(OperatingError::WrongOperation)?,
            ),
            KernelStrategy::Column => PhysicalRoute::Column(
                self.program.regions()[self.region.0]
                    .column
                    .as_ref()
                    .ok_or(OperatingError::WrongOperation)?,
            ),
        };
        let authority = physical::authorize(self.operation(), route, binding, &meaning)?;
        Ok(PermittedPhysical {
            permit: self,
            meaning,
            authority,
        })
    }
}

/// The request and its validated authority cannot be separated or rewritten.
pub(crate) struct PermittedPhysical<'permit, 'meaning, 'scope> {
    permit: &'permit SemanticPermit<'scope>,
    meaning: ryo_operators::inbound::use_cases::operators::interface::OperatorCall<'meaning>,
    authority: crate::domain::executable_program::physical::PhysicalAuthorization,
}
impl PermittedPhysical<'_, '_, '_> {
    pub(in crate::application::file_dag) fn dispatch(
        self,
        port: &dyn crate::outbound::operators::interface::OperatorsPort,
    ) -> Result<
        ryo_operators::inbound::use_cases::operators::interface::OperatorAnswer,
        super::super::executor::KernelFault,
    > {
        if !self.permit.live.is_live() {
            return Err(super::super::executor::KernelFault::Refused(
                OperatingError::NotRead,
            ));
        }
        let _authority = self.authority;
        port.operators(self.meaning)
            .map_err(super::super::executor::KernelFault::Portable)
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
