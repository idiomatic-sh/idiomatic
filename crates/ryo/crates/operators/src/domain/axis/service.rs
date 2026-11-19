//! Each admitted candidate of an axis walk is one row, in walk order; the nearest ancestor is the first.
use ryo_values::domain::plan_parts::types::ColumnIndex;

/// The one lowering of an axis: its walk's columns and direction, and its domain's comparisons compiled as a filter's.
pub(crate) fn lower(
    shape: crate::domain::axis::types::AxisShape<'_>,
    domain_filter: &[ryo_values::domain::plan_parts::types::PlanComparison],
    domain_view: &[ColumnIndex],
    bindings: &[ryo_values::domain::query_bindings::types::ParameterBinding],
) -> crate::domain::axis::types::AxisParts {
    crate::domain::axis::types::AxisParts {
        direction: shape.direction,
        id: shape.id,
        parent: shape.parent,
        child: shape.child,
        domain_id: shape.domain_id,
        domain: crate::domain::select::service::lower(domain_filter, domain_view, bindings),
        output: shape.output.to_vec(),
    }
}

/// The keyed columns of an axis's edges: from the child to the parent upward, else downward.
pub(crate) fn keyed_columns(axis: &super::types::AxisParts) -> super::types::KeyedColumns {
    use ryo_values::domain::plan_parts::types::Direction;
    let up = matches!(
        axis.direction,
        Direction::Parent | Direction::Ancestor | Direction::NearestAncestor
    );
    let (from, to) = if up {
        (axis.child, axis.parent)
    } else {
        (axis.parent, axis.child)
    };
    super::types::KeyedColumns { from, to }
}
