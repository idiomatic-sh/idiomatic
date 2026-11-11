//! The `lease` application operation.

use super::super::super::executor::{service::PrimitiveExecutor, types::KernelCall};
use super::super::super::{
    invocation::service::InvocationResources, predicates::types::BoundPredicate,
};
use super::capability::service::ExecutableLease;
use crate::application::file_dag::executor::{types::KernelAnswer, types::KernelFault};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::operating::service::portable_fault;
use ryo_execution_domain::domain::operator::contract::{
    context::with_primitive_context, primitive::PreparedInputs, types::*,
};
use ryo_execution_domain::domain::operator::types::{Emit, OperatorContext};
use ryo_execution_domain::domain::table_scratch::types::OperatorScratch;
use ryo_execution_domain::domain::table_texts::{
    service::datum,
    types::{Datum, Texts},
};
use ryo_execution_store::application::file_dag::workspace::service::InvocationWorkspace;
use ryo_language::domain::symbols::service::query_reborrow;
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::rows::service::cell;

/// All resources are borrowed for this call. No mutable owner crosses the boundary.
pub(crate) fn evaluate(
    operators: &dyn PrimitiveExecutor,
    contract: &OperatorContract,
    permit: &crate::application::file_dag::program::runtime::service::KernelPermit<'_>,
    operation: &ryo_values::domain::plans::types::Operation,
    input: PreparedInputs<'_>,
    resources: &mut InvocationResources<'_>,
    workspace: &mut InvocationWorkspace,
    context: &mut OperatorContext<'_>,
    emit: &mut dyn Emit,
) -> Result<(), OperatingError> {
    if resources.inputs.partitioned_closure.is_none() {
        return evaluate_once(
            operators, contract, permit, operation, input, resources, workspace, context, emit,
        );
    }
    loop {
        let available = resources
            .inputs
            .partitioned_closure
            .as_mut()
            .ok_or_else(|| OperatingError::NotRun)?
            .advance()?;
        if available == ryo_execution_store::application::file_dag::closure_partitions::types::PartitionAdvance::Exhausted {
            return Ok(());
        }
        evaluate_once(
            operators, contract, permit, operation, input, resources, workspace, context, emit,
        )?;
    }
}

