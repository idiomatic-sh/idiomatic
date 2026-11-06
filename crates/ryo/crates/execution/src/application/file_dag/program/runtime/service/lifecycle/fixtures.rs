//! Compiled program lifecycle proof with a real owner.
use super::inputs::fixtures::{TestInputs, value};
use crate::application::file_dag::program::admission::AdmittedInput;
use crate::application::file_dag::program::runtime::types::InvocationFailure;
use crate::application::file_dag::program::runtime::service::KernelInvocation;
use crate::application::file_dag::program::runtime::service::ProgramExecution;
use crate::application::file_dag::program::runtime::types::ColumnCounters;
use crate::application::file_dag::program::runtime::types::KernelInput;
use crate::application::file_dag::program::runtime::types::KernelScratch;
use crate::application::file_dag::program::runtime::types::StrategyOutcome;
use crate::domain::completion::types::completion_policy::value::CompletionPolicy;
use crate::domain::executable_program::types::Instruction;
use crate::domain::executable_program::types::RegionId;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::Emit;
use crate::domain::operating::types::OperatorContext;
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
    SemanticAndCleanupFailure { completed: RegionId },
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
        _: &crate::application::file_dag::program::runtime::service::KernelPermit<'_>,
        _: &AdmittedInput,
        _: &mut OperatorContext<'_>,
        _: &mut KernelScratch,
        _: &mut dyn Emit,
    ) -> Result<StrategyOutcome, InvocationFailure> {
        Err(InvocationFailure::Semantic(OperatingError::WrongOperation))
    }
    fn positions(
        &self,
        _: &crate::application::file_dag::program::runtime::service::KernelPermit<'_>,
        _: &mut OperatorContext<'_>,
        _: &mut KernelScratch,
    ) -> Result<Option<crate::application::file_dag::relations::PublishedInput>, InvocationFailure>
    {
        Err(InvocationFailure::Semantic(OperatingError::WrongOperation))
    }
    fn selected(
        &self,
        _: &crate::domain::executable_program::selection::SelectionAlternative,
        _: &crate::application::file_dag::program::runtime::service::KernelPermit<'_>,
        _: &mut OperatorContext<'_>,
        _: &mut KernelScratch,
    ) -> Result<Option<crate::application::file_dag::relations::PublishedInput>, InvocationFailure>
    {
        Err(InvocationFailure::Semantic(OperatingError::WrongOperation))
    }
    fn column(
        &self,
        _: &crate::domain::executable_program::column::ColumnAlternative,
        _: &crate::application::file_dag::program::runtime::service::KernelPermit<'_>,
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
        _: &crate::application::file_dag::program::runtime::service::KernelPermit<'_>,
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
            RuntimeProbeMode::SemanticAndCleanupFailure { completed } => {
                let state = self.execution.file_state();
                let before = self.execution.retry_checkpoint(state, completed).unwrap();
                for _ in 0..2 {
                    assert!(matches!(
                        self.execution.release_inputs(state, completed),
                        Err(OperatingError::NotRun)
                    ));
                    assert_eq!(
                        self.execution.retry_checkpoint(state, completed).unwrap(),
                        before
                    );
                }
                return Err(InvocationFailure::Semantic(OperatingError::WrongBinding));
            }
            RuntimeProbeMode::NestedFailure { outer, inner }
                if instruction.producer == RuleIndex(0) =>
            {
                assert_eq!(
                    self.execution.program.rules()[0].emit.as_ref().copied(),
                    Ok(outer)
                );
                let result = self.execution.invoke(
                    self.execution.file_state(),
                    inner,
                    &mut *self.nested.borrow_mut(),
                    self,
                    &policy,
                    context,
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
                    self.execution.file_state(),
                    root,
                    &mut *self.nested.borrow_mut(),
                    self,
                    &policy,
                    context,
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
        phase: crate::domain::operating::types::AfterTheFiles::Before,
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
