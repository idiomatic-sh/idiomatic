//! The service of the model's per-file view. Track P1's placeholder of the file track P2 writes: nothing is built; the part asked for is kept.

use super::types::{Built, ModelPart, SemanticModel};
use ruff_python_ast::AnyNodeRef;
use ryo::languages::NodeTable;

/// Builds the parts of `model` that `part` needs and `built` lacks, from the parser's own nodes.
pub(crate) fn built(
    model: &mut SemanticModel,
    built: &mut Built,
    part: ModelPart,
    _table: &NodeTable,
    _nodes: &[AnyNodeRef<'_>],
) {
    model.asked.push(part);
    built.asked = Some(part);
}
