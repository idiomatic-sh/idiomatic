//! The names of the `physical_plan` concept: the one list of the operation kinds this engine never runs.

use ryo_values::domain::query_operators::types::OperationKind;

/// The kinds of operation this engine never runs: its lowering answers them `Unlowered`, and a plan is expanded into the
/// operations it does run before it reaches this engine (`plan_expansion` of the execution domain).
///
/// # Examples
///
/// ```
/// use ryo_operators::domain::physical_plan::constants::EXPANDED;
/// use ryo_values::domain::query_operators::types::OperationKind;
/// assert!(EXPANDED.contains(&OperationKind::Follow));
/// assert!(!EXPANDED.contains(&OperationKind::Closure));
/// ```
pub const EXPANDED: [OperationKind; 4] = [
    OperationKind::AsOf,
    OperationKind::GroupBound,
    OperationKind::GroupOrdinal,
    OperationKind::Follow,
];
