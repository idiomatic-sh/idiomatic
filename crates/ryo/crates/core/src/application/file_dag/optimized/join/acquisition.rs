//! Prepared equality acquisition, with no per-invocation schema preparation or local index build.
use super::diagnostic::Counts;
use crate::application::file_dag::derived::view::AcquiredView;
use crate::domain::{
    derived_views::{service::slot, types::DerivedKind},
    operating::{errors::OperatingError, types::OperatorContext},
    operator::contract::{
        types::{EvaluationMode, OperatorContract},
        views::BorrowedRows,
    },
    table_model::types::Rows,
};

pub(super) fn acquired_index(
    contract: &OperatorContract,
    mode: EvaluationMode,
    secondary: bool,
    left: &Rows<'_>,
    right: &Rows<'_>,
    admission: &super::super::super::workspace::InvocationWorkspace,
    context: &mut OperatorContext<'_>,
    counts: &mut Counts,
    eligible: bool,
) -> Result<AcquiredView, OperatingError> {
    let _route_consumer = crate::application::file_dag::route_counts::consumer("join");
    let slots = match mode {
        EvaluationMode::Streaming => &contract.derived.streaming,
        EvaluationMode::Collected => &contract.derived.collected,
    };
    let group = slots[slot(DerivedKind::Equality)]
        .as_ref()
        .ok_or(OperatingError::NotRun)?;
    let use_ = if secondary {
        group.secondary.as_deref().ok_or(OperatingError::NotRun)?
    } else {
        group.primary.as_ref()
    };
    let before = counts.enabled.then(|| admission.derived_counts());
    let inputs = [BorrowedRows::from(left), BorrowedRows::from(right)];
    let storage = admission.acquire_rows(use_, &inputs, context)?;
    if let Some(before) = before {
        counts.derived(before, admission.derived_counts(), eligible);
    }
    let acquired = storage;
    if !matches!(
        acquired.view()?,
        crate::domain::derived_views::types::DerivedView::Equality(_)
    ) {
        return Err(OperatingError::WrongBinding);
    }
    Ok(acquired)
}
