//! Kernel, semantic and physical permits issued from an admitted invocation.
use super::super::{BoundInputs, KernelAdmission, KernelPermit, ProgramExecution, SemanticPermit};
use crate::application::file_dag::program::admission::AdmittedInput;
use crate::application::file_dag::program::admission::ProducerRoot;
use crate::application::file_dag::program::runtime::types::InstructionIndex;
use ryo_execution_domain::domain::executable_program::types::InputId;
use ryo_execution_domain::domain::executable_program::types::InputSource;
use ryo_execution_domain::domain::executable_program::types::RegionId;
use ryo_execution_domain::domain::executable_program::types::RouteKind;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use std::sync::Arc;

impl<'scope> KernelPermit<'scope> {
    pub(crate) fn code(
        execution: &ProgramExecution,
        region: RegionId,
    ) -> Result<
        &ryo_execution_domain::domain::executable_program::types::region::value::Region,
        OperatingError,
    > {
        if execution.owner.matches_generation(&execution.generation)
            == ryo_execution_store::application::file_dag::relations::types::Sameness::Different
        {
            return Err(OperatingError::NotRead);
        }
        let code = execution
            .program
            .regions()
            .get(region.0)
            .ok_or_else(|| OperatingError::NotRun)?;
        if code.instructions().is_empty() {
            return Err(OperatingError::NotRun);
        }
        Ok(code)
    }
    pub(crate) fn admitted(
        authority: KernelAdmission<'scope>,
        inputs: &'scope [AdmittedInput],
    ) -> Result<Self, OperatingError> {
        let execution = authority.execution();
        let region = authority.region();
        let code = Self::code(execution, region)?;
        if code.instructions().is_empty() || inputs.len() != code.inputs().len() {
            return Err(OperatingError::WrongOperation);
        }
        for (source, input) in code.inputs().iter().zip(inputs) {
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
            bindings: BoundInputs::Ordinary(inputs),
            strategy: RouteKind::Generic,
        })
    }
    pub(crate) fn ordered(
        authority: KernelAdmission<'scope>,
        input: &'scope AdmittedInput,
    ) -> Result<Self, OperatingError> {
        let execution = authority.execution();
        let region = authority.region();
        let code = Self::code(execution, region)?;
        let shape = ryo_execution_domain::domain::executable_program::service::ordered_existence(
            &code.route(),
        )
        .ok_or_else(|| OperatingError::WrongOperation)?;
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
            bindings: BoundInputs::Ordered(input),
            strategy: RouteKind::OrderedExistence,
        })
    }
    /// A continuation borrows the already admitted invocation; no refcount,
    /// population walk, room or ledger operation occurs here.
    pub(crate) fn instruction(&self, at: InstructionIndex) -> Result<Self, OperatingError> {
        self.program.regions()[self.region.0]
            .instructions()
            .get(at.0)
            .ok_or_else(|| OperatingError::NotRun)?;
        Ok(Self {
            program: self.program,
            region: self.region,
            instruction: at,
            generation: self.generation,
            owner: self.owner,
            partition: self.partition,
            bindings: self.bindings,
            strategy: self.strategy,
        })
    }
    pub(crate) fn region(&self) -> RegionId {
        self.region
    }
    pub(crate) fn validate_program(
        &self,
        program: &ryo_execution_domain::domain::executable_program::types::executable_program::value::ExecutableProgram,
    ) -> Result<(), OperatingError> {
        self.validate_live()?;
        if std::ptr::eq(self.program, program) {
            Ok(())
        } else {
            Err(OperatingError::WrongOperation)
        }
    }
    /// Opaque diagnostic correlation for this live generation; never execution authority.
    pub(crate) fn diagnostic_generation(&self) -> *const () {
        Arc::as_ptr(self.generation).cast::<()>()
    }
    pub(crate) fn descriptor(
        &self,
    ) -> &ryo_execution_domain::domain::executable_program::types::instruction::value::Instruction
    {
        &self.program.regions()[self.region.0].instructions()[self.instruction.0]
    }
    pub(crate) fn semantics(&self) -> SemanticPermit<'_> {
        SemanticPermit {
            program: self.program,
            region: self.region,
            instruction: self.instruction,
            live: self.generation,
            strategy: self.strategy,
        }
    }
    pub(crate) fn bound_input(&self, at: InputId) -> Result<&AdmittedInput, OperatingError> {
        match self.bindings {
            BoundInputs::Ordinary(inputs) => inputs.get(at.0),
            BoundInputs::Ordered(input) => (at.0 == 0).then_some(input),
        }
        .ok_or_else(|| OperatingError::MissingInput { position: at.0 + 1 })
    }
    pub(crate) fn validate_live(&self) -> Result<(), OperatingError> {
        if self.generation.is_live()
            == ryo_execution_store::application::file_dag::relations::types::Liveness::Live
        {
            Ok(())
        } else {
            Err(OperatingError::NotRead)
        }
    }
    pub(crate) fn validate_registered(
        &self,
        operation: &ryo_values::domain::plans::types::Operation,
    ) -> Result<(), OperatingError> {
        self.validate_live()?;
        if !matches!(self.strategy, RouteKind::Generic) || self.operation() != operation {
            return Err(OperatingError::WrongOperation);
        }
        Ok(())
    }
    pub(crate) fn kind(&self) -> ryo_values::domain::query_operators::types::OperationKind {
        self.program.regions()[self.region.0].instructions()[self.instruction.0]
            .contract()
            .kind
    }
    pub(crate) fn operation(&self) -> &ryo_values::domain::plans::types::Operation {
        &self.program.regions()[self.region.0].instructions()[self.instruction.0].operation()
    }
}
