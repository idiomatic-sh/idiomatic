//! The failures of the `semantics` concept: a rendering request that does not belong to the current file's tree.

use ryo_relations::domain::relations::types::{FileId, NodeId};

/// Why a rendering request was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum SemanticsError {
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
