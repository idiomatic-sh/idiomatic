//! One provider answer per request and the engine-owned delivery of the known ones.



/// What a render lowers to: the node column and the role it is rendered in.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct RenderParts {
    /// The node column.
    pub(crate) node: ryo_values::domain::plan_parts::types::ColumnIndex,
    /// The role the node is embedded in.
    pub(crate) context: ryo_language::domain::semantics::types::RenderContext,
}
