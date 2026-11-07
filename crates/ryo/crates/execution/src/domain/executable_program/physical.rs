//! Pure physical-meaning authorization against the compiled semantic descriptor.
use super::types::OrderedExistence;
use crate::domain::operating::errors::OperatingError;
use ryo_operators::domain::equality_join::types::JoinInput;
use ryo_operators::domain::contract::types::OperatorCall;
use ryo_operators::domain::ordered_existence::types::PartitionExtremum;
use ryo_operators::domain::set_rows::types::SetInput;
use ryo_operators::domain::set_rows::types::SetKind;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::query_operators::types::OperationKind;

pub(crate) enum PhysicalRoute<'a> {
    Registered,
    Positions(&'a crate::domain::lowering::types::LoweredShape),
    Ordered(&'a OrderedExistence),
    Column(&'a super::types::ColumnAlternative),
}
pub(crate) struct PhysicalBinding<'a> {
    pub bindings: &'a [ryo_values::domain::query_bindings::types::ParameterBinding],
    pub regex_identity: Option<usize>,
    pub native_join: bool,
    pub source_len: usize,
}
pub(crate) struct PhysicalAuthorization {
    _private: (),
}

pub(crate) fn authorize(
    operation: &Operation,
    route: PhysicalRoute<'_>,
    binding: &PhysicalBinding<'_>,
    call: &OperatorCall<'_>,
) -> Result<PhysicalAuthorization, OperatingError> {
    const PROJECT: u32 = 1;
    const APPEND: u32 = 2;
    const BOUNDS: u32 = 4;
    const BETWEEN: u32 = 8;
    const SET: u32 = 16;
    const JOIN: u32 = 32;
    const ORDERED: u32 = 64;
    const SELECT: u32 = 128;
    const WORDS: u32 = 256;
    const CAPTURE: u32 = 512;
    const RANK: u32 = 1024;
    const REPLACE: u32 = 2048;
    const AGGREGATE: u32 = 4096;
    const ORDERED_MATCH: u32 = 8192;
    const SOURCE_MATCHES: u32 = 16384;
    const REDUCTION: u32 = 32768;
    let allowed = match ryo_values::domain::plans::service::kind_of(operation) {
        OperationKind::Project => {
            if matches!(route, PhysicalRoute::Column(_)) {
                PROJECT | JOIN
            } else {
                PROJECT
            }
        }
        OperationKind::Extend => APPEND,
        OperationKind::Replace => APPEND | REPLACE,
        OperationKind::SpanBounds => BOUNDS | APPEND,
        OperationKind::SpanBetween => BETWEEN | APPEND,
        OperationKind::Union
        | OperationKind::Intersection
        | OperationKind::Difference
        | OperationKind::Antijoin => SET,
        OperationKind::Join => JOIN,
        OperationKind::Select => {
            if matches!(route, PhysicalRoute::Ordered(_)) {
                ORDERED
            } else {
                SELECT
            }
        }
        OperationKind::GroupCount | OperationKind::GroupDigest => AGGREGATE | REDUCTION,
        OperationKind::Capture => CAPTURE,
        OperationKind::Rank => RANK,
        OperationKind::Words => WORDS,
        OperationKind::OrderedMatch => ORDERED_MATCH,
        OperationKind::SourceMatches => SOURCE_MATCHES,
        OperationKind::Axis
        | OperationKind::Closure
        | OperationKind::Cycles
        | OperationKind::OrderedDfs
        | OperationKind::Rows
        | OperationKind::Hosted
        | OperationKind::RenderSyntax
        | OperationKind::FitsRendered => 0,
    };
    let (family, exact) = match call {
        OperatorCall::Select { kept, .. } => (
            SELECT,
            matches!(operation, Operation::Select { kept: expected, .. } if *kept == expected.as_slice()),
        ),
        OperatorCall::Words { .. } => (WORDS, matches!(operation, Operation::Words { .. })),
        OperatorCall::Capture { pattern, .. } => (
            CAPTURE,
            matches!(operation, Operation::Capture { pattern: expected, .. } if *pattern == expected.0.as_ref()),
        ),
        OperatorCall::Rank { layers, .. } => (
            RANK,
            match operation {
                Operation::Rank { sets, .. } => {
                    matches!(binding.bindings.get(sets.0 as usize), Some(ryo_values::domain::query_bindings::types::ParameterBinding::TextSets(expected)) if *layers == expected.as_slice())
                }
                _ => false,
            },
        ),
        OperatorCall::Replace {
            regex, replacement, ..
        } => (
            REPLACE,
            matches!(operation, Operation::Replace { replacement: expected, .. } if *replacement == expected.0.as_ref())
                && binding.regex_identity == Some(*regex as *const _ as usize),
        ),
        OperatorCall::Aggregate { keys, digest, .. } => (
            AGGREGATE,
            match operation {
                Operation::GroupCount { keys: expected, .. } => {
                    digest.is_none() && *keys == expected.as_slice()
                }
                Operation::GroupDigest { keys: expected, .. } => {
                    digest.is_some() && *keys == expected.as_slice()
                }
                _ => false,
            },
        ),
        OperatorCall::OrderedMatch { output, .. } => (
            ORDERED_MATCH,
            matches!(operation, Operation::OrderedMatch { output: expected, .. } if *output == expected.as_slice()),
        ),
        OperatorCall::SourceMatches { input, .. } => (
            SOURCE_MATCHES,
            matches!(operation, Operation::SourceMatches { capture, capture_text, capture_line, .. } if input.capture == capture.as_ref().map(|name| name.0.as_ref()) && (input.capture_text == (*capture_text == ryo_values::domain::query_operators::types::TextOutput::Appended)) && (input.capture_line == (*capture_line == ryo_values::domain::query_operators::types::LineOutput::Appended)))
                && binding.regex_identity == Some(input.regex as *const _ as usize),
        ),
        OperatorCall::ReduceCount { keys, distinct, .. } => (
            REDUCTION,
            matches!((operation, keys), (Operation::GroupCount { keys: expected, distinct: expected_distinct, .. }, ryo_operators::domain::reduction::types::GroupColumns::Columns(actual)) if *actual == expected.as_slice() && distinct == expected_distinct),
        ),
        OperatorCall::ReduceDigest { keys, over, .. } => (
            REDUCTION,
            matches!(operation, Operation::GroupDigest { keys: expected, over: expected_over, .. } if *keys == expected.as_slice() && over == expected_over),
        ),
        OperatorCall::CheckAggregation { program, .. }
        | OperatorCall::StreamAggregation { program, .. } => {
            (REDUCTION, aggregation_matches(operation, program))
        }
        OperatorCall::FinishAggregation { digest, .. } => (
            REDUCTION,
            match operation {
                Operation::GroupCount { .. } => !*digest,
                Operation::GroupDigest { .. } => *digest,
                _ => false,
            },
        ),
        OperatorCall::HashValue { .. } => return Err(OperatingError::WrongOperation),
        OperatorCall::OrderedExistence {
            partition,
            order,
            extremum,
            ..
        } => (
            ORDERED,
            match route {
                PhysicalRoute::Ordered(shape) => {
                    *partition == shape.partition.as_ref()
                        && *order == shape.order
                        && *extremum
                            == match shape.extremum {
                                crate::domain::ordered_existence::types::Extremum::Maximum => {
                                    PartitionExtremum::Maximum
                                }
                                crate::domain::ordered_existence::types::Extremum::Minimum => {
                                    PartitionExtremum::Minimum
                                }
                            }
                }
                PhysicalRoute::Registered
                | PhysicalRoute::Column(_)
                | PhysicalRoute::Positions(_) => false,
            },
        ),
        OperatorCall::Project { columns, .. } => (
            PROJECT,
            matches!(operation, Operation::Project { columns: expected, .. } if columns == &expected.as_slice()),
        ),
        OperatorCall::Append { appended, .. } => (
            APPEND,
            match operation {
                Operation::Extend { values, .. } => appended.len() == values.len(),
                Operation::Replace { .. } | Operation::SpanBetween { .. } => appended.len() == 1,
                Operation::SpanBounds { .. } => appended.len() == 2,
                _ => false,
            },
        ),
        OperatorCall::SpanBounds { column, .. } => (
            BOUNDS,
            matches!(operation, Operation::SpanBounds { span, .. } if span == column),
        ),
        OperatorCall::SpanBetween {
            start,
            end,
            source_len,
            ..
        } => (
            BETWEEN,
            matches!(operation, Operation::SpanBetween { start: expected_start, end: expected_end, .. } if start == expected_start && end == expected_end && source_len.0 == binding.source_len),
        ),
        OperatorCall::EqualityJoin { input } => (
            JOIN,
            match (operation, input) {
                (
                    Operation::Project { .. },
                    JoinInput::Values {
                        left_on,
                        output,
                        view,
                        ..
                    },
                ) if matches!(route, PhysicalRoute::Column(_)) => {
                    let PhysicalRoute::Column(shape) = route else {
                        return Err(OperatingError::WrongOperation);
                    };
                    *left_on == shape.chain().left_on.as_slice()
                        && *output == shape.chain().projected.as_slice()
                        && view.is_empty()
                }

                (
                    Operation::Join {
                        left_on,
                        output,
                        view,
                        ..
                    },
                    JoinInput::Values {
                        left_on: actual_left,
                        output: actual_output,
                        view: actual_view,
                        ..
                    },
                ) => {
                    left_on.as_slice() == *actual_left
                        && output.as_slice() == *actual_output
                        && if binding.native_join {
                            view.as_slice() == *actual_view
                        } else {
                            actual_view.is_empty()
                        }
                }
                (Operation::Join { .. }, JoinInput::Positions { .. }) => {
                    matches!(route, PhysicalRoute::Positions(shape) if shape.width.0 > 0)
                }
                // Deferred positions require their own compiled native descriptor;
                // an ordinary row permit cannot authorize that representation.
                (_, JoinInput::Positions { .. }) | (_, JoinInput::Values { .. }) => false,
            },
        ),
        OperatorCall::SetRow { input } => (
            SET,
            match input {
                SetInput::Values { kind, left_on, .. } => match (operation, kind) {
                    (Operation::Union { .. }, SetKind::Union)
                    | (Operation::Intersection { .. }, SetKind::Intersection)
                    | (Operation::Difference { .. }, SetKind::Difference) => left_on.is_empty(),
                    (
                        Operation::Antijoin {
                            left_on: expected, ..
                        },
                        SetKind::Antijoin,
                    ) => expected.as_slice() == *left_on,
                    _ => false,
                },
                SetInput::Positions { kind, .. } => {
                    matches!(route, PhysicalRoute::Positions(shape) if shape.width.0 > 0)
                        && matches!(
                            (operation, kind),
                            (Operation::Union { .. }, SetKind::Union)
                                | (Operation::Antijoin { .. }, SetKind::Antijoin)
                        )
                }
            },
        ),
    };
    if allowed & family == 0 || !exact {
        return Err(OperatingError::WrongOperation);
    }
    Ok(PhysicalAuthorization { _private: () })
}

fn aggregation_matches(
    operation: &Operation,
    program: &ryo_operators::domain::reduction::types::AggregationPlan<'_>,
) -> bool {
    use ryo_operators::domain::reduction::types::AggregationPlan;
    match (operation, program) {
        (
            Operation::GroupCount { keys, distinct, .. },
            AggregationPlan::Count {
                keys: actual,
                distinct: actual_distinct,
            },
        ) => keys.as_slice() == *actual && distinct == actual_distinct,
        (
            Operation::GroupDigest { keys, over, .. },
            AggregationPlan::Digest {
                keys: actual,
                over: actual_over,
            },
        ) => keys.as_slice() == *actual && over == actual_over,
        _ => false,
    }
}
