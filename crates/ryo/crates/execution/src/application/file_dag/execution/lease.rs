//! Construct and release the finite primitive capability at one scheduled evaluation.
use crate::application::file_dag::executor::{KernelAnswer, KernelFault};
pub(crate) mod capability;
use super::super::executor::{KernelCall, PrimitiveExecutor};
use super::super::{
    invocation::InvocationResources, predicates::BoundPredicate, workspace::InvocationWorkspace,
};
use crate::domain::operating::{
    errors::OperatingError,
    types::{Emit, OperatorContext},
};
use crate::domain::operator::contract::{
    context::with_primitive_context, types::*, views::BorrowedRows,
};
use crate::domain::table_scratch::types::OperatorScratch;
use crate::domain::table_texts::{
    service::datum,
    types::{Datum, Texts},
};
use capability::service::ExecutableLease;
use ryo_language::domain::symbols::service::query_reborrow;
use ryo_relations::domain::relations::types::Value;
use {
    crate::domain::table_model::types::Verdict, ryo_values::domain::rows::service::cell,
    ryo_values::domain::rows::service::reset,
};

/// All resources are borrowed for this call. No mutable owner crosses the boundary.
pub(in crate::application::file_dag) fn evaluate(
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
            .ok_or(OperatingError::NotRun)?
            .advance()?;
        if !available {
            return Ok(());
        }
        evaluate_once(
            operators, contract, permit, operation, input, resources, workspace, context, emit,
        )?;
    }
}
fn evaluate_once(
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
    let _progress_invocation =
        crate::application::file_dag::route_counts::resume(workspace.progress_origin);
    #[cfg(test)]
    resources.observe(workspace);
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
        crate::application::file_dag::route_counts::progress::service::row_file(row, context.paths);
        crate::application::file_dag::route_counts::progress::service::progress(
            crate::domain::operator::contract::views::ProgressCounter::InputRows,
            1,
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
                validated_source=crate::domain::operator::collections::operators::source_matches::source::validate(row,span,base_line,context.source.0)?;
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
        if let Some(prepared) = context
            .prepared_matchers
            .and_then(|prepared| prepared.regex(&pattern.0))
        {
            Some(prepared?)
        } else {
            if workspace.literal_regex.is_none() {
                workspace.literal_regex = Some(
                    crate::domain::matching::types::prepared::value::compile_regex(&pattern.0)?,
                );
            }
            workspace.literal_regex.as_ref()
        }
    } else {
        None
    };
    let cached = if let (Some(graph), Some(row)) = (resources.cached_graph.as_mut(), row) {
        Some(workspace.owner().derived.graph_row(graph, row)?)
    } else {
        None
    };
    let (cached_reach, cached_axis) = match cached {
        Some(crate::domain::operator::contract::graph_view::CachedGraphRow::Reach(view)) => {
            (Some(view), None)
        }
        Some(crate::domain::operator::contract::graph_view::CachedGraphRow::Axis(view)) => {
            (None, Some(view))
        }
        None => (None, None),
    };
    let mut predicate = BoundPredicate {
        program: &contract.predicates,
        bindings: context.bindings,
        source: context.source,
        paths: context.paths,
        arenas: context.arenas,
        prepared: context.prepared_matchers,
        state: &mut workspace.predicates,
        memo: &mut workspace.predicate_memo,
        names: &mut workspace.predicate_names,
    };
    resources.inputs.validate_declared()?;
    if matches!(
        input,
        PreparedInputs::Row {
            input: crate::domain::operating::types::InputPlace(0),
            ..
        }
    ) {
        crate::domain::operator::contract::workspace::begin(
            &contract.workspace,
            WorkspaceLifetime::PrimaryRow,
            &mut workspace.primitive,
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
        headers: super::output::Headers::from_context(context),
        scratch: &mut workspace.downstream_scratch,
        memo: &mut workspace.downstream_memo,
    };
    let semantics = super::PrimitiveSemanticsBridge {
        port: operators,
        permit: permit.semantics(),
        failure: &semantic_failure,
        binding: crate::domain::executable_program::physical::PhysicalBinding {
            bindings: context.bindings,
            regex_identity: regex.map(|compiled| compiled as *const _ as usize),
            native_join: resources.inputs.native_join.is_some(),
            source_len: context.source.0.len(),
        },
    };
    #[cfg(test)]
    let registered_calls = &mut workspace.observation.registered_calls;
    let outcome = with_primitive_context(
        views,
        &mut workspace.primitive,
        query_reborrow(&mut context.symbols),
        &mut predicate,
        &semantics,
        &workspace.work,
        |context| {
            let request = ExecutableLease::new(permit, context, operation, input, &mut output);
            #[cfg(test)]
            {
                *registered_calls += 1;
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
    resources.observe(workspace);
    semantic_failure.into_inner().map_or(outcome, Err)
}
struct OutputBridge<'a> {
    emit: &'a mut dyn Emit,
    headers: super::output::Headers<'a>,
    scratch: &'a mut OperatorScratch,
    memo: &'a mut Vec<crate::domain::operating::types::Memo>,
}
impl RowSink for OutputBridge<'_> {
    fn row(&mut self, lease: &mut PrimitiveLease<'_>, row: &[Value]) -> Result<(), OperatingError> {
        self.headers
            .row(self.emit, &mut lease.derived, self.scratch, self.memo, row)
    }
}
