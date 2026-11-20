//! The types of the `lowering` concept: the rules it lowers and the plan shapes a refusal names.

use ryo_values::domain::plans::types::Plan;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;

/// One selected rule and the plan its query compiled to.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::lowering::fixtures::named;
/// use ryo_engine::domain::lowering::types::SelectedRule;
/// use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
///
/// let plan = named("one tree rule");
/// let rule = SelectedRule { rule: SelectedRuleOrdinal(0), plan: &plan };
/// assert_eq!(rule.rule, SelectedRuleOrdinal(0));
/// ```
#[derive(Clone, Copy, Debug)]
pub struct SelectedRule<'a> {
    /// The rule.
    pub rule: SelectedRuleOrdinal,
    /// Its compiled plan.
    pub plan: &'a Plan,
}

/// The selected rules, in rule order.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::lowering::types::SelectedRules;
///
/// assert!(SelectedRules(&[]).0.is_empty());
/// ```
#[derive(Clone, Copy, Debug)]
pub struct SelectedRules<'a>(pub &'a [SelectedRule<'a>]);

/// A step of a rule's plan, by its place in the plan's steps.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::lowering::types::PlanNode;
///
/// assert_ne!(PlanNode(0), PlanNode(1));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlanNode(pub u32);

/// The plan shapes a refusal names.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::lowering::types::{PlanNode, PlanShape};
///
/// assert_ne!(PlanShape::Step(PlanNode(0)), PlanShape::Unbound(PlanNode(0)));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlanShape {
    /// A step whose operation the lowering does not express.
    Step(PlanNode),
    /// A correlated sub-plan whose variables the lowering cannot bind.
    Unbound(PlanNode),
    /// More alternatives than the budget.
    Alternatives(PlanNode),
}
