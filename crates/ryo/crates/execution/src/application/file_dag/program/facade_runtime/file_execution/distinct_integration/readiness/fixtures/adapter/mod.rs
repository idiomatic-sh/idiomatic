//! The kernel invocation that omits only the ready callback and counts what it skipped.
use super::super::super::*;
use super::events::Events;
use crate::application::file_dag::program::runtime::service::{
    KernelInvocation, KernelPermit, ReadySelection,
};
use crate::application::file_dag::program::runtime::types::{
    ColumnCounters, InvocationFailure, KernelInput, KernelScratch, StrategyOutcome,
};
use crate::domain::executable_program::types::{instruction::value::Instruction, LogicalInstruction};
use crate::domain::operating::types::Emit as RowSink;
use std::cell::Cell as Counter;
pub(crate) struct ReadyAdapter<'a> {
    pub(crate) actual: &'a dyn KernelInvocation,
    pub(crate) omitted: Option<LogicalInstruction>,
    pub(crate) skipped: Counter<usize>,
    pub(crate) events: Events,
}
impl KernelInvocation for ReadyAdapter<'_> {
    fn ready(
        &self,
        ready: &ReadySelection<'_, '_>,
        inputs: &[AdmittedInput],
        context: &mut OperatorContext<'_>,
    ) -> Result<(), OperatingError> {
        let logical = ready.permit().descriptor().logical();
        {
            let events = self.events.0.lock().unwrap();
            assert!(
                events
                    .iter()
                    .any(|event| event.get("event").map(String::as_str)
                        == Some(crate::application::file_dag::program::constants::WALK_READY)
                        && event.get("logical") == Some(&logical.0.to_string())),
                "actual runtime obligation must exist before this callback, even when it will be skipped; head={logical:?}; operation={:?}; captured={events:?}",
                ready.permit().descriptor().operation()
            );
        }
        if self.omitted == Some(logical) {
            self.skipped.set(self.skipped.get() + 1);
            return Ok(());
        }
        self.actual.ready(ready, inputs, context)
    }
    fn ordered(
        &self,
        shape: &crate::domain::executable_program::types::OrderedExistence,
        permit: &KernelPermit<'_>,
        input: &AdmittedInput,
        context: &mut OperatorContext<'_>,
        scratch: &mut KernelScratch,
        output: &mut dyn RowSink,
    ) -> Result<StrategyOutcome, InvocationFailure> {
        self.actual
            .ordered(shape, permit, input, context, scratch, output)
    }
    fn positions(
        &self,
        permit: &KernelPermit<'_>,
        context: &mut OperatorContext<'_>,
        scratch: &mut KernelScratch,
    ) -> Result<
        crate::application::file_dag::program::runtime::types::Attempt<
            crate::application::file_dag::relations::handle::service::PublishedInput,
        >,
        InvocationFailure,
    > {
        self.actual.positions(permit, context, scratch)
    }
    fn selected(
        &self,
        shape: &crate::domain::executable_program::types::SelectionAlternative,
        permit: &KernelPermit<'_>,
        context: &mut OperatorContext<'_>,
        scratch: &mut KernelScratch,
    ) -> Result<
        crate::application::file_dag::program::runtime::types::Attempt<
            crate::application::file_dag::relations::handle::service::PublishedInput,
        >,
        InvocationFailure,
    > {
        self.actual.selected(shape, permit, context, scratch)
    }
    fn column(
        &self,
        shape: &crate::domain::executable_program::types::ColumnAlternative,
        permit: &KernelPermit<'_>,
        left: &AdmittedInput,
        right: &AdmittedInput,
        context: &mut OperatorContext<'_>,
        scratch: &mut KernelScratch,
        counts: &ColumnCounters,
        rows: &mut Table,
    ) -> Result<StrategyOutcome, InvocationFailure> {
        self.actual
            .column(shape, permit, left, right, context, scratch, counts, rows)
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
