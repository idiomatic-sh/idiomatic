//! The kinds of operation whose growth case waits for the lowering or the executor, and what the growth harness binds a growth
//! plan's parameters to.

use ryo_values::domain::plan_parts::types::{FlagState, LiteralText, WholeNumber};
use ryo_values::domain::query_bindings::types::{ParameterBinding, ParameterType};
use ryo_values::domain::query_operators::types::OperationKind;
use ryo_values::domain::query_plans::types::Plan;

/// What the growth harness binds a growth plan's parameters to, in the plan's order: the one set of the first word for a
/// list of sets of text, so a plan that ranks the words keeps one.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::lowering::fixtures::growth::growth_plan;
/// use ryo_engine::domain::lowering::fixtures::pending::growth_bindings;
/// use ryo_values::domain::query_operators::types::OperationKind;
///
/// assert_eq!(growth_bindings(&growth_plan(OperationKind::Rank)).len(), 1);
/// ```
#[must_use]
pub fn growth_bindings(plan: &Plan) -> Vec<ParameterBinding> {
    let first = || LiteralText("w0".into());
    plan.parameters
        .iter()
        .map(|parameter| match parameter.kind {
            ParameterType::Flag => ParameterBinding::Flag(FlagState::Raised),
            ParameterType::Number => ParameterBinding::Number(WholeNumber(0)),
            ParameterType::Text => ParameterBinding::Text(first()),
            ParameterType::Numbers => ParameterBinding::Numbers(Vec::new()),
            ParameterType::Texts => ParameterBinding::Texts(vec![first()]),
            ParameterType::TextSets => ParameterBinding::TextSets(vec![vec![first()]]),
            ParameterType::Tables => ParameterBinding::Tables(Vec::new()),
        })
        .collect()
}

/// The kinds whose growth case waits for the lowering or the executor: every kind in the contract commit. The integrator takes a
/// kind out in the merge that makes its case hold, which the harness names: it fails for a listed kind whose case holds, and
/// for an unlisted kind whose case does not. Of length nought at milestone 1.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::lowering::fixtures::pending::GROWTH_PENDING;
/// use ryo_values::domain::query_operators::types::OperationKind;
///
/// assert!(GROWTH_PENDING.iter().all(|kind| OperationKind::ALL.contains(kind)));
/// ```
pub const GROWTH_PENDING: [OperationKind; 13] = [
    OperationKind::OrderedMatch,
    OperationKind::Intersection,
    OperationKind::Difference,
    OperationKind::GroupDigest,
    OperationKind::Rank,
    OperationKind::Words,
    OperationKind::Closure,
    OperationKind::Cycles,
    OperationKind::Hosted,
    OperationKind::RenderSyntax,
    OperationKind::FitsRendered,
    OperationKind::OrderedDfs,
    OperationKind::Worklist,
];
