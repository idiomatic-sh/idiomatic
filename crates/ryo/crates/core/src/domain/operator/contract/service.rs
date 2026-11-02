//! Exhaustive semantic declarations; strategy, cache admission and lifetimes are not kinds.
use super::types::*;
use ryo_values::domain::plans::types::OperationKind;

/// Adding a kind cannot inherit an implicit policy or enter a legacy path.
pub(crate) fn modes(
    kind: OperationKind,
) -> (ModeContract, ModeContract, ResourceNeeds, Effects, u8) {
    use DemandOrder::*;
    use Effects::{DerivedText, PhysicalSourceText, Pure};
    use InputShape::*;
    use Publication::*;
    use ResourceNeeds::*;
    let row = |primary, demand| ModeContract {
        shape: Row { primary },
        demand,
        publication: Incremental,
        held_publication: Incremental,
        schema_boundary: SchemaBoundary::FirstApplicableRow,
        acquisition: AcquisitionBoundary::FirstRow,
        row_inputs: 1 << primary,
        evaluation: EvaluationMode::Streaming,
        preflight: PreflightOrder::ResourcesFirst,
    };
    let views = |demand| ModeContract {
        shape: Views,
        demand,
        publication: AfterSuccess,
        held_publication: AfterSuccess,
        schema_boundary: SchemaBoundary::BeforeDemand,
        acquisition: AcquisitionBoundary::BeforeRows,
        row_inputs: 0,
        evaluation: EvaluationMode::Streaming,
        preflight: PreflightOrder::ResourcesFirst,
    };
    let source = ModeContract {
        shape: RequestSource,
        demand: DemandOrder::None,
        publication: Incremental,
        held_publication: Incremental,
        schema_boundary: SchemaBoundary::FirstApplicableRow,
        acquisition: AcquisitionBoundary::FirstRow,
        row_inputs: 0,
        evaluation: EvaluationMode::Streaming,
        preflight: PreflightOrder::ResourcesFirst,
    };
    match kind {
        OperationKind::Select => (row(0, Unary), views(Unary), Predicates, Pure, 0),
        OperationKind::Project => (row(0, Unary), views(Unary), DistinctRows, Pure, 0),
        OperationKind::Extend => (
            row(0, Unary),
            views(Unary),
            ResourceNeeds::None,
            DerivedText,
            0,
        ),
        OperationKind::Join => (
            row(0, JoinTransport),
            views(LeftThenRight),
            JoinKeys,
            Pure,
            0,
        ),
        OperationKind::Union => (
            row(0, LeftThenRight),
            views(LeftThenRight),
            DistinctRows,
            Pure,
            0,
        ),
        OperationKind::Intersection => (
            row(0, LeftThenRightUnlessEmptyLeft),
            views(LeftThenRight),
            KeyMembership,
            Pure,
            0,
        ),
        OperationKind::Difference => (
            row(0, LeftThenRightUnlessEmptyLeft),
            views(LeftThenRight),
            KeyMembership,
            Pure,
            2,
        ),
        OperationKind::Antijoin => (
            row(0, LeftThenRightUnlessEmptyLeft),
            views(LeftThenRight),
            KeyMembership,
            Pure,
            2,
        ),
        OperationKind::GroupCount => (views(Unary), views(Unary), GroupCounts, Pure, 1),
        OperationKind::GroupDigest => (views(Unary), views(Unary), GroupDigest, Pure, 1),
        OperationKind::Capture => (row(0, Unary), views(Unary), LiteralPatterns, DerivedText, 0),
        OperationKind::Rank => (row(0, Unary), views(Unary), ResourceNeeds::None, Pure, 0),
        OperationKind::Words => (row(0, Unary), views(Unary), LiteralPatterns, DerivedText, 0),
        OperationKind::Replace => (
            ModeContract {
                publication: AfterSuccess,
                held_publication: AfterSuccess,
                ..row(0, Unary)
            },
            views(Unary),
            LiteralPatterns,
            DerivedText,
            0,
        ),
        OperationKind::OrderedMatch => (
            views(LeftThenRight),
            views(LeftThenRight),
            OrderedKeys,
            Pure,
            3,
        ),
        OperationKind::Axis => (
            row(0, AxisTransport),
            views(LeftThenRight),
            Adjacency,
            Pure,
            0,
        ),
        OperationKind::Closure => (views(Unary), views(Unary), Adjacency, Pure, 0),
        OperationKind::Cycles => (views(Unary), views(Unary), Adjacency, Pure, 0),
        OperationKind::OrderedDfs => (
            views(LeftThenRight),
            views(LeftThenRight),
            OrderedAdjacency,
            DerivedText,
            3,
        ),
        OperationKind::Rows => (
            source,
            source,
            ResourceNeeds::None,
            Effects::RequestSource,
            0,
        ),
        OperationKind::Hosted => (
            source,
            source,
            ResourceNeeds::None,
            Effects::RequestSource,
            0,
        ),
        OperationKind::SpanBetween => (row(0, Unary), views(Unary), ResourceNeeds::None, Pure, 0),
        OperationKind::SourceMatches => (
            row(0, Unary),
            views(Unary),
            LiteralPatterns,
            PhysicalSourceText,
            0,
        ),
        OperationKind::SpanBounds => (row(0, Unary), views(Unary), ResourceNeeds::None, Pure, 0),
    }
}

