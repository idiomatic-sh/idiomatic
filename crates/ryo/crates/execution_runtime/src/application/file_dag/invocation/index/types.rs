//! The types of the `index` concept: plain data, public fields and derives, with no constructor and no inherent impl.

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
