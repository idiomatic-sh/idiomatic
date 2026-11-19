//! File-specific public ports of a shared physical graph.

use crate::domain::executable_program::types::RuleNodes;
use ryo_operators::domain::physical_plan::types::PlanNodeId;

/// A rule whose authored steps or engine uses select different slots after the file proof.
#[derive(Debug)]
pub struct RuleRoute {
    /// The alternate node for each original authored step.
    pub nodes: RuleNodes,
    /// The external uses to release at the end of this rule on the proven route.
    pub uses: Vec<PlanNodeId>,
}

/// Conditional ports. Node storage and planned operations belong to one executable program.
#[derive(Debug, Default)]
pub struct FileProgramRoutes {
    /// Alternate rule ports, only for rules with a changed step or external use.
    pub rules: Vec<Option<RuleRoute>>,
    /// Unused external holds when the file takes the ordinary route.
    pub unused_when_ordinary: Vec<PlanNodeId>,
    /// Unused external holds when the file takes the proven route.
    pub unused_when_proven: Vec<PlanNodeId>,
}

/// A single logical graph and the per-file public ports into it.
pub struct RoutedBuild<'p> {
    /// The single graph for the ordinary physical planner to lower.
    pub program: crate::domain::executable_program::types::logical_build::value::LogicalBuild<'p>,
    /// The conditional routes into that same graph.
    pub routes: FileProgramRoutes,
}
