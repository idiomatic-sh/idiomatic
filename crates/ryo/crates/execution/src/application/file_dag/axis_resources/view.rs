//! Owned derived handles stay in application state; the primitive sees borrowed views only.
use super::super::derived::owner::acquisition::admission::service::AcquiredView;
use ryo_execution_domain::domain::{
    operating::errors::OperatingError,
    operator::contract::{
        views::PreparedAxis,
        views::{IndexedRows},
    },
    table_cells::types::TreeRows,
};
use ryo_execution_domain::domain::table_model::types::RepresentationInput;
use ryo_derived::domain::contract::types::DerivedView;
use ryo_derived::domain::membership::types::MembershipView;
use ryo_derived::domain::positions::types::DenseMarks;
use ryo_values::domain::plan_parts::types::ColumnIndex;

pub(crate) enum AxisResource<'f> {
    Empty,
    Native {
        tree: TreeRows<'f>,
        among: Option<TreeRows<'f>>,
        membership: Option<AcquiredView>,
    },
    Indexed {
        edges: RepresentationInput<'f>,
        from: [ColumnIndex; 1],
        to: ColumnIndex,
        equality: AcquiredView,
        membership: AcquiredView,
    },
}

impl AxisResource<'_> {
    pub(crate) fn view<'s>(
        &'s self,
        #[cfg(feature = "diagnostic-row-progress")]
        progress: &'s dyn ryo_execution_domain::domain::operator::contract::views::ProgressObserver,
    ) -> Result<PreparedAxis<'s>, OperatingError> {
        match self {
            Self::Empty => Ok(PreparedAxis::Empty),
            Self::Native {
                tree,
                among,
                membership,
            } => {
                let marks = match membership.as_ref().map(AcquiredView::view).transpose()? {
                    Some(DerivedView::Membership(MembershipView::Dense(marks))) => marks,
                    None if among.is_some() => DenseMarks(&[]),
                    _ => return Err(OperatingError::WrongBinding),
                };
                Ok(PreparedAxis::Native {
                    tree: *tree,
                    among: *among,
                    marks,
                })
            }
            Self::Indexed {
                edges,
                from,
                to,
                equality,
                membership,
            } => {
                let DerivedView::Equality(index) = equality.view()? else {
                    return Err(OperatingError::WrongBinding);
                };
                let DerivedView::Membership(domain @ MembershipView::Values(_)) =
                    membership.view()?
                else {
                    return Err(OperatingError::WrongBinding);
                };
                Ok(PreparedAxis::Indexed {
                    edges: IndexedRows {
                        #[cfg(feature = "diagnostic-row-progress")]
                        progress,
                        rows: *edges,
                        columns: from,
                        index,
                    },
                    to: *to,
                    domain,
                })
            }
        }
    }
}
