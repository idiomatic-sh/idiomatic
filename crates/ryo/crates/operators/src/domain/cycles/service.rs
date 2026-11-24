//! The cycles concept's lowering: a cycle search over a graph's edges, which the `components` kernel answers.

/// Mark vertices belonging to a cycle in a caller-supplied numbered adjacency.
pub use crate::domain::physical_plan::collections::kernels::components::item::components;

/// The graph of a cycle search over edges whose `source` leads to `destination`: its one lowering, Tarjan's components
/// over its edges.
pub(crate) const fn lower(
    source: ryo_values::domain::plan_parts::types::ColumnIndex,
    destination: ryo_values::domain::plan_parts::types::ColumnIndex,
) -> super::types::CycleParts {
    super::types::CycleParts {
        source,
        destination,
    }
}
