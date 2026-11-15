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
