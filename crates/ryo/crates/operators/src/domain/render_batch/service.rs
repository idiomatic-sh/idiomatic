//! The whole batch is checked before any row is delivered.

/// The one lowering of a render: its node column and the role the node is rendered in, for the `render` kernel.
pub(crate) fn lower(
    node: ryo_values::domain::plan_parts::types::ColumnIndex,
    context: ryo_language::domain::semantics::types::RenderContext,
) -> super::types::RenderParts {
    super::types::RenderParts { node, context }
}
