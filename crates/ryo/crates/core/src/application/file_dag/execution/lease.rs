//! Construct and release the finite primitive capability at one scheduled evaluation.
use super::super::{
    invocation::InvocationResources, predicates::BoundPredicate, workspace::InvocationWorkspace,
};
use crate::domain::operating::{
    errors::OperatingError,
    types::{Emit, OperatorContext},
};
use crate::domain::operator::contract::{types::*, views::BorrowedRows};
use crate::domain::table_model::{
    service::{cell, reset},
    types::Verdict,
};
use crate::domain::table_scratch::types::OperatorScratch;
use crate::domain::table_texts::{
    service::datum,
    types::{Datum, Texts},
};
use crate::outbound::operators::interface::OperatorsPort;
use ryo_relations::domain::relations::types::Value;
use ryo_language::domain::symbols::service::query_reborrow;

/// All resources are borrowed for this call. No mutable owner crosses the boundary.
pub(super) fn evaluate(
    operators: &dyn OperatorsPort,
    contract: &OperatorContract,
    input: PreparedInputs<'_>,
    resources: &mut InvocationResources<'_>,
    workspace: &mut InvocationWorkspace,
    context: &mut OperatorContext<'_>,
    emit: &mut dyn Emit,
) -> Result<(), OperatingError> {
    #[cfg(test)]
    resources.observe(workspace);
    let row = match input {
        PreparedInputs::Row { input, sides, row } => {
            if input.0 >= contract.inputs.used.0
                || ((contract.streaming.row_inputs.0 | contract.collected.row_inputs.0)
                    & (1 << input.0))
                    == 0
                || sides.len() > contract.inputs.used.0
            {
                return Err(OperatingError::WrongOperation);
            }
            Some(row)
        }
        _ => None,
    };
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
                    regex_automata::meta::Regex::new(&pattern.0).map_err(|error| {
                        OperatingError::InvalidRegex {
                            reason: error.to_string(),
                        }
                    })?,
                );
            }
            workspace.literal_regex.as_ref()
        }
    } else {
        None
    };
    if let (Some(row), crate::domain::operator::contract::resources::ResourceProgram::Join(join)) =
        (row, &contract.resources)
        && join.reach.is_none()
        && !join.has_filter
        && join.view.is_empty()
        && let Some(selected) = resources.index.as_ref().and_then(|index| index.selected())
    {
        return selected.emit_join(
            &super::PrimitiveSemanticsBridge { port: operators },
            &join.left_on,
            &join.output,
            row,
            &mut workspace.row,
            &mut workspace.other,
            context,
            emit,
        );
    }
    if let (Some(row), crate::domain::operator::contract::resources::ResourceProgram::Join(join)) =
        (row, &contract.resources)
        && join.reach.is_none()
        && !join.has_filter
        && join.view.is_empty()
        && let Some(index) = resources.index.as_ref()
    {
        // The invocation already owns its immutable index. Ordinary Join needs
        // only this finite reborrow, not an all-operator lease/predicate bundle.
        return super::super::equality_join::indexed(
            &super::PrimitiveSemanticsBridge { port: operators },
            index.view(),
            row,
            &join.left_on,
            &join.output,
            &mut workspace.row,
            &mut workspace.other,
            context,
            emit,
        );
    }
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
    let index = resources.index.as_ref().map(|index| index.view());
    let indices = std::array::from_fn::<_, 1, _>(|_| index);
    let index_slice = indices[0].as_ref().map_or(&[][..], std::slice::from_ref);
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
    let axis = resources
        .axis
        .as_ref()
        .map(|axis| axis.view())
        .transpose()?;
    let views = PrimitiveViews {
        semantic: context.semantic,
        paths: context.paths,
        arenas: context.arenas,
        source: context.source,
        bindings: context.bindings,
        indices: index_slice,
        validated_source,
        regex,
        appended: resources.appended.as_deref(),
        rank_layers: resources.rank_layers.as_deref(),
        axis: axis.as_ref(),
        native_join: resources.native_join,
        cached_reach,
        cached_axis,
        dfs: InvocationResources::dfs_view(resources.dfs.as_ref())?,
        cycles: InvocationResources::cycles_view(resources.cycles.as_ref())?,
        closure: if let Some(parts) = resources.partitioned_closure.as_ref() {
            std::slice::from_ref(parts.current())
        } else {
            &resources.closure
        },
        ordered: InvocationResources::ordered_view(resources.ordered.as_ref())?,
        groups: resources.groups.as_deref(),
    };
    let scratch = PrimitiveWorkspace {
        row: &mut workspace.row,
        other: &mut workspace.other,
        text: &mut workspace.text,
        word: &mut workspace.word,
        stretches: &mut workspace.stretches,
        captured: &mut workspace.captured,
        axis: Some(&mut workspace.axis),
        positions: &mut workspace.positions,
        dfs: Some(&mut workspace.dfs),
        cycles: Some(&mut workspace.cycle),
        closure_walk: Some(&mut workspace.closure),
        distinct: Some(DistinctWorkspace {
            rows: &mut workspace.distinct,
            seen: &mut workspace.seen,
        }),
    };
    let mut output = OutputBridge {
        emit,
        headers: super::output::Headers::from_context(context),
        scratch: &mut workspace.downstream_scratch,
        memo: &mut workspace.downstream_memo,
    };
    let semantics = super::PrimitiveSemanticsBridge { port: operators };
    let mut lease = PrimitiveLease::new(
        views,
        scratch,
        query_reborrow(&mut context.symbols),
        &mut predicate,
        &semantics,
    );
    let outcome = crate::domain::operator::service::registered(contract.kind).evaluate(
        &contract.identity,
        input,
        &mut lease,
        &mut output,
    );
    drop(lease);
    drop(output);
    drop(predicate);
    #[cfg(test)]
    resources.observe(workspace);
    outcome
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