/// Once-per-request preparation preserves execution-timed metadata failures.
pub(crate) fn prepare_for(
    expected: OperationKind,
    operation: &ryo_values::domain::plans::types::Operation,
    inputs: &[crate::domain::operation_schema::types::InputSchema<'_>],
    bindings: &[crate::domain::table_model::types::ParameterBinding],
) -> Result<OperatorContract, crate::domain::operating::errors::OperatingError> {
    if crate::domain::features::service::kind_of(operation) != expected {
        return Err(crate::domain::operating::errors::OperatingError::WrongOperation);
    }
    let (mut streaming, mut collected, _resource_class, effects, absence_inputs) = modes(expected);
    if expected == OperationKind::Union {
        streaming.row_inputs = 3;
        collected.row_inputs = 3;
    }
    if matches!(
        expected,
        OperationKind::Union | OperationKind::Intersection | OperationKind::Difference
    ) {
        streaming.schema_boundary = SchemaBoundary::BeforeDemand;
    }
    if expected == OperationKind::Axis {
        streaming.held_publication = Publication::AfterSuccess;
    }
    streaming.evaluation = EvaluationMode::Streaming;
    collected.evaluation = EvaluationMode::Collected;
    if matches!(
        expected,
        OperationKind::Select
            | OperationKind::Project
            | OperationKind::Join
            | OperationKind::Union
            | OperationKind::Intersection
            | OperationKind::Difference
    ) {
        collected.preflight = PreflightOrder::SchemaFirst;
    }
    // Both modes call the same evaluator. Collection changes publication and
    // validation timing, never primitive code or its input meaning.
    if matches!(streaming.shape, InputShape::Row { .. }) {
        collected.shape = streaming.shape;
        collected.row_inputs = streaming.row_inputs;
    }
    // Rank bindings and simple row-only operand refusals remain row-lazy in
    // collection; Extend alone resolves authored appended values even when empty.
    if matches!(
        expected,
        OperationKind::Rank
            | OperationKind::Capture
            | OperationKind::Words
            | OperationKind::Replace
            | OperationKind::SpanBetween
            | OperationKind::SpanBounds
            | OperationKind::SourceMatches
    ) {
        collected.acquisition = AcquisitionBoundary::FirstRow;
    }
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
        } if ((*capture_text || *capture_line) && capture.is_none())
            || (*capture_line && base_line.is_none()) =>
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
    let collected_kinds = super::preflight::collected_layout(operation, inputs, &schema);
    use ryo_values::domain::plans::types::Operation;
    let aggregation = match operation {
        Operation::GroupCount { keys, distinct, .. } => Some(AggregationProgram::Count {
            keys: keys.clone(),
            distinct: *distinct,
        }),
        Operation::GroupDigest { keys, over, .. } => Some(AggregationProgram::Digest {
            keys: keys.clone(),
            over: *over,
        }),
        _ => None,
    };
    let set_scope = match operation {
        Operation::Project {
            distinct: ryo_values::domain::plan_parts::types::RepeatedRows::Dropped,
            ..
        }
        | Operation::Union { .. }
        | Operation::Intersection { .. }
        | Operation::Difference { .. } => SetScope::Invocation,
        Operation::Join { reach: Some(_), .. } => SetScope::PrimaryRow,
        _ => SetScope::None,
    };
    let set_kinds = match operation {
        Operation::Join {
            reach: Some(reach), ..
        } => crate::domain::operation_schema::service::reach_kinds(
            inputs.get(1).map_or(&[][..], |input| input.kinds),
            reach,
        )
        .unwrap_or_default(),
        Operation::Project { columns, .. } => collected_kinds.clone().unwrap_or_else(|_| {
            vec![ryo_language::domain::relations::types::CellKind::Number; columns.len()]
        }),
        _ => collected_kinds.clone().unwrap_or_default(),
    };
    let resources = super::resources::prepare(operation);
    let collected_resources = match &resources {
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
    Ok(OperatorContract {
        kind: expected,
        inputs: ryo_values::domain::plans::service::inputs_of(operation),
        external_source: matches!(
            operation,
            ryo_values::domain::plans::types::Operation::Hosted { .. }
        ),
        input_kinds: inputs.iter().map(|input| input.kinds.to_vec()).collect(),
        streaming,
        collected,
        schema,
        collected_kinds,
        absence_inputs,
        resources,
        collected_resources,
        collected_empty: super::preflight::empty_results(operation, inputs),
        set_scope,
        aggregation,
        set_kinds,
        source_gate,
        row_error,
        literal_pattern,
        predicates,
        effects,
        reuse: if effects == Effects::Pure && !super::resources::binding_dependent(operation) {
            ReuseScope::Request
        } else {
            ReuseScope::Rule
        },
        identity: operation.clone(),
    })
}
