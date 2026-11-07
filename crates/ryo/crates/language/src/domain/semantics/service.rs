//! The `semantics` service: free functions named by verb that admit a rendering request to the file whose tree is current, with no IO
//! and no ports.

use super::errors::SemanticsError;
use super::types::RenderRequest;
use ryo_relations::domain::relations::service::{node_file, node_position};
use ryo_relations::domain::relations::types::{Count, FileId};

/// Admits a request whose node is in the table of `file`, which holds `nodes` real elements.
///
/// # Errors
///
/// `ForeignFile` when the node belongs to another file, and `StaleNode` when the file's table has no such position.
pub fn admit(request: &RenderRequest, file: FileId, nodes: Count) -> Result<(), SemanticsError> {
    let from = node_file(request.node);
    if from != file {
        return Err(SemanticsError::ForeignFile {
            node: request.node,
            file: from,
        });
    }
    if usize::try_from(node_position(request.node).0).unwrap_or(usize::MAX) >= nodes.0 {
        return Err(SemanticsError::StaleNode { node: request.node });
    }
    Ok(())
}
