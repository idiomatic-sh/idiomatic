//! The `physical` service: free functions named by verb over the domain's types, with no IO and no ports.

use super::super::types::RegexIdentity;
use super::types::{PhysicalAuthorization, PhysicalBinding, PhysicalRoute};
use crate::domain::operating::errors::OperatingError;
use ryo_operators::domain::equality_join::types::PartitionExtremum;
use ryo_operators::domain::evaluation::types::Lent;
use ryo_operators::inbound::use_cases::operators::types::OperatorCall;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::query_operators::types::OperationKind;

/// Refuses resources an evaluation lends that are not the ones its instruction was prepared with: the
/// acquired matcher, the bound text-set layers and the source length. An absent resource is the operators'
/// refusal.
pub(super) fn bound_views(
    operation: &Operation,
    binding: &PhysicalBinding<'_>,
    views: &ryo_operators::domain::evaluation::types::EvaluationViews<'_>,
) -> Result<(), OperatingError> {
    let bound = match operation {
        Operation::Replace { .. } | Operation::SourceMatches { .. } => match views.regex {
            Lent::Unbound => true,
            Lent::Bound(regex) => {
                binding.regex_identity == Some(RegexIdentity(regex as *const _ as usize))
            }
        },
        Operation::Rank { sets, .. } => match views.layers {
            Lent::Unbound => true,
            Lent::Bound(layers) => {
                matches!(binding.bindings.get(sets.0 as usize), Some(ryo_values::domain::query_bindings::types::ParameterBinding::TextSets(expected)) if layers == expected.as_slice())
            }
        },
        Operation::SpanBetween { .. } => views.source.0.len() == binding.source_len.0,
        _ => true,
    };
    if bound {
        Ok(())
    } else {
        Err(OperatingError::WrongOperation)
    }
}

/// Authorizes one call of the operators as the compiled meaning of its instruction: the one evaluation of the
/// instruction's own operation on the registered route, the ordered-existence driver of a resolved region, or a
/// reduction that prepares the grouping of a grouped instruction.
///
/// # Errors
///
/// `WrongOperation` when the call is not the compiled meaning of the instruction, and the error the bound views
/// refuse with when the call lends resources the instruction was not prepared with.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::interpreted_program::physical::service::authorize;
/// use ryo_execution_domain::domain::interpreted_program::physical::types::{PhysicalBinding, PhysicalRoute};
/// use ryo_operators::inbound::use_cases::operators::types::OperatorCall;
/// use ryo_values::domain::plans::types::Operation;
/// fn authorized(
///     operation: &Operation,
///     route: PhysicalRoute<'_>,
///     binding: &PhysicalBinding<'_>,
///     call: &OperatorCall<'_>,
/// ) -> bool {
///     authorize(operation, route, binding, call).is_ok()
/// }
/// ```
pub fn authorize(
    operation: &Operation,
    route: PhysicalRoute<'_>,
    binding: &PhysicalBinding<'_>,
    call: &OperatorCall<'_>,
) -> Result<PhysicalAuthorization, OperatingError> {
    let exact = match call {
        OperatorCall::Evaluate {
            operation: called,
            views,
            ..
        } => {
            bound_views(operation, binding, views)?;
            matches!(route, PhysicalRoute::Registered) && std::ptr::eq(*called, operation)
        }
        OperatorCall::OrderedExistence {
            partition,
            order,
            extremum,
            ..
        } => match route {
            PhysicalRoute::Ordered(shape) => {
                ryo_values::domain::plans::service::kind_of(operation) == OperationKind::Select
                    && *partition == shape.partition.as_ref()
                    && *order == shape.order
                    && *extremum
                        == match shape.extremum {
                            crate::domain::self_join_recogniser::types::Extremum::Maximum => {
                                PartitionExtremum::Maximum
                            }
                            crate::domain::self_join_recogniser::types::Extremum::Minimum => {
                                PartitionExtremum::Minimum
                            }
                        }
            }
            PhysicalRoute::Registered => false,
        },
        OperatorCall::ReduceCount { keys, distinct, .. } => {
            matches!((operation, keys), (Operation::GroupCount { keys: expected, distinct: expected_distinct, .. }, ryo_operators::domain::reduction::types::GroupColumns::Columns(actual)) if *actual == expected.as_slice() && distinct == expected_distinct)
        }
        OperatorCall::ReduceDigest { keys, over, .. } => {
            matches!(operation, Operation::GroupDigest { keys: expected, over: expected_over, .. } if *keys == expected.as_slice() && over == expected_over)
        }
        _ => false,
    };
    if !exact {
        return Err(OperatingError::WrongOperation);
    }
    Ok(PhysicalAuthorization {
        _sealed: std::marker::PhantomData,
    })
}
