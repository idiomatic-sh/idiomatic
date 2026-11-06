//! Kernel, semantic and physical permits issued from an admitted invocation.
use super::super::{
    BoundInputs, KernelAdmission, KernelPermit, KernelStrategy, ProgramExecution, SemanticPermit,
};
use crate::application::file_dag::program::admission::AdmittedInput;
use crate::application::file_dag::program::admission::ProducerRoot;
use crate::application::file_dag::program::runtime::types::InstructionIndex;
use crate::application::file_dag::relations::RelationOwner;
use crate::domain::executable_program::types::InputId;
use crate::domain::executable_program::types::InputSource;
use crate::domain::executable_program::types::RegionId;
use crate::domain::operating::errors::OperatingError;
use std::sync::Arc;

impl<'scope> KernelPermit<'scope> {
    pub(in crate::application::file_dag::program::runtime::service) fn code(
        execution: &ProgramExecution,
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
        authority: KernelAdmission<'scope>,
        inputs: &'scope [AdmittedInput],
    ) -> Result<Self, OperatingError> {
        let execution = authority.execution();
        let region = authority.region();
        let code = Self::code(execution, region)?;
        if code.instructions.is_empty() || inputs.len() != code.inputs.len() {
            return Err(OperatingError::WrongOperation);
        }
        for (source, input) in code.inputs.iter().zip(inputs) {
            let expected = match source {
                InputSource::Region(id) => ProducerRoot::Region(*id),
                other => ProducerRoot::Input(*other),
            };
            input.validate_for(
                &execution.owner,
                &execution.generation,
                execution.partition(),
                expected,
            )?;
        }
        Ok(Self {
            program: &execution.program,
            region,
            instruction: InstructionIndex(0),
            generation: &execution.generation,
            owner: execution.owner.as_ref(),
            partition: execution.partition(),
            live: &execution.generation_live,
            bindings: BoundInputs::Ordinary(inputs),
            strategy: KernelStrategy::Registered,
        })
    }
    pub(in crate::application::file_dag::program) fn ordered(
        authority: KernelAdmission<'scope>,
        input: &'scope AdmittedInput,
    ) -> Result<Self, OperatingError> {
        let execution = authority.execution();
        let region = authority.region();
        let code = Self::code(execution, region)?;
        let shape = code
            .ordered_existence
            .as_ref()
            .ok_or(OperatingError::WrongOperation)?;
        input.validate_for(
            &execution.owner,
            &execution.generation,
            execution.partition(),
            ProducerRoot::Region(shape.source),
        )?;
        Ok(Self {
            program: &execution.program,
            region,
            instruction: InstructionIndex(0),
            generation: &execution.generation,
            owner: execution.owner.as_ref(),
            partition: execution.partition(),
            live: &execution.generation_live,
            bindings: BoundInputs::Ordered(input),
            strategy: KernelStrategy::Ordered,
        })
    }
    pub(in crate::application::file_dag::program) fn column(
        authority: KernelAdmission<'scope>,
        left: &'scope AdmittedInput,
        right: &'scope AdmittedInput,
    ) -> Result<Self, OperatingError> {
        let execution = authority.execution();
        let region = authority.region();
        let code = Self::code(execution, region)?;
        let shape = code.column.as_ref().ok_or(OperatingError::WrongOperation)?;
        left.validate_for(
            &execution.owner,
            &execution.generation,
            execution.partition(),
            ProducerRoot::Region(shape.left()),
        )?;
        right.validate_for(
            &execution.owner,
            &execution.generation,
            execution.partition(),
            ProducerRoot::Input(InputSource::Relation(shape.chain().right)),
        )?;
        Ok(Self {
            program: &execution.program,
            region,
            instruction: InstructionIndex(0),
            generation: &execution.generation,
            owner: execution.owner.as_ref(),
            partition: execution.partition(),
            live: &execution.generation_live,
            bindings: BoundInputs::Column { left, right },
            strategy: KernelStrategy::Column,
        })
    }
    /// A continuation borrows the already admitted invocation; no refcount,
    /// population walk, room or ledger operation occurs here.
    pub(in crate::application::file_dag::program) fn instruction(
        &self,
        at: InstructionIndex,
    ) -> Result<Self, OperatingError> {
        self.program.regions()[self.region.0]
            .instructions
            .get(at.0)
            .ok_or(OperatingError::NotRun)?;
        Ok(Self {
            program: self.program,
            region: self.region,
            instruction: at,
            generation: self.generation,
            owner: self.owner,
            partition: self.partition,
            live: self.live,
            bindings: self.bindings,
            strategy: self.strategy,
        })
    }
    pub(in crate::application::file_dag::program) fn native_shape(
        &self,
    ) -> Result<&Arc<crate::domain::lowering::types::LoweredShape>, OperatingError> {
        self.program.regions()[self.region.0]
            .native_positions
            .as_ref()
            .ok_or(OperatingError::WrongOperation)
    }
    pub(in crate::application::file_dag::program) fn region(&self) -> RegionId {
        self.region
    }
    pub(in crate::application::file_dag::program) fn validate_program(
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
    /// Opaque diagnostic correlation for this live generation; never execution authority.
    pub(in crate::application::file_dag::program) fn diagnostic_generation(&self) -> *const () {
        Arc::as_ptr(self.generation)
    }
    pub(in crate::application::file_dag::program) fn descriptor(
        &self,
    ) -> &crate::domain::executable_program::types::Instruction {
        &self.program.regions()[self.region.0].instructions[self.instruction.0]
    }
    pub(in crate::application::file_dag::program) fn position_semantics(
        &self,
        shape: &Arc<crate::domain::lowering::types::LoweredShape>,
    ) -> Result<SemanticPermit<'_>, OperatingError> {
        self.validate_registered(self.operation())?;
        let actual = self.program.regions()[self.region.0]
            .native_positions
            .as_ref()
            .ok_or(OperatingError::WrongOperation)?;
        if !Arc::ptr_eq(actual, shape) {
            return Err(OperatingError::WrongOperation);
        }
        let mut permit = self.semantics();
        permit.strategy = KernelStrategy::Positions;
        Ok(permit)
    }
    /// Derived acquisition must borrow the owner of this actual live invocation.
    pub(in crate::application::file_dag) fn validate_derived_owner(
        &self,
        owner: &RelationOwner,
    ) -> Result<(), OperatingError> {
        self.validate_live()?;
        if !std::ptr::eq(self.owner, owner) || !owner.matches_generation(self.generation) {
            return Err(OperatingError::NotRead);
        }
        Ok(())
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
    pub(in crate::application::file_dag::program) fn bound_input(
        &self,
        at: InputId,
    ) -> Result<&AdmittedInput, OperatingError> {
        match self.bindings {
            BoundInputs::Ordinary(inputs) => inputs.get(at.0),
            BoundInputs::Ordered(input) => (at.0 == 0).then_some(input),
            BoundInputs::Column { left, right } => match at.0 {
                0 => Some(left),
                1 => Some(right),
                _ => None,
            },
        }
        .ok_or(OperatingError::MissingInput { position: at.0 + 1 })
    }
    pub(in crate::application::file_dag::program) fn validate_live(
        &self,
    ) -> Result<(), OperatingError> {
        if self.live.is_live() {
            Ok(())
        } else {
            Err(OperatingError::NotRead)
        }
    }
    pub(in crate::application::file_dag::program) fn output_kinds(
        &self,
    ) -> Result<&[ryo_relations::domain::relations::types::CellKind], OperatingError> {
        self.program.regions()[self.region.0].instructions[self.instruction.0]
            .contract
            .output_kinds
            .as_deref()
            .map_err(Clone::clone)
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
        self.program.regions()[self.region.0].instructions[self.instruction.0]
            .contract
            .kind
    }
    pub(crate) fn operation(&self) -> &ryo_values::domain::plans::types::Operation {
        &self.program.regions()[self.region.0].instructions[self.instruction.0].operation
    }
}
