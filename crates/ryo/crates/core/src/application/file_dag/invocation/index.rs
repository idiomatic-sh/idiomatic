//! Application-owned equality handles and direct selected projection views.
use super::*;
/// Owned immutable view handles stay in application; primitives borrow only the index.
pub(in crate::application::file_dag) enum IndexResource<'f> {
    Derived {
        rows: BorrowedRows<'f>,
        plan: std::sync::Arc<crate::domain::derived_views::types::DerivedContract>,
        storage: std::sync::Arc<crate::domain::derived_views::types::DerivedStorage>,
    },
    Selected(super::super::selected::SelectedInput<'f>),
}
impl<'f> IndexResource<'f> {
    pub(super) fn acquired(
        contract: &OperatorContract,
        mode: EvaluationMode,
        inputs: &[BorrowedRows<'f>],
        source: BorrowedRows<'f>,
        original: Option<&crate::domain::table_model::types::Rows<'f>>,
        admission: &super::super::workspace::InvocationWorkspace,
        context: &mut OperatorContext<'_>,
        skip_empty: bool,
    ) -> Result<Self, OperatingError> {
        use crate::domain::derived_views::types::{DerivedKind, DerivedStorage};
        let use_ = prepared(contract, mode, DerivedKind::Equality)?;
        // Absence's established empty-RHS gate precedes key interpretation.
        if skip_empty && source.count() == 0 && !matches!(source, BorrowedRows::Selected { .. }) {
            return Ok(Self::Derived {
                rows: source,
                plan: std::sync::Arc::clone(&use_.plan),
                storage: std::sync::Arc::new(DerivedStorage::Equality(KeyIndex::default())),
            });
        }
        let storage = admission.acquire_rows(use_, inputs, context)?;
        if let Some(crate::domain::table_model::types::Rows::Selected(selected)) = original {
            return Ok(Self::Selected(
                super::super::selected::SelectedInput::from_derived(
                    selected,
                    std::sync::Arc::clone(&use_.plan),
                    storage,
                )?,
            ));
        }
        Ok(Self::Derived {
            rows: source,
            plan: std::sync::Arc::clone(&use_.plan),
            storage,
        })
    }
    pub(in crate::application::file_dag) fn view(&self) -> IndexedRows<'_> {
        match self {
            Self::Selected(selected) => selected.view(),
            Self::Derived {
                rows,
                plan,
                storage,
            } => {
                let crate::domain::derived_views::types::ViewRequest::Equality { columns } =
                    &plan.request
                else {
                    unreachable!("prepared equality")
                };
                let crate::domain::derived_views::types::DerivedView::Equality(index) =
                    crate::domain::derived_views::collections::views::registration::registered(
                        plan.kind,
                    )
                    .view(plan, storage)
                    .expect("validated equality storage")
                else {
                    unreachable!("prepared equality view")
                };
                IndexedRows {
                    rows: *rows,
                    columns,
                    index,
                }
            }
        }
    }
    pub(in crate::application::file_dag) fn selected(
        &self,
    ) -> Option<&super::super::selected::SelectedInput<'_>> {
        match self {
            Self::Selected(selected) => Some(selected),
            Self::Derived { .. } => None,
        }
    }
}
