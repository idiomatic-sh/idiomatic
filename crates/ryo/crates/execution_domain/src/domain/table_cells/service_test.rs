//! The shared table subjects the crate's tests build from the prepared derived payloads.

// The prepared derived payloads the crate's tests build their subjects from, through the derived crate's own use cases.
use crate::domain::table_model::types::Rows;
use ryo_derived::domain::equality::types::KeyIndex;
use ryo_derived::inbound::use_cases::build::interface::BuildUseCase as _;
use ryo_derived::inbound::use_cases::build::use_case::BuildUseCaseImpl;
use ryo_derived::inbound::use_cases::prepare_derived::interface::PrepareDerivedUseCase as _;
use ryo_derived::inbound::use_cases::prepare_derived::use_case::PrepareDerivedUseCaseImpl;
use ryo_derived::inbound::use_cases::view::interface::ViewUseCase as _;
use ryo_derived::inbound::use_cases::view::use_case::ViewUseCaseImpl;
use ryo_relations::domain::relations::types::CellKind;
use ryo_relations::domain::relations::types::Value as Cell;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::errors::RowsError;
use ryo_values::domain::rows::types::RowIndex;
/// Finite typed input adapter for pure semantic conformance, with no cache or scheduler.
pub(crate) struct SemanticRows<'f>(pub crate::domain::table_model::types::Rows<'f>);
impl ryo_derived::domain::row_cells::types::RowCells for SemanticRows<'_> {
    fn bulk(&self) -> Option<ryo_derived::domain::row_cells::types::BulkRows<'_>> {
        match &self.0 {
            Rows::Stored(source) => Some(ryo_derived::domain::row_cells::types::BulkRows::Stored(
                *source,
            )),
            Rows::Computed(table) => Some(
                ryo_derived::domain::row_cells::types::BulkRows::Computed(table),
            ),
            Rows::Tree(_) | Rows::Compact(_) => None,
        }
    }
    fn validate(&self) -> Result<(), ryo_derived::domain::fault::types::DerivedFault> {
        Ok(())
    }
    fn kinds(&self) -> &[CellKind] {
        crate::domain::table_model::service::kinds_of(&self.0)
    }
    fn count(&self) -> ryo_values::domain::rows::types::RowCount {
        crate::domain::table_model::service::count_of(&self.0)
    }
    fn cell(&self, row: RowIndex, column: ColumnIndex) -> Option<Cell> {
        crate::domain::table_model::service::cell_at(&self.0, row, column)
    }
}
pub(crate) fn prepared_payload(
    request: ryo_derived::domain::request::types::DerivedRequest,
    rows: &Rows<'_>,
) -> Result<ryo_derived::domain::contract::types::derived_payload::value::DerivedPayload, RowsError>
{
    use ryo_derived::domain::contract::types::BuildInputs;
    use ryo_derived::domain::contract::types::BuildOutcome;
    use ryo_derived::domain::contract::types::BuildWorkspace;
    use ryo_derived::domain::fault::types::DerivedFault;
    let prepared = PrepareDerivedUseCaseImpl::new().prepare_derived(
        request,
        ryo_derived::domain::contract::types::InputSchemas(vec![
            crate::domain::table_model::service::kinds_of(rows).to_vec(),
        ]),
    );
    let input = SemanticRows(rows.clone());
    match BuildUseCaseImpl::new().build(
        &prepared,
        BuildInputs::Rows(&input),
        &mut BuildWorkspace::default(),
        ryo_derived::domain::contract::types::Recycled::Nothing,
    ) {
        Ok(BuildOutcome::Built(payload)) => Ok(payload),
        Err(DerivedFault::NoSuchColumn { column }) => {
            Err(RowsError::NoSuchColumn { index: column.0 })
        }
        other => panic!("unexpected semantic fixture result: {other:?}"),
    }
}
pub(crate) fn prepared_index(
    rows: &Rows<'_>,
    columns: &[ColumnIndex],
) -> Result<std::sync::Arc<KeyIndex>, RowsError> {
    prepared_payload(
        ryo_derived::domain::request::types::DerivedRequest::Equality {
            columns: columns.to_vec(),
        },
        rows,
    )
    .map(|payload| {
        payload
            .share_equality()
            .expect("successful equality payload")
            .0
    })
}

pub(crate) fn prepared_order(
    rows: &Rows<'_>,
    columns: &[ColumnIndex],
) -> Result<Vec<RowIndex>, RowsError> {
    use ryo_derived::domain::contract::types::DerivedView;
    use ryo_derived::domain::request::types::DerivedRequest;
    let request = DerivedRequest::PartitionOrder {
        keys: columns.to_vec(),
    };
    let payload = prepared_payload(request.clone(), rows)?;
    let plan = PrepareDerivedUseCaseImpl::new().prepare_derived(
        request,
        ryo_derived::domain::contract::types::InputSchemas(vec![
            crate::domain::table_model::service::kinds_of(rows).to_vec(),
        ]),
    );
    let DerivedView::PartitionOrder(order) = ViewUseCaseImpl::new()
        .view(&plan, &payload)
        .expect("prepared order")
    else {
        panic!("partition order")
    };
    Ok(order.to_vec())
}
