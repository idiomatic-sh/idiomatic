// ignore: idiomatic.arch/file-length - exhaustive semantic declarations: one match per operation kind, each naming every kind it leaves to its default, which a split would scatter across files
//! Exhaustive semantic declarations; strategy, cache admission and lifetimes are not kinds.
use super::types::*;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operator::contract::presence::{
    types::InputMask, types::PresenceMask, types::RowFailure,
};
use ryo_values::domain::query_operators::types::OperationKind;

/// One semantic input and dependency contract, independent of output delivery.
pub fn semantics(kind: OperationKind) -> OperationSemantics {
    use OperationKind::*;
    let (demand, _resource, effects, absence) = match kind {
        Select => (
            DemandOrder::Unary,
            ResourceNeeds::Predicates,
            Effects::Pure,
            0,
        ),
        Project => (
            DemandOrder::Unary,
            ResourceNeeds::DistinctRows,
            Effects::Pure,
            0,
        ),
        Extend => (
            DemandOrder::Unary,
            ResourceNeeds::None,
            Effects::DerivedText,
            0,
        ),
        Join => (
            DemandOrder::LeftThenRight,
            ResourceNeeds::JoinKeys,
            Effects::Pure,
            0,
        ),
        Union => (
            DemandOrder::LeftThenRight,
            ResourceNeeds::DistinctRows,
            Effects::Pure,
            0,
        ),
        Intersection => (
            DemandOrder::LeftThenRight,
            ResourceNeeds::KeyMembership,
            Effects::Pure,
            0,
        ),
        Difference | Antijoin => (
            DemandOrder::LeftThenRight,
            ResourceNeeds::KeyMembership,
            Effects::Pure,
            2,
        ),
        GroupCount => (
            DemandOrder::Unary,
            ResourceNeeds::GroupCounts,
            Effects::Pure,
            1,
        ),
        GroupDigest => (
            DemandOrder::Unary,
            ResourceNeeds::GroupDigest,
            Effects::Pure,
            1,
        ),
        Capture | Words | Replace => (
            DemandOrder::Unary,
            ResourceNeeds::LiteralPatterns,
            Effects::DerivedText,
            0,
        ),
        Rank | SpanBetween | SpanBounds => {
            (DemandOrder::Unary, ResourceNeeds::None, Effects::Pure, 0)
        }
        OrderedMatch => (
            DemandOrder::LeftThenRight,
            ResourceNeeds::OrderedKeys,
            Effects::Pure,
            3,
        ),
        Axis => (
            DemandOrder::LeftThenRight,
            ResourceNeeds::Adjacency,
            Effects::Pure,
            0,
        ),
        Closure | Cycles => (
            DemandOrder::Unary,
            ResourceNeeds::Adjacency,
            Effects::Pure,
            0,
        ),
        OrderedDfs => (
            DemandOrder::LeftThenRight,
            ResourceNeeds::OrderedAdjacency,
            Effects::DerivedText,
            3,
        ),
        Rows | Hosted => (
            DemandOrder::None,
            ResourceNeeds::None,
            Effects::RequestSource,
            0,
        ),
        SourceMatches => (
            DemandOrder::Unary,
            ResourceNeeds::LiteralPatterns,
            Effects::PhysicalSourceText,
            0,
        ),
        RenderSyntax => (
            DemandOrder::Unary,
            ResourceNeeds::None,
            Effects::DerivedText,
            0,
        ),
        FitsRendered => (
            DemandOrder::Unary,
            ResourceNeeds::None,
            Effects::PhysicalSourceText,
            0,
        ),
    };
    let shape = match kind {
        Rows | Hosted => InputMode::RequestSource,
        GroupCount | GroupDigest | OrderedMatch | Closure | Cycles | OrderedDfs | RenderSyntax => {
            InputMode::Views
        }
        Replace | Select | Project | Extend | Join | Union | Intersection | Difference
        | Antijoin | Capture | Rank | Words | Axis | SpanBetween | SourceMatches | FitsRendered
        | SpanBounds => InputMode::Row {
            primary: ryo_values::domain::plan_parts::types::InputNumber(0),
        },
    };
    let row_inputs = RowInputMask(match shape {
        InputMode::Row { .. } => {
            if kind == Union {
                3
            } else {
                1
            }
        }
        InputMode::Views | InputMode::RequestSource => 0,
    });
    let acquisition = if matches!(
        kind,
        Join | Rank | Capture | Words | Replace | SpanBetween | SpanBounds | SourceMatches
    ) {
        AcquisitionBoundary::FirstRow
    } else {
        AcquisitionBoundary::BeforeRows
    };
    let preflight = if matches!(
        kind,
        Select | Project | Join | Union | Intersection | Difference
    ) {
        PreflightOrder::SchemaFirst
    } else {
        PreflightOrder::ResourcesFirst
    };
    OperationSemantics {
        contract: SemanticContract {
            shape,
            demand,
            acquisition,
            row_inputs,
            preflight,
        },
        effects,
        absence: AbsenceMask(absence),
        presence: presence(kind),
    }
}

