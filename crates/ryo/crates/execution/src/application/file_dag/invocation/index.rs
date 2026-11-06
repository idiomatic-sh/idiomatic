//! Application-owned equality handles and direct selected projection views.
use super::*;
/// Owned immutable view handles stay in application; primitives borrow only the index.
pub(in crate::application::file_dag) enum IndexResource<'f> {
    Derived {
        rows: BorrowedRows<'f>,
        view: AcquiredView,
    },
    Empty {
        rows: BorrowedRows<'f>,
        plan: std::sync::Arc<crate::domain::derived_views::types::DerivedContract>,
        index: KeyIndex,
    },
    Selected(super::super::selected::SelectedInput<'f>),
}
impl<'f> IndexResource<'f> {
    pub(super) fn acquired(
        contract: &OperatorContract,
        original: Option<&crate::domain::table_model::types::Rows<'f>>,
        admission: &super::super::workspace::InputAdmissions<'_, 'f>,
        context: &mut OperatorContext<'_>,
        skip_empty: bool,
    ) -> Result<Self, OperatingError> {
        use ryo_derived::domain::contract::types::DerivedKind;
        let use_ = prepared(contract, DerivedKind::Equality)?;
        let place = use_.inputs[0];
        let source = admission.input(place)?;
        // Absence's established empty-RHS gate precedes key interpretation.
        if skip_empty && source.count() == 0 && !matches!(source, BorrowedRows::Selected { .. }) {
            return Ok(Self::Empty {
                rows: source,
                plan: use_.plan.clone(),
                index: KeyIndex::default(),
            });
        }
        let source = admission.rows(place)?;
        let storage = admission.acquire_rows(use_, context)?;
        if let Some(crate::domain::table_model::types::Rows::Selected(selected)) = original {
            return Ok(Self::Selected(
                super::super::selected::SelectedInput::from_derived(selected, storage)?,
            ));
        }
        Ok(Self::Derived {
            rows: source,
            view: storage,
        })
    }
    pub(in crate::application::file_dag) fn view(&self) -> IndexedRows<'_> {
        match self {
            Self::Selected(selected) => selected.view(),
            Self::Derived { rows, view } => {
                let crate::domain::derived_views::types::ViewRequest::Equality { columns } =
                    &view.contract().request
                else {
                    unreachable!("prepared equality")
                };
                let ryo_derived::domain::contract::types::DerivedView::Equality(index) =
                    view.view().expect("validated equality")
                else {
                    unreachable!("prepared equality")
                };
                IndexedRows {
                    #[cfg(feature = "diagnostic-row-progress")]
                    progress: crate::application::file_dag::route_counts::progress,
                    rows: *rows,
                    columns,
                    index,
                }
            }
            Self::Empty { rows, plan, index } => {
                let crate::domain::derived_views::types::ViewRequest::Equality { columns } =
                    &plan.request
                else {
                    unreachable!("prepared empty equality")
                };
                IndexedRows {
                    #[cfg(feature = "diagnostic-row-progress")]
                    progress: crate::application::file_dag::route_counts::progress,
                    rows: *rows,
                    columns,
                    index,
                }
            }
        }
    }
}
