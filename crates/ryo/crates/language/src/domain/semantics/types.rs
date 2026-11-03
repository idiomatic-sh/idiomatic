//! The syntax roles and outcomes shared by language providers and their consumers.

use ryo_relations::domain::relations::types::{FileId, NodeId};
use ryo_relations::domain::terms::types::UnknownReason;
use std::sync::Arc;

/// The role in which a rendered syntax node will be embedded.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RenderContext {
    /// A target on the left of an assignment.
    AssignmentTarget,
    /// The receiver before an attribute access.
    AttributeReceiver,
    /// An argument in a call's positional argument list.
    PositionalArgument,
}

/// One syntax node and the embedding role for which its text is requested.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RenderRequest {
    /// The node in the current file's element table.
    pub node: NodeId,
    /// The syntax role of the resulting text.
    pub context: RenderContext,
}

/// What the language can prove about rendering one requested node.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RenderOutcome {
    /// Exact text in the requested context; the empty string remains a known answer.
    Known(Arc<str>),
    /// This node cannot occur in the requested syntax role.
    NotApplicable,
    /// Valid syntax for which the provider cannot prove exact rendered text.
    Unsupported(UnknownReason),
}

/// A rendering request that does not belong to the current file's tree.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum RenderError {
    /// The node was not produced by the current file generation.
    #[error("semantic rendering received stale node {node:?}")]
    StaleNode {
        /// The stale node.
        node: NodeId,
    },
    /// The node belongs to a different file.
    #[error("semantic rendering received node {node:?} from file {file:?}")]
    ForeignFile {
        /// The foreign node.
        node: NodeId,
        /// The file encoded in that node.
        file: FileId,
    },
}
