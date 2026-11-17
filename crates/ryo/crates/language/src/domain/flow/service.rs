//! The `flow` service: free functions named by verb over a file's control flow graphs: empty them for the next file, and say a
//! kind of edge or step in the words of the flow relations, with no IO and no ports.

use super::constants::{
    BACKEDGE, BREAK, BREAK_LABELED, CONDITION, CONTINUE, CONTINUE_LABELED, ERROR_EXPLICIT,
    ERROR_IMPLICIT, FINALIZE, ITERATION_IN, ITERATION_OF, JOIN, JUMP, NEW_FUNCTION, NORMAL, RETURN,
    RETURN_UNDEFINED, STATEMENT, THROW, UNREACHABLE,
};
use super::types::{FlowEdgeKind, FlowGraph, FlowStepKind};
use ryo_relations::domain::terms::types::NameText;

/// Empties the graphs, keeping their room for the next file.
pub(crate) fn clear_flow(graph: &mut FlowGraph) {
    graph.blocks.clear();
    graph.edges.clear();
    graph.steps.clear();
    graph.members.clear();
}

/// The word `flow-edge.edge-kind` carries for `kind`.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::flow::service::flow_edge_word;
/// use ryo_language::domain::flow::types::FlowEdgeKind;
///
/// assert_eq!(flow_edge_word(FlowEdgeKind::ErrorExplicit).0, "error-explicit");
/// ```
#[must_use]
pub fn flow_edge_word(kind: FlowEdgeKind) -> NameText<'static> {
    NameText(match kind {
        FlowEdgeKind::Normal => NORMAL,
        FlowEdgeKind::Jump => JUMP,
        FlowEdgeKind::Backedge => BACKEDGE,
        FlowEdgeKind::Join => JOIN,
        FlowEdgeKind::Finalize => FINALIZE,
        FlowEdgeKind::NewFunction => NEW_FUNCTION,
        FlowEdgeKind::Unreachable => UNREACHABLE,
        FlowEdgeKind::ErrorExplicit => ERROR_EXPLICIT,
        FlowEdgeKind::ErrorImplicit => ERROR_IMPLICIT,
    })
}

/// The word `flow-step.step-kind` carries for `kind`.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::flow::service::flow_step_word;
/// use ryo_language::domain::flow::types::FlowStepKind;
///
/// assert_eq!(flow_step_word(FlowStepKind::BreakLabeled).0, "break-labeled");
/// ```
#[must_use]
pub fn flow_step_word(kind: FlowStepKind) -> NameText<'static> {
    NameText(match kind {
        FlowStepKind::Statement => STATEMENT,
        FlowStepKind::Return => RETURN,
        FlowStepKind::ReturnUndefined => RETURN_UNDEFINED,
        FlowStepKind::Break => BREAK,
        FlowStepKind::BreakLabeled => BREAK_LABELED,
        FlowStepKind::Continue => CONTINUE,
        FlowStepKind::ContinueLabeled => CONTINUE_LABELED,
        FlowStepKind::Throw => THROW,
        FlowStepKind::Condition => CONDITION,
        FlowStepKind::IterationOf => ITERATION_OF,
        FlowStepKind::IterationIn => ITERATION_IN,
    })
}
