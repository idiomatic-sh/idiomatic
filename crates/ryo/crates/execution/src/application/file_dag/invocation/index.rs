//! Application-owned equality handles.
use ryo_values::domain::plan_parts::types::InputNumber;
use crate::application::file_dag::derived::owner::acquisition::admission::service::AcquiredView;
use crate::application::file_dag::invocation::prepared_use::service::prepared;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::operating::types::OperatorContext;
use ryo_execution_domain::domain::operator::contract::types::OperatorContract;
use ryo_execution_domain::domain::operator::contract::views::IndexedRows;
use ryo_execution_domain::domain::table_model::types::RepresentationInput;
use ryo_derived::domain::equality::types::KeyIndex;
/// Owned immutable view handles stay in application; primitives borrow only the index.
/// Whether an index over an empty input is skipped or still built.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum EmptyInput {
    Skip,
    Keep,
}
pub(crate) enum IndexResource<'f> {
    Derived {
        rows: RepresentationInput<'f>,
        view: AcquiredView,
    },
    Empty {
        rows: RepresentationInput<'f>,
        plan: std::sync::Arc<ryo_execution_domain::domain::derived_views::types::contract::value::DerivedContract>,
        index: KeyIndex,
    },
}
impl<'f> IndexResource<'f> {
    pub(super) fn acquired(
        contract: &OperatorContract,
        admission: &super::super::workspace::InputAdmissions<'_, 'f>,
        context: &mut OperatorContext<'_>,
        empty: EmptyInput,
    ) -> Result<Self, OperatingError> {
        use ryo_derived::domain::request::types::DerivedKind;
        let use_ = prepared(contract, DerivedKind::Equality)?;
        let place = use_.inputs[0];
        let source = admission.input(place)?;
        // Absence's established empty-RHS gate precedes key interpretation.
        if empty == EmptyInput::Skip && source.count().0 == 0 {
            return Ok(Self::Empty {
                rows: source,
                plan: use_.plan.clone(),
                index: KeyIndex::default(),
            });
        }
        let source = admission.rows(place)?;
        let storage = admission.acquire_rows(use_, context)?;
        Ok(Self::Derived {
            rows: source,
            view: storage,
        })
    }
    pub(crate) fn view<'s>(
        &'s self,
        #[cfg(feature = "diagnostic-row-progress")]
        progress: &'s dyn ryo_execution_domain::domain::operator::contract::views::ProgressObserver,
    ) -> IndexedRows<'s> {
        match self {
            Self::Derived { rows, view } => {
                let ryo_execution_domain::domain::derived_views::types::ViewRequest::Equality { columns } =
                    &view.contract().request
                else {
                    unreachable!("prepared equality")
                };
                let ryo_derived::domain::contract::types::DerivedView::Equality(index) =
                    view.view().expect(super::constants::VALIDATED_EQUALITY)
                else {
                    unreachable!("prepared equality")
                };
                IndexedRows {
                    #[cfg(feature = "diagnostic-row-progress")]
                    progress,
                    rows: *rows,
                    columns,
                    index,
                }
            }
            Self::Empty { rows, plan, index } => {
                let ryo_execution_domain::domain::derived_views::types::ViewRequest::Equality { columns } =
                    &plan.request
                else {
                    unreachable!("prepared empty equality")
                };
                IndexedRows {
                    #[cfg(feature = "diagnostic-row-progress")]
                    progress,
                    rows: *rows,
                    columns,
                    index,
                }
            }
        }
    }
}
