//! Compiled program lifecycle proof with a real owner.
use super::super::*;
use super::owner_inputs::{TestInputs, value};
use crate::domain::completion::types::completion_policy::value::CompletionPolicy;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{FileId, Integer, Value};
use ryo_values::domain::rows::types::Table;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use {
    crate::domain::coverage::types::RuleIndex,
    crate::domain::table_scratch::types::OperatorScratch,
    ryo_values::domain::files::types::SourceView,
};

#[derive(Clone, Copy)]
pub(super) enum RuntimeProbeMode {
    Plain,
    NestedFailure { outer: RegionId, inner: RegionId },
    Reenter { root: RegionId },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct EvaluationCount(pub usize);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Reentry {
    Unobserved,
    Refused,
}
pub(super) struct RuntimeProbe {
    execution: Rc<ProgramExecution>,
    nested: RefCell<TestInputs>,
    mode: RuntimeProbeMode,
    pub(super) calls: Cell<EvaluationCount>,
    pub(super) reentry: Cell<Reentry>,
}
impl RuntimeProbe {
    pub(super) fn new(execution: Rc<ProgramExecution>, mode: RuntimeProbeMode) -> Self {
        Self {
            nested: RefCell::new(TestInputs::new(&execution)),
            execution,
            mode,
            calls: Cell::new(EvaluationCount(0)),
            reentry: Cell::new(Reentry::Unobserved),
        }
    }
}
impl KernelInvocation for RuntimeProbe {
    fn ordered(
        &self,
        _: &crate::domain::executable_program::types::OrderedExistence,
        _: &super::super::super::admission::KernelPermit<'_>,
        _: &AdmittedInput,
        _: &mut OperatorContext<'_>,
        _: &mut KernelScratch,
        _: &mut dyn Emit,
    ) -> Result<StrategyOutcome, InvocationFailure> {
        Err(InvocationFailure::Semantic(OperatingError::WrongOperation))
    }
    fn positions(
        &self,
        _: &super::super::super::admission::KernelPermit<'_>,
        _: &mut OperatorContext<'_>,
        _: &mut KernelScratch,
    ) -> Result<Option<crate::application::file_dag::relations::PublishedInput>, InvocationFailure>
    {
        Err(InvocationFailure::Semantic(OperatingError::WrongOperation))
    }
    fn selected(
        &self,
        _: &crate::domain::executable_program::selection::SelectionAlternative,
        _: &super::super::super::admission::KernelPermit<'_>,
        _: &mut OperatorContext<'_>,
        _: &mut KernelScratch,
    ) -> Result<Option<crate::application::file_dag::relations::PublishedInput>, InvocationFailure>
    {
        Err(InvocationFailure::Semantic(OperatingError::WrongOperation))
    }
    fn column(
        &self,
        _: &crate::domain::executable_program::column::ColumnAlternative,
        _: &super::super::super::admission::KernelPermit<'_>,
        _: &AdmittedInput,
        _: &AdmittedInput,
        _: &mut OperatorContext<'_>,
        _: &mut KernelScratch,
        _: &ColumnCounters,
        _: &mut Table,
    ) -> Result<StrategyOutcome, InvocationFailure> {
        Err(InvocationFailure::Semantic(OperatingError::WrongOperation))
    }
    fn evaluate(
        &self,
        instruction: &Instruction,
        _: &super::super::super::admission::KernelPermit<'_>,
        _: KernelInput<'_>,
        context: &mut OperatorContext<'_>,
        scratch: &mut KernelScratch,
        output: &mut dyn Emit,
    ) -> Result<(), InvocationFailure> {
        self.calls.set(EvaluationCount(self.calls.get().0 + 1));
        scratch
            .ordered_row
            .push(Value::Number(Integer(100 + instruction.producer.0 as i64)));
        let policy = CompletionPolicy::file(RuleIndex(0), FileId(0), &[]);
        match self.mode {
            RuntimeProbeMode::NestedFailure { outer, inner }
                if instruction.producer == RuleIndex(0) =>
            {
                assert_eq!(
                    self.execution.program.rules()[0].emit.as_ref().copied(),
                    Ok(outer)
                );
                let result = self.execution.invoke(
                    inner,
                    &mut *self.nested.borrow_mut(),
                    self,
                    &policy,
                    context,
                    Delivery::Retain,
                );
                assert!(matches!(result, Err(OperatingError::WrongBinding)));
                assert_eq!(
                    scratch.ordered_row,
                    vec![Value::Number(Integer(100))],
                    "nested failure must not replace the outer frame"
                );
                return Err(InvocationFailure::Semantic(OperatingError::WrongBinding));
            }
            RuntimeProbeMode::NestedFailure { .. } if instruction.producer == RuleIndex(1) => {
                return Err(InvocationFailure::Semantic(OperatingError::WrongBinding));
            }
            RuntimeProbeMode::Reenter { root } => {
                let result = self.execution.invoke(
                    root,
                    &mut *self.nested.borrow_mut(),
                    self,
                    &policy,
                    context,
                    Delivery::Retain,
                );
                self.reentry
                    .set(if matches!(result, Err(OperatingError::NotRun)) {
                        Reentry::Refused
                    } else {
                        Reentry::Unobserved
                    });
            }
            _ => {}
        }
        let kinds = instruction
            .contract
            .schema
            .as_ref()
            .map_err(|error| InvocationFailure::Semantic(error.clone()))?;
        output
            .row(
                context,
                &kinds.iter().copied().map(value).collect::<Vec<_>>(),
            )
            .map_err(InvocationFailure::Delivery)
    }
}

#[derive(Clone, Copy)]
pub(super) enum SinkMode {
    Accept,
    Reject,
}
pub(super) struct Sink {
    pub(super) mode: SinkMode,
    pub(super) rows: Vec<Vec<Value>>,
}
impl Emit for Sink {
    fn row(&mut self, _: &mut OperatorContext<'_>, row: &[Value]) -> Result<(), OperatingError> {
        if matches!(self.mode, SinkMode::Reject) {
            return Err(OperatingError::WrongBinding);
        }
        self.rows.push(row.to_vec());
        Ok(())
    }
}
pub(super) fn context<'a>(
    symbols: &'a mut Symbols,
    scratch: &'a mut OperatorScratch,
    memo: &'a mut Vec<crate::domain::operating::types::Memo>,
) -> OperatorContext<'a> {
    OperatorContext {
        semantic: None,
        prepared_matchers: None,
        prepared_kinds: None,
        bindings: &[],
        symbols: symbols.into(),
        source: SourceView(&[]),
        paths: &[],
        arenas: &[],
        memo,
        scratch,
    }
}
