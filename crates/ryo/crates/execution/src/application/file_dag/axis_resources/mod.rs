//! Axis admission and engine-owned prepared views over original borrowed populations.
use ryo_derived::domain::contract::types::DerivedView;
use ryo_derived::domain::membership::types::MembershipView;
pub(crate) mod view;
use view::AxisResource;
#[cfg(test)]
mod test;

use super::derived::owner::acquisition::admission::service::AcquiredView;
use ryo_execution_domain::domain::{
    operating::errors::OperatingError,
    operator::contract::{resources::AxisResources, },
    table_cells::types::{TreeForm, TreeRows},
    table_model::service::check_columns,
    tree_walk::service::entity_of,
};
use ryo_execution_domain::domain::table_model::types::RepresentationInput;
use ryo_values::domain::plan_parts::types::{ColumnIndex, Direction};

#[derive(Clone, Copy)]
pub(crate) enum AxisMode<'f> {
    Empty,
    Native {
        tree: TreeRows<'f>,
        among: Option<TreeRows<'f>>,
    },
    Indexed {
        from: [ColumnIndex; 1],
        to: ColumnIndex,
    },
}

/// Preserve the old admission and error gates before acquiring any derived resource.
pub(crate) fn classify<'f>(
    program: &AxisResources,
    edges: RepresentationInput<'f>,
    domain: RepresentationInput<'f>,
) -> Result<AxisMode<'f>, OperatingError> {
    let AxisResources {
        parent,
        child,
        domain_id,
        has_filter,
        domain_view,
        output,
        direction,
        ..
    } = program;
    if matches!(domain, RepresentationInput::Computed(_) | RepresentationInput::Compact(_))
        && domain.count().0 == 0
        && output.is_empty()
    {
        return Ok(AxisMode::Empty);
    }
    if let RepresentationInput::Tree(tree) = edges {
        let domain_of_tree = matches!(domain, RepresentationInput::Tree(each) if each.form == TreeForm::Elements)
            && entity_of(domain_view, *domain_id) == ColumnIndex(0);
        if tree.form == TreeForm::Edges
            && (*parent, *child) == (ColumnIndex(0), ColumnIndex(1))
            && ((*has_filter == ryo_execution_domain::domain::operator::contract::resources::Filtering::Unfiltered && domain_view.is_empty()) || domain_of_tree)
        {
            let among = if let RepresentationInput::Tree(each) = domain {
                (each.form == TreeForm::Elements).then_some(each)
            } else {
                None
            };
            return Ok(AxisMode::Native { tree, among });
        }
    }
    if *has_filter == ryo_execution_domain::domain::operator::contract::resources::Filtering::Filtered
        || !domain_view.is_empty()
        || !output.is_empty()
        || matches!(
            direction,
            Direction::Deep | Direction::PreviousSibling | Direction::NextSibling
        )
    {
        return Err(OperatingError::WrongOperation);
    }
    check_columns(edges.kinds(), &[*parent, *child])?;
    check_columns(domain.kinds(), &[*domain_id])?;
    let (from, to) = match direction {
        Direction::Parent | Direction::Ancestor | Direction::NearestAncestor => ([*child], *parent),
        _ => ([*parent], *child),
    };
    Ok(AxisMode::Indexed { from, to })
}

pub(crate) fn prepare_axis<'f>(
    program: &AxisResources,
    edges: RepresentationInput<'f>,
    domain: RepresentationInput<'f>,
    equality: Option<AcquiredView>,
    membership: Option<AcquiredView>,
) -> Result<AxisResource<'f>, OperatingError> {
    match classify(program, edges, domain)? {
        AxisMode::Empty => Ok(AxisResource::Empty),
        AxisMode::Native { tree, among } => {
            if among.is_none()
                && !matches!(
                    membership.as_ref().map(AcquiredView::view).transpose()?,
                    Some(DerivedView::Membership(MembershipView::Dense(_)))
                )
            {
                return Err(OperatingError::WrongBinding);
            }
            Ok(AxisResource::Native {
                tree,
                among,
                membership,
            })
        }
        AxisMode::Indexed { from, to } => {
            if !matches!(
                equality.as_ref().map(AcquiredView::view).transpose()?,
                Some(DerivedView::Equality(_))
            ) || !matches!(
                membership.as_ref().map(AcquiredView::view).transpose()?,
                Some(DerivedView::Membership(MembershipView::Values(_)))
            ) {
                return Err(OperatingError::WrongBinding);
            }
            Ok(AxisResource::Indexed {
                edges,
                from,
                to,
                equality: equality.ok_or(OperatingError::WrongBinding)?,
                membership: membership.ok_or(OperatingError::WrongBinding)?,
            })
        }
    }
}