pub(super) fn evaluate_once(
    operators: &dyn PrimitiveExecutor,
    contract: &OperatorContract,
    permit: &crate::application::file_dag::program::runtime::service::KernelPermit<'_>,
    operation: &ryo_values::domain::plans::types::Operation,
    input: PreparedInputs<'_>,
    resources: &mut InvocationResources<'_>,
    workspace: &mut InvocationWorkspace,
    context: &mut OperatorContext<'_>,
    emit: &mut dyn Emit,
) -> Result<(), OperatingError> {
    #[cfg(feature = "diagnostic-row-progress")]
    let routes = workspace.progress_origin().profile().clone();
    #[cfg(feature = "diagnostic-row-progress")]
    let _progress_invocation = workspace.progress_origin().resume();
    #[cfg(test)]
    resources.observe(workspace);
    let parts = workspace.parts();
    let semantic_failure = std::sync::OnceLock::new();
    let row = match input {
        PreparedInputs::Row { input, sides, row } => {
            if input.0 >= contract.inputs.used.0
                || (contract.semantics.row_inputs.0 & (1 << input.0)) == 0
                || sides.len() > contract.inputs.used.0
            {
                return Err(OperatingError::WrongOperation);
            }
            Some(row)
        }
        _ => None,
    };
    #[cfg(feature = "diagnostic-row-progress")]
    if let Some(row) = row {
        routes.row_file(row, context.paths);
        ryo_execution_domain::domain::operating::types::ProgressObserver::observe(
            &routes,
            ryo_execution_domain::domain::operating::types::ProgressCounter::InputRows,
            ryo_execution_domain::domain::operating::types::ProgressAmount(1),
        );
    }
    let mut validated_source = None;
    if let Some(row) = row {
        if let Some(error) = &contract.row_error {
            return Err(error.clone());
        }
        match contract.source_gate {
            SourceGate::None => {}
            SourceGate::CheckedSpan { span, base_line } => {
                validated_source =
                    ryo_execution_domain::domain::operator::contract::views::validate(
                        row,
                        span,
                        base_line,
                        context.source,
                    )?;
                if validated_source.is_none() {
                    return Ok(());
                }
            }
            SourceGate::CheckedText { column } => {
                let texts = Texts {
                    symbols: context.symbols.as_ref(),
                    source: context.source,
                    paths: context.paths,
                    arenas: context.arenas,
                };
                if !matches!(datum(cell(row, column)?, &texts), Datum::Text(_)) {
                    return Err(OperatingError::ShapeMismatch);
                }
            }
        }
    }
    let regex = if let Some(pattern) = &contract.literal_pattern {
        if let Some(prepared) = context.prepared_matchers.and_then(|prepared| {
            prepared.regex(
                ryo_execution_domain::domain::table_texts::types::TextView(&pattern.0),
                context.matcher,
            )
        }) {
            Some(prepared?)
        } else {
            if parts.literal_regex.is_none() {
                *parts.literal_regex = Some(
                    ryo_execution_domain::domain::matching::types::prepared::value::compile_regex(
                        ryo_execution_domain::domain::table_texts::types::TextView(&pattern.0),
                        context.matcher,
                    )?,
                );
            }
            parts.literal_regex.as_ref()
        }
    } else {
        None
    };
    let cached = if let (Some(graph), Some(row)) = (resources.cached_graph.as_mut(), row) {
        Some(parts.owner.derived().graph_row(graph, row)?)
    } else {
        None
    };
    let (cached_reach, cached_axis) = match cached {
        Some(
            ryo_execution_domain::domain::operator::contract::graph_view::CachedGraphRow::Reach(
                view,
            ),
        ) => (Some(view), None),
        Some(
            ryo_execution_domain::domain::operator::contract::graph_view::CachedGraphRow::Axis(
                view,
            ),
        ) => (None, Some(view)),
        None => (None, None),
    };
    let mut predicate = BoundPredicate {
        program: &contract.predicates,
        bindings: context.bindings,
        source: context.source,
        paths: context.paths,
        arenas: context.arenas,
        prepared: context.prepared_matchers,
        state: &mut *parts.predicates,
        memo: &mut *parts.predicate_memo,
        names: &mut *parts.predicate_names,
        #[cfg(feature = "diagnostic-row-progress")]
        progress: &routes,
    };
    resources.inputs.validate_declared()?;
    if matches!(
        input,
        PreparedInputs::Row {
            input: ryo_execution_domain::domain::operating::types::InputPlace(0),
            ..
        }
    ) {
        ryo_execution_domain::domain::operator::contract::workspace::begin(
            &contract.workspace,
            WorkspaceLifetime::PrimaryRow,
            &mut *parts.primitive,
        );
    }
    let views = PrimitiveViews {
        phase: context.phase,
        resources: &resources.inputs,
        semantic: context.semantic,
        paths: context.paths,
        arenas: context.arenas,
        source: context.source,
        bindings: context.bindings,
        validated_source,
        regex,
        cached_reach,
        cached_axis,
    };
    let mut output = OutputBridge {
        emit,
        headers: super::super::output::service::Headers::from_context(context),
        scratch: &mut *parts.downstream_scratch,
        memo: &mut *parts.downstream_memo,
    };
    let semantics = super::super::types::PrimitiveSemanticsBridge {
        port: operators,
        permit: permit.semantics(),
        failure: &semantic_failure,
        #[cfg(feature = "diagnostic-row-progress")]
        progress: &routes,
        binding: ryo_execution_domain::domain::executable_program::physical::PhysicalBinding {
            bindings: context.bindings,
            regex_identity: regex.map(|compiled| {
                ryo_execution_domain::domain::executable_program::types::RegexIdentity(
                    compiled as *const _ as usize,
                )
            }),
            source_len: ryo_operators::domain::contract::types::SourceByteLength(
                context.source.0.len(),
            ),
        },
    };
    #[cfg(test)]
    let mut registered_calls = 0;
    let outcome = with_primitive_context(
        views,
        &mut *parts.primitive,
        query_reborrow(&mut context.symbols),
        &mut predicate,
        &semantics,
        parts.work,
        |context| {
            let request = ExecutableLease::new(permit, context, operation, input, &mut output);
            #[cfg(test)]
            {
                registered_calls += 1;
            }
            match operators.execute(KernelCall::Registered { request }) {
                Ok(KernelAnswer::Complete) => Ok(()),
                Ok(KernelAnswer::Physical(_) | KernelAnswer::Walk(_)) => {
                    Err(OperatingError::WrongOperation)
                }
                Err(KernelFault::Refused(error)) => Err(error),
                Err(KernelFault::Portable(error)) => Err(portable_fault(error)),
            }
        },
    );
    drop(output);
    drop(predicate);
    #[cfg(test)]
    {
        workspace.observation_mut().registered_calls += registered_calls;
        resources.observe(workspace);
    }
    semantic_failure.into_inner().map_or(outcome, Err)
}

pub(super) struct OutputBridge<'a> {
    emit: &'a mut dyn Emit,
    headers: super::super::output::service::Headers<'a>,
    scratch: &'a mut OperatorScratch,
    memo: &'a mut Vec<ryo_execution_domain::domain::operating::types::Memo>,
}

impl RowSink for OutputBridge<'_> {
    fn row(&mut self, lease: &mut PrimitiveLease<'_>, row: &[Value]) -> Result<(), OperatingError> {
        self.headers
            .row(self.emit, &mut lease.derived, self.scratch, self.memo, row)
    }
}
