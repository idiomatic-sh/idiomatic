//! Admission progress: which inputs a region has supplied and admitted before its kernel may run.
use super::super::{
    AcquiredInput, AdmissionProgress, EmptinessAnswer, InputStage, KernelAdmission, KernelPermit,
    ProgramExecution, ReadySelection,
};
use crate::application::file_dag::program::admission::service::AdmittedInput;
use crate::application::file_dag::program::admission::service::Inputs;
use crate::application::file_dag::program::admission::types::Emptiness;
use crate::application::file_dag::program::admission::types::Partition;
use crate::application::file_dag::program::constants::EXECUTION_TARGET;
use crate::application::file_dag::program::constants::INPUT_ADMISSION_REFUSED;
use crate::application::file_dag::program::constants::PHASE_AVAILABILITY;
use crate::application::file_dag::program::constants::PHASE_DEPENDENCY;
use crate::application::file_dag::program::constants::PHASE_PHYSICAL;
use crate::application::file_dag::program::runtime::types::StageProgress;
use ryo_execution_domain::domain::executable_program::types::RegionId;
use ryo_execution_domain::domain::executable_program::types::region::value::Region;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::operator::types::OperatorContext;

impl<'a> KernelAdmission<'a> {
    pub(crate) fn execution(&self) -> &'a ProgramExecution {
        self.execution
    }
    pub(crate) fn region(&self) -> RegionId {
        self.region
    }
}
impl ReadySelection<'_, '_> {
    pub(crate) fn permit(&self) -> &KernelPermit<'_> {
        self.permit
    }
}
pub(crate) fn first_preflight(
    region: &Region,
) -> Option<ryo_execution_domain::domain::operator::contract::types::PreflightOrder> {
    region
        .instructions()
        .first()
        .map(|instruction| instruction.contract().semantics.preflight)
}
impl AdmissionProgress {
    pub(crate) fn is_empty(
        &mut self,
        inputs: &dyn Inputs,
        source: ryo_execution_domain::domain::executable_program::types::InputSource,
        input: &AdmittedInput,
        representation: ryo_execution_domain::domain::operator::contract::types::EmptyRepresentation,
    ) -> Result<Emptiness, OperatingError> {
        if let Some(answer) = self
            .emptiness
            .iter()
            .find(|answer| answer.source == source && answer.representation == representation)
        {
            return Ok(answer.emptiness);
        }
        let emptiness = inputs.is_empty(input, representation)?;
        self.emptiness.push(EmptinessAnswer {
            source,
            representation,
            emptiness,
        });
        Ok(emptiness)
    }

    pub(crate) fn check_schema(&mut self, region: &Region) -> Result<(), OperatingError> {
        if self.schema_done == StageProgress::Pending {
            for instruction in region.instructions().iter().rev() {
                if instruction.contract().semantics.preflight
                    == ryo_execution_domain::domain::operator::contract::types::PreflightOrder::SchemaFirst
                {
                    instruction
                        .contract()
                        .output_kinds
                        .as_ref()
                        .map_err(Clone::clone)?;
                }
            }
            self.schema_done = StageProgress::Done;
        }
        Ok(())
    }
    pub(crate) fn supply(
        &mut self,
        inputs: &mut dyn Inputs,
        source: ryo_execution_domain::domain::executable_program::types::InputSource,
        partition: &Partition,
        context: &mut OperatorContext<'_>,
    ) -> Result<(), OperatingError> {
        if self
            .acquired
            .iter()
            .any(|acquired| acquired.source == source)
        {
            return Ok(());
        }
        if let Some(dependency) = inputs.dependency(source).inspect_err(|error| {
            tracing::trace!(target: EXECUTION_TARGET, event = INPUT_ADMISSION_REFUSED, input_phase = PHASE_DEPENDENCY, action = self.action.0, ?source, ?error);
        })? {
            self.waiting = Some(dependency);
            return Err(OperatingError::NotRun);
        }
        let available = inputs.supply(source, partition, context).inspect_err(|error| {
            tracing::trace!(target: EXECUTION_TARGET, event = INPUT_ADMISSION_REFUSED, input_phase = PHASE_AVAILABILITY, action = self.action.0, ?source, ?error);
        })?;
        self.acquired.push(AcquiredInput {
            source,
            stage: InputStage::Available(available),
        });
        Ok(())
    }
    pub(crate) fn admit(
        &mut self,
        inputs: &mut dyn Inputs,
        source: ryo_execution_domain::domain::executable_program::types::InputSource,
        partition: &Partition,
        context: &mut OperatorContext<'_>,
    ) -> Result<AdmittedInput, OperatingError> {
        self.supply(inputs, source, partition, context)?;
        let acquired = self
            .acquired
            .iter_mut()
            .find(|acquired| acquired.source == source)
            .ok_or_else(|| OperatingError::NotRun)?;
        let stage = &mut acquired.stage;
        match stage {
            InputStage::Admitted(input) => Ok(input.clone()),
            InputStage::Available(available) => {
                let admitted = inputs.admit(available, partition, context).inspect_err(|error| {
                    tracing::trace!(target: EXECUTION_TARGET, event = INPUT_ADMISSION_REFUSED, input_phase = PHASE_PHYSICAL, action = self.action.0, ?source, ?error);
                })?;
                *stage = InputStage::Admitted(admitted.clone());
                Ok(admitted)
            }
        }
    }
}
pub(crate) fn empty_output_kinds(
    region: &Region,
    gate: &ryo_execution_domain::domain::operator::contract::types::EmptyResult,
) -> Result<Vec<ryo_relations::domain::relations::types::CellKind>, OperatingError> {
    let mut kinds = gate.kinds.clone()?;
    for instruction in region.instructions().iter().skip(1) {
        kinds = ryo_execution_domain::domain::operation_schema::projection::continuation_kinds(
            &instruction.operation(),
            &kinds,
        )?;
    }
    Ok(kinds)
}
