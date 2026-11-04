//! Pure physical-meaning authorization against the compiled semantic descriptor.
use super::types::OrderedExistence;
use crate::domain::operating::errors::OperatingError;
use ryo_operators::domain::contract::types::{
    JoinInput, OperatorCall, PartitionExtremum, SetInput, SetKind,
};
use ryo_values::domain::plans::types::{Operation, OperationKind};

pub(crate) enum PhysicalRoute<'a> {
    Registered,
    Ordered(&'a OrderedExistence),
    Column(&'a super::column::ColumnAlternative),
}
pub(crate) struct PhysicalBinding {
    pub native_join: bool,
    pub source_len: usize,
}
pub(crate) struct PhysicalAuthorization {
    _private: (),
}

pub(crate) fn authorize(
    operation: &Operation,
    route: PhysicalRoute<'_>,
    binding: &PhysicalBinding,
    call: &OperatorCall<'_>,
) -> Result<PhysicalAuthorization, OperatingError> {
    const PROJECT: u8 = 1;
    const APPEND: u8 = 2;
    const BOUNDS: u8 = 4;
    const BETWEEN: u8 = 8;
    const SET: u8 = 16;
    const JOIN: u8 = 32;
    const ORDERED: u8 = 64;
    let allowed = match crate::domain::features::service::kind_of(operation) {
        OperationKind::Project => {
            if matches!(route, PhysicalRoute::Column(_)) {
                PROJECT | JOIN
            } else {
                PROJECT
            }
        }
        OperationKind::Extend | OperationKind::Replace => APPEND,
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
                0
            }
        }
        OperationKind::GroupCount
        | OperationKind::GroupDigest
        | OperationKind::Capture
        | OperationKind::Rank
        | OperationKind::Words
        | OperationKind::OrderedMatch
        | OperationKind::Axis
        | OperationKind::Closure
        | OperationKind::Cycles
        | OperationKind::OrderedDfs
        | OperationKind::Rows
        | OperationKind::Hosted
        | OperationKind::SourceMatches
        | OperationKind::RenderSyntax
        | OperationKind::FitsRendered => 0,
    };
    let (family, exact) = match call {
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
                PhysicalRoute::Registered | PhysicalRoute::Column(_) => false,
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
                SetInput::Positions { .. } => false,
            },
        ),
    };
    if allowed & family == 0 || !exact {
        return Err(OperatingError::WrongOperation);
    }
    Ok(PhysicalAuthorization { _private: () })
}
