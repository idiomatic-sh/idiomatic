//! Exhaustive semantic declarations; strategy, cache admission and lifetimes are not kinds.
use super::types::*;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operator::contract::presence::{InputMask, PresenceMask, RowFailure};
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
        _ => InputMode::Row {
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
        _ => 0,
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
/// checked by the presence fold, which makes the input fallible when the pattern does not compile.
fn failure(
    operation: &ryo_values::domain::plans::types::Operation,
    inputs: &[crate::domain::operation_schema::types::InputShape<'_>],
    scope: ryo_values::domain::plan_parts::types::StepScope,
) -> RowFailure {
    use ryo_relations::domain::relations::types::CellKind;
    use ryo_values::domain::plans::types::Operation;
    match operation {
        Operation::Replace { text, .. } => {
            let kind = inputs
                .first()
                .and_then(|input| input.kinds.get(text.0 as usize));
            if scope == ryo_values::domain::plan_parts::types::StepScope::PerFile
                && matches!(kind, Some(CellKind::Symbol | CellKind::File))
            {
                RowFailure::Never
            } else {
                RowFailure::OnRows(InputMask(1))
            }
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
        _ => RowFailure::Never,
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
) -> Result<OperatorContract, crate::domain::operating::errors::OperatingError> {
    if ryo_values::domain::plans::service::kind_of(operation) != expected {
        return Err(crate::domain::operating::errors::OperatingError::WrongOperation);
    }
    let OperationSemantics {
        contract: semantics,
        effects,
        absence: absence_inputs,
        ..
    } = semantics(expected);
    let source_gate = match operation {
        ryo_values::domain::plans::types::Operation::SourceMatches {
            span, base_line, ..
        } => SourceGate::CheckedSpan {
            span: *span,
            base_line: *base_line,
        },
        ryo_values::domain::plans::types::Operation::Replace { text, .. } => {
            SourceGate::CheckedText { column: *text }
        }
        _ => SourceGate::None,
    };
    let row_error = match operation {
        ryo_values::domain::plans::types::Operation::SourceMatches {
            capture,
            base_line,
            capture_text,
            capture_line,
            ..
        } if ((*capture_text
            == ryo_values::domain::query_operators::types::TextOutput::Appended
            || *capture_line
                == ryo_values::domain::query_operators::types::LineOutput::Appended)
            && capture.is_none())
            || (*capture_line
                == ryo_values::domain::query_operators::types::LineOutput::Appended
                && base_line.is_none()) =>
        {
            Some(crate::domain::operating::errors::OperatingError::WrongOperation)
        }
        _ => None,
    };
    let literal_pattern = match operation {
        ryo_values::domain::plans::types::Operation::Replace { pattern, .. }
        | ryo_values::domain::plans::types::Operation::SourceMatches { pattern, .. } => {
            Some(pattern.clone())
        }
        _ => None,
    };
    let (comparisons, predicate_input) = match operation {
        ryo_values::domain::plans::types::Operation::Select { comparisons, .. } => {
            (comparisons.as_slice(), 0)
        }
        ryo_values::domain::plans::types::Operation::Join { filter, .. } => (filter.as_slice(), 1),
        ryo_values::domain::plans::types::Operation::Axis { domain_filter, .. } => {
            (domain_filter.as_slice(), 2)
        }
        _ => (&[][..], 0),
    };
    let predicates = PreparedPredicates {
        comparisons: comparisons.to_vec(),
        kinds: inputs
            .get(predicate_input)
            .map_or(&[][..], |schema| schema.kinds)
            .to_vec(),
    };
    let schema = crate::domain::operation_schema::service::schema_of(operation, inputs, bindings);
    let output_kinds = super::preflight::output_layout(operation, inputs, &schema);
    use ryo_values::domain::plans::types::Operation;
    let resources = super::resources::prepare(operation);
    let resources = match &resources {
        super::resources::ResourceProgram::Index {
            input,
            columns: super::resources::IndexColumns::Columns(columns),
        } if matches!(operation, Operation::Antijoin { .. }) => {
            super::resources::ResourceProgram::Index {
                input: *input,
                columns: super::resources::IndexColumns::ColumnsUnlessEmpty(columns.clone()),
            }
        }
        _ => resources.clone(),
    };
    let workspace = super::workspace::prepare(operation, inputs, &output_kinds);
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
        empty_results: super::preflight::empty_results(operation, inputs),
        source_gate,
        failure: failure(operation, inputs, scope),
        row_error,
        literal_pattern,
        predicates,
        effects,
        reuse: reuse_scope(effects, super::resources::binding_dependent(operation)),
        identity: operation.clone(),
    })
}

/// Where equal operations share one result. Derived and physical text belong to the file, not to a rule: only a binding or
/// the request's own source makes equal operations of different rules answer differently. Every effect chooses here, so a
/// new one cannot be request-shared by default.
fn reuse_scope(effects: Effects, bindings: super::resources::BindingUse) -> ReuseScope {
    let shareable = match effects {
        Effects::Pure | Effects::DerivedText | Effects::PhysicalSourceText => true,
        Effects::RequestSource => false,
    };
    if shareable && bindings == super::resources::BindingUse::Independent {
        ReuseScope::Request
    } else {
        ReuseScope::Rule
    }
}
