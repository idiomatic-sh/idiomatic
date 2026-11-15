//! The `lease` application operation.

use super::super::super::executor::{service::PrimitiveExecutor, types::KernelCall};
use super::super::super::predicates::types::BoundPredicate;
use super::capability::service::ExecutableLease;
use crate::application::file_dag::derived::invocation::types::InvocationResources;
use crate::application::file_dag::executor::{types::KernelAnswer, types::KernelFault};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::operating::service::portable_fault;
use ryo_execution_domain::domain::operator::contract::{
    context::service::with_primitive_context, primitive::types::PreparedInputs, types::*,
};
use ryo_execution_domain::domain::operator::types::{Emit, OperatorContext};
use ryo_execution_store::application::file_dag::workspace::interface::WorkspacePort;
use ryo_language::domain::symbols::service::query_reborrow;
use ryo_operators::domain::table_texts::{
    service::datum,
    types::{Datum, Texts},
};
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
    workspace: &mut impl WorkspacePort,
    #[cfg(test)]
    probe: &mut ryo_execution_domain::domain::operating::fixtures::ExecutionObservation,
    context: &mut OperatorContext<'_>,
    emit: &mut dyn Emit,
) -> Result<(), OperatingError> {
    if resources.partitioned_closure().is_none() {
        return evaluate_once(
            operators,
            contract,
            permit,
            operation,
            input,
            resources,
            workspace,
            #[cfg(test)]
            probe,
            context,
            emit,
        );
    }
    loop {
        let available = resources
            .partitioned_closure()
            .ok_or_else(|| OperatingError::NotRun)?
            .advance()?;
        if available == ryo_execution_store::application::file_dag::closure_partitions::types::PartitionAdvance::Exhausted {
            return Ok(());
        }
        evaluate_once(
            operators,
            contract,
            permit,
            operation,
            input,
            resources,
            workspace,
            #[cfg(test)]
            probe,
            context,
            emit,
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
    workspace: &mut impl WorkspacePort,
    #[cfg(test)]
    probe: &mut ryo_execution_domain::domain::operating::fixtures::ExecutionObservation,
    context: &mut OperatorContext<'_>,
    emit: &mut dyn Emit,
) -> Result<(), OperatingError> {
    #[cfg(feature = "diagnostic-row-progress")]
    let routes = workspace.progress_origin().profile().clone();
    #[cfg(feature = "diagnostic-row-progress")]
    let _progress_invocation = workspace.progress_origin().resume();
    #[cfg(test)]
    resources.observe(workspace, probe);
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
                    ryo_execution_domain::domain::operator::contract::views::service::validate(
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
                ryo_operators::domain::table_texts::types::TextView(&pattern.0),
                context.matcher,
            )
        }) {
            Some(prepared?)
        } else {
            if parts.literal_regex.is_none() {
                *parts.literal_regex = Some(
                    ryo_operators::domain::patterns::types::prepared::value::compile_regex(
                        ryo_operators::domain::table_texts::types::TextView(&pattern.0),
                        context.matcher,
                    )?,
                );
            }
            parts.literal_regex.as_ref()
        }
    } else {
        None
    };
    let lent = resources.lend_row(parts.owner.derived(), row)?;
    let (cached_reach, cached_axis) = match lent.cached {
        Some(
            ryo_execution_domain::domain::operator::contract::graph_view::types::CachedGraphRow::Reach(
                view,
            ),
        ) => (Some(view), None),
        Some(
            ryo_execution_domain::domain::operator::contract::graph_view::types::CachedGraphRow::Axis(
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
    lent.resources.validate_declared()?;
    if matches!(
        input,
        PreparedInputs::Row {
            input: ryo_execution_domain::domain::operating::types::InputPlace(0),
            ..
        }
    ) {
        ryo_execution_domain::domain::operator::contract::workspace::service::begin(
            &contract.workspace,
            WorkspaceLifetime::PrimaryRow,
            &mut *parts.primitive,
        );
    }
    let views = PrimitiveViews {
        phase: context.phase,
        resources: lent.resources,
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
        downstream: &mut *parts.downstream,
    };
    let semantics = super::super::types::PrimitiveSemanticsBridge {
        port: operators,
        permit: permit.semantics(),
        failure: &semantic_failure,
        #[cfg(feature = "diagnostic-row-progress")]
        progress: &routes,
        binding:
            ryo_execution_domain::domain::interpreted_program::physical::types::PhysicalBinding {
                bindings: context.bindings,
                regex_identity: regex.map(|compiled| {
                    ryo_execution_domain::domain::interpreted_program::types::RegexIdentity(
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
        probe.registered_calls += registered_calls;
        resources.observe(workspace, probe);
    }
    semantic_failure.into_inner().map_or(outcome, Err)
}

pub(super) struct OutputBridge<'a> {
    emit: &'a mut dyn Emit,
    headers: super::super::output::service::Headers<'a>,
    downstream: &'a mut ryo_operators::domain::table_scratch::types::downstream_buffers::value::DownstreamBuffers,
}

impl RowSink for OutputBridge<'_> {
    fn row(&mut self, lease: &mut PrimitiveLease<'_>, row: &[Value]) -> Result<(), OperatingError> {
        self.headers
            .row(self.emit, &mut lease.derived, self.downstream, row)
    }
}
