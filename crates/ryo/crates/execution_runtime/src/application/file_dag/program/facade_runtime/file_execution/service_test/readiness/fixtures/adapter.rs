//! The kernel invocation that omits only the ready callback and counts what it skipped.
use super::events::Events;
use crate::application::file_dag::program::admission::service::AdmittedInput;
use crate::application::file_dag::program::runtime::service::{
    KernelInvocation, KernelPermit, ReadySelection,
};
use crate::application::file_dag::program::runtime::types::{
    InvocationFailure, KernelInput, KernelScratch,
};
use ryo_execution_domain::domain::executable_program::types::{
    LogicalInstruction, instruction::value::Instruction,
};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::operator::types::Emit as RowSink;
#[cfg(test)]
use ryo_execution_domain::domain::operator::types::OperatorContext;
use std::cell::Cell as Counter;
pub(crate) struct ReadyAdapter<'a> {
    pub(crate) actual: &'a dyn KernelInvocation,
    pub(crate) omitted: Option<LogicalInstruction>,
    pub(crate) skipped: Counter<usize>,
    /// Ready callbacks that arrived with no readiness obligation of their head captured before them.
    pub(crate) unannounced: Counter<usize>,
    pub(crate) events: Events,
}
impl KernelInvocation for ReadyAdapter<'_> {
    fn ready(
        &self,
        ready: &ReadySelection<'_, '_>,
        inputs: &[AdmittedInput],
        context: &mut OperatorContext<'_>,
    ) -> Result<crate::application::file_dag::program::runtime::types::Submission, OperatingError>
    {
        let logical = ready.permit().descriptor().logical();
        {
            let events = self.events.0.lock().unwrap();
            // The runtime's obligation must exist before this callback, even when it will be skipped.
            if !events.iter().any(|event| {
                event.get("event").map(String::as_str)
                    == Some(crate::application::file_dag::program::constants::WALK_READY)
                    && event.get("logical") == Some(&logical.0.to_string())
            }) {
                self.unannounced.set(self.unannounced.get() + 1);
            }
        }
        if self.omitted == Some(logical) {
            self.skipped.set(self.skipped.get() + 1);
            // The skipped head still waits for its cohort, as a submitted one would.
            return Ok(
                crate::application::file_dag::program::runtime::types::Submission::Submitted,
            );
        }
        self.actual.ready(ready, inputs, context)
    }
    fn ordered(
        &self,
        shape: &ryo_execution_domain::domain::executable_program::types::OrderedExistence,
        permit: &KernelPermit<'_>,
        input: &AdmittedInput,
        context: &mut OperatorContext<'_>,
        scratch: &mut KernelScratch,
        output: &mut dyn RowSink,
    ) -> Result<(), InvocationFailure> {
        self.actual
            .ordered(shape, permit, input, context, scratch, output)
    }
    fn evaluate(
        &self,
        instruction: &Instruction,
        permit: &KernelPermit<'_>,
        input: KernelInput<'_>,
        context: &mut OperatorContext<'_>,
        scratch: &mut KernelScratch,
        output: &mut dyn RowSink,
    ) -> Result<(), InvocationFailure> {
        self.actual
            .evaluate(instruction, permit, input, context, scratch, output)
    }
}