/// The inputs that must have a row for the operation to answer one: each is implied by the kernel, which emits nothing
/// from an empty required input. An operation that could answer a row from empty inputs declares `Always`.
fn presence(kind: OperationKind) -> PresenceMask {
    use OperationKind::*;
    match kind {
        Select | Project | Extend | Capture | Words | Replace | Rank | SpanBetween | SpanBounds
        | SourceMatches | RenderSyntax | FitsRendered | GroupCount | GroupDigest => {
            PresenceMask::AllOf(InputMask(1))
        }
        Join | Intersection | OrderedMatch => PresenceMask::AllOf(InputMask(3)),
        Union => PresenceMask::AnyOf(InputMask(3)),
        // The right input only removes rows.
        Antijoin | Difference => PresenceMask::AllOf(InputMask(1)),
        // The walk starts from the rows of the first input.
        Axis | OrderedDfs => PresenceMask::AllOf(InputMask(1)),
        // No edge, no pair.
        Closure | Cycles => PresenceMask::AllOf(InputMask(1)),
        Rows | Hosted => PresenceMask::Always,
    }
}

/// Which inputs' rows can make a prepared operation fail on their values. Replace reads its text column through
/// `table_texts::datum`, which answers text for every `Value::Symbol` and `Value::File`; a `Value::Name` (a name that
/// left its file, held under `CellKind::Symbol`) may have none. Names reach a region only through paths that never
/// skip: cross-file steps, steps downstream of a bound observation, project relations and frontiers. So a per-file
/// Replace over a resolved Symbol or File column cannot fail on its rows; any other Replace can. Its literal pattern is
/// checked by the presence fold, which makes the input fallible when the pattern does not compile. A per-file
/// SourceMatches over a span column, with no base line and no row error, fails only on its span or its pattern: it declares
/// `UnlessLocated`, which the fold drops for a located span and a pattern and capture that compiled.
fn failure(
    operation: &ryo_values::domain::plans::types::Operation,
    inputs: &[crate::domain::operation_schema::types::InputShape<'_>],
    scope: ryo_values::domain::plan_parts::types::StepScope,
    row_error: Option<&OperatingError>,
) -> RowFailure {
    use ryo_relations::domain::relations::types::CellKind;
    use ryo_values::domain::plans::types::Operation;
    let per_file = scope == ryo_values::domain::plan_parts::types::StepScope::PerFile;
    let kinds = inputs.first().map_or(&[][..], |input| input.kinds);
    match operation {
        Operation::Replace { text, .. } => {
            match usize::try_from(text.0).map_or(None, |at| kinds.get(at)) {
                Some(CellKind::Symbol | CellKind::File) if per_file => RowFailure::Never,
                _ => RowFailure::OnRows(InputMask(1)),
            }
        }
        Operation::SourceMatches {
            span,
            base_line: None,
            ..
        } if per_file
            && row_error.is_none()
            && usize::try_from(span.0).is_ok_and(|at| kinds.get(at) == Some(&CellKind::Span)) =>
        {
            RowFailure::UnlessLocated(*span)
        }
        Operation::RenderSyntax { .. }
        | Operation::FitsRendered { .. }
        | Operation::SourceMatches { .. }
        | Operation::SpanBetween { .. }
        | Operation::SpanBounds { .. } => RowFailure::OnRows(InputMask(1)),
        Operation::OrderedMatch { .. } | Operation::OrderedDfs { .. } => {
            RowFailure::OnRows(InputMask(3))
        }
        Operation::Select { comparisons, .. } => comparison_failure(comparisons, InputMask(1)),
        Operation::Join { filter, .. } => comparison_failure(filter, InputMask(2)),
        Operation::Axis { domain_filter, .. } => comparison_failure(domain_filter, InputMask(4)),
        Operation::Project { .. }
        | Operation::Extend { .. }
        | Operation::Antijoin { .. }
        | Operation::Difference { .. }
        | Operation::Union { .. }
        | Operation::Intersection { .. }
        | Operation::GroupCount { .. }
        | Operation::GroupDigest { .. }
        | Operation::Capture { .. }
        | Operation::Rank { .. }
        | Operation::Words { .. }
        | Operation::Closure { .. }
        | Operation::Cycles { .. }
        | Operation::Rows { .. }
        | Operation::Hosted { .. } => RowFailure::Never,
    }
}

/// A regular expression read from the row is compiled when a row reaches its comparison, so it can fail on the rows of the
/// input the comparisons filter. A literal pattern is compiled at first use too: the presence fold compiles it with the
/// request's matcher, so one refused by its syntax or the size limit makes the input fallible there.
fn comparison_failure(
    comparisons: &[ryo_values::domain::plan_parts::types::PlanComparison],
    input: InputMask,
) -> RowFailure {
    use ryo_values::domain::plan_parts::types::{Operand, QueryLiteral, Test};
    let fallible = comparisons.iter().any(|comparison| {
        comparison.test == Test::Matches
            && !matches!(&comparison.right, Operand::Literal(QueryLiteral::Text(_)))
    });
    if fallible {
        RowFailure::OnRows(input)
    } else {
        RowFailure::Never
    }
}

/// Once-per-request preparation preserves execution-timed metadata failures.
pub(crate) fn prepare_for(
    expected: OperationKind,
    operation: &ryo_values::domain::plans::types::Operation,
    inputs: &[crate::domain::operation_schema::types::InputShape<'_>],
    bindings: &[ryo_values::domain::query_bindings::types::ParameterBinding],
    scope: ryo_values::domain::plan_parts::types::StepScope,
) -> Result<OperatorContract, OperatingError> {
    if ryo_values::domain::plans::service::kind_of(operation) != expected {
        return Err(OperatingError::WrongOperation);
    }
    let OperationSemantics {
        contract: semantics,
        effects,
        absence: absence_inputs,
        ..
    } = semantics(expected);
    let source_gate = match operation {
        Operation::SourceMatches {
            span, base_line, ..
        } => SourceGate::CheckedSpan {
            span: *span,
            base_line: *base_line,
        },
        Operation::Replace { text, .. } => SourceGate::CheckedText { column: *text },
        Operation::Select { .. }
        | Operation::Project { .. }
        | Operation::Extend { .. }
        | Operation::Join { .. }
        | Operation::Union { .. }
        | Operation::Intersection { .. }
        | Operation::Difference { .. }
        | Operation::Antijoin { .. }
        | Operation::GroupCount { .. }
        | Operation::GroupDigest { .. }
        | Operation::Capture { .. }
        | Operation::Rank { .. }
        | Operation::OrderedMatch { .. }
        | Operation::Words { .. }
        | Operation::Axis { .. }
        | Operation::Closure { .. }
        | Operation::OrderedDfs { .. }
        | Operation::Cycles { .. }
        | Operation::Rows { .. }
        | Operation::Hosted { .. }
        | Operation::SpanBetween { .. }
        | Operation::RenderSyntax { .. }
        | Operation::FitsRendered { .. }
        | Operation::SpanBounds { .. } => SourceGate::None,
    };
    let row_error = if let Operation::SourceMatches {
        capture,
        base_line,
        capture_text,
        capture_line,
        ..
    } = operation
        && (((*capture_text == ryo_values::domain::query_operators::types::TextOutput::Appended
            || *capture_line == ryo_values::domain::query_operators::types::LineOutput::Appended)
            && capture.is_none())
            || (*capture_line == ryo_values::domain::query_operators::types::LineOutput::Appended
                && base_line.is_none()))
    {
        Some(OperatingError::WrongOperation)
    } else {
        None
    };
    let literal_pattern = if let Operation::Replace { pattern, .. }
    | Operation::SourceMatches { pattern, .. } = operation
    {
        Some(pattern.clone())
    } else {
        None
    };
    let (comparisons, predicate_input) = match operation {
        Operation::Select { comparisons, .. } => (comparisons.as_slice(), 0),
        Operation::Join { filter, .. } => (filter.as_slice(), 1),
        Operation::Axis { domain_filter, .. } => (domain_filter.as_slice(), 2),
        Operation::Project { .. }
        | Operation::Extend { .. }
        | Operation::Union { .. }
        | Operation::Intersection { .. }
        | Operation::Difference { .. }
        | Operation::Antijoin { .. }
        | Operation::GroupCount { .. }
        | Operation::GroupDigest { .. }
        | Operation::Capture { .. }
        | Operation::Rank { .. }
        | Operation::Replace { .. }
        | Operation::OrderedMatch { .. }
        | Operation::Words { .. }
        | Operation::Closure { .. }
        | Operation::OrderedDfs { .. }
        | Operation::Cycles { .. }
        | Operation::Rows { .. }
        | Operation::Hosted { .. }
        | Operation::SpanBetween { .. }
        | Operation::SourceMatches { .. }
        | Operation::RenderSyntax { .. }
        | Operation::FitsRendered { .. }
        | Operation::SpanBounds { .. } => (&[][..], 0),
    };
    let predicates = PreparedPredicates {
        comparisons: comparisons.to_vec(),
        kinds: inputs
            .get(predicate_input)
            .map_or(&[][..], |schema| schema.kinds)
            .to_vec(),
    };
    let schema = crate::domain::operation_schema::service::schema_of(operation, inputs, bindings);
    let output_kinds = super::preflight::service::output_layout(operation, inputs, &schema);
    use ryo_values::domain::plans::types::Operation;
    let resources = super::resources::service::prepare(operation);
    let resources = if let super::resources::types::ResourceProgram::Index {
        input,
        columns: super::resources::types::IndexColumns::Columns(columns),
    } = &resources
        && matches!(operation, Operation::Antijoin { .. })
    {
        super::resources::types::ResourceProgram::Index {
            input: *input,
            columns: super::resources::types::IndexColumns::ColumnsUnlessEmpty(columns.clone()),
        }
    } else {
        resources.clone()
    };
    let workspace = super::workspace::service::prepare(operation, inputs, &output_kinds);
    Ok(OperatorContract {
        derived: Default::default(),
        kind: expected,
        inputs: ryo_values::domain::plans::service::inputs_of(operation),
        input_kinds: inputs.iter().map(|input| input.kinds.to_vec()).collect(),
        semantics,
        schema,
        output_kinds,
        absence_inputs,
        workspace,
        resources,
        empty_results: super::preflight::service::empty_results(operation, inputs),
        source_gate,
        failure: failure(operation, inputs, scope, row_error.as_ref()),
        row_error,
        literal_pattern,
        predicates,
        effects,
        reuse: reuse_scope(
            effects,
            super::resources::service::binding_dependent(operation),
        ),
        identity: operation.clone(),
    })
}

/// Where equal operations share one result. Derived and physical text belong to the file, not to a rule: only a binding or
/// the request's own source makes equal operations of different rules answer differently. Every effect chooses here, so a
/// new one cannot be request-shared by default.
fn reuse_scope(effects: Effects, bindings: super::resources::types::BindingUse) -> ReuseScope {
    let shareable = match effects {
        Effects::Pure | Effects::DerivedText | Effects::PhysicalSourceText => true,
        Effects::RequestSource => false,
    };
    if shareable && bindings == super::resources::types::BindingUse::Independent {
        ReuseScope::Request
    } else {
        ReuseScope::Rule
    }
}
