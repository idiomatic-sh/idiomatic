//! Owned derived handles stay in application state; the primitive sees borrowed views only.
use super::super::derived::owner::acquisition::admission::service::AcquiredView;
use crate::domain::{
    operating::errors::OperatingError,
    operator::contract::{
        axis::PreparedAxis,
        views::{BorrowedRows, IndexedRows},
    },
    table_cells::types::TreeRows,
};
use ryo_derived::domain::contract::types::{DerivedView, MembershipView};
use ryo_values::domain::plan_parts::types::ColumnIndex;

pub(crate) enum AxisResource<'f> {
    Empty,
    Native {
        tree: TreeRows<'f>,
        among: Option<TreeRows<'f>>,
        membership: Option<AcquiredView>,
    },
    Indexed {
        edges: BorrowedRows<'f>,
        from: [ColumnIndex; 1],
        to: ColumnIndex,
        equality: AcquiredView,
        membership: AcquiredView,
    },
}

impl AxisResource<'_> {
    pub(crate) fn view(&self) -> Result<PreparedAxis<'_>, OperatingError> {
        match self {
            Self::Empty => Ok(PreparedAxis::Empty),
            Self::Native {
                tree,
                among,
                membership,
            } => {
                let marks = match membership.as_ref().map(AcquiredView::view).transpose()? {
                    Some(DerivedView::Membership(MembershipView::Dense(marks))) => marks,
                    None if among.is_some() => &[],
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
                        progress: crate::application::file_dag::route_counts::progress::service::progress,
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
