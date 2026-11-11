//! Application-owned equality handles.
use crate::application::file_dag::invocation::prepared_use::service::prepared;
use ryo_derived::domain::equality::types::KeyIndex;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::operator::contract::types::OperatorContract;
use ryo_execution_domain::domain::operator::types::OperatorContext;
use ryo_execution_domain::domain::table_model::types::RepresentationInput;
use ryo_execution_domain::domain::table_reach::borrowed::IndexedRows;
use ryo_execution_store::application::file_dag::derived::owner::acquisition::admission::service::AcquiredView;
use ryo_values::domain::plan_parts::types::InputNumber;
/// Owned immutable view handles stay in application; primitives borrow only the index.
/// Whether an index over an empty input is skipped or still built.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum EmptyInput {
    Skip,
    Keep,
}
/// An equality index the invocation holds: its key columns are read from the plan once, at acquisition, so a plan
/// that is not an equality is refused there and the variants hold only what an equality index has.
pub(crate) enum IndexResource<'f> {
    Derived {
        rows: RepresentationInput<'f>,
        columns: Vec<ryo_values::domain::plan_parts::types::ColumnIndex>,
        view: AcquiredView,
    },
    Empty {
        rows: RepresentationInput<'f>,
        columns: Vec<ryo_values::domain::plan_parts::types::ColumnIndex>,
        index: KeyIndex,
    },
}
impl<'f> IndexResource<'f> {
    pub(super) fn acquired(
        contract: &OperatorContract,
        admission: &ryo_execution_store::application::file_dag::workspace::service::InputAdmissions<
            '_,
            'f,
        >,
        context: &mut OperatorContext<'_>,
        empty: EmptyInput,
    ) -> Result<Self, OperatingError> {
        use ryo_derived::domain::request::types::DerivedKind;
        let use_ = prepared(contract, DerivedKind::Equality)?;
        let ryo_execution_domain::domain::derived_views::types::ViewRequest::Equality { columns } =
            &use_.plan.request
        else {
            return Err(OperatingError::WrongOperation);
        };
        let columns = columns.clone();
        let place = use_.inputs[0];
        let source = admission.input(place)?;
        // Absence's established empty-RHS gate precedes key interpretation.
        if empty == EmptyInput::Skip && source.count().0 == 0 {
            return Ok(Self::Empty {
                rows: source,
                columns,
                index: KeyIndex::default(),
            });
        }
        let source = admission.rows(place)?;
        let storage = admission.acquire_rows(use_, context)?;
        Ok(Self::Derived {
            rows: source,
            columns,
            view: storage,
        })
    }
    /// The index the rows are looked up through. The derived port answers the view of the plan it built; an answer of
    /// another family lends no index, so the primitive refuses its missing index instead of reading a wrong one.
    pub(crate) fn view<'s>(
        &'s self,
        #[cfg(feature = "diagnostic-row-progress")]
        progress: &'s dyn ryo_execution_domain::domain::operating::types::ProgressObserver,
    ) -> Option<IndexedRows<'s>> {
        let (rows, columns, index) = match self {
            Self::Derived {
                rows,
                columns,
                view,
            } => {
                let Ok(ryo_derived::domain::contract::types::DerivedView::Equality(index)) =
                    view.view()
                else {
                    return None;
                };
                (rows, columns, index)
            }
            Self::Empty {
                rows,
                columns,
                index,
            } => (rows, columns, index),
        };
        Some(IndexedRows {
            #[cfg(feature = "diagnostic-row-progress")]
            progress,
            rows: *rows,
            columns,
            index,
        })
    }
}
