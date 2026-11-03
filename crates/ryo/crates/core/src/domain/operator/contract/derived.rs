//! Finite scoped semantic access; no owner, ledger or scheduler crosses this boundary.
use crate::domain::operating::errors::OperatingError;
use ryo_derived::domain::contract::types::RawOrder;

/// The engine holds the required payload lease until the callback returns, including error paths.
/// A borrowed order cannot escape the callback or retain upstream input authority.
pub(crate) trait ScopedDerived: std::fmt::Debug {
    fn with_raw_order(
        &mut self,
        keys: &[u64],
        stride: usize,
        body: &mut dyn FnMut(&RawOrder) -> Result<(), OperatingError>,
    ) -> Result<(), OperatingError>;
}

pub(crate) fn portable_fault(
    fault: ryo_derived::domain::contract::types::DerivedFault,
) -> OperatingError {
    use ryo_derived::domain::contract::types::DerivedFault;
    match fault {
        DerivedFault::MissingInput { position } => OperatingError::MissingInput { position },
        DerivedFault::NoSuchColumn { column } => {
            crate::domain::table_model::errors::TableModelError::NoSuchColumn { index: column.0 }
                .into()
        }
        DerivedFault::DuplicateOrder => OperatingError::DuplicateOrder,
        DerivedFault::DuplicateDfsOrder => OperatingError::DuplicateDfsOrder,
        DerivedFault::ShapeMismatch => OperatingError::ShapeMismatch,
        DerivedFault::NotRun => OperatingError::NotRun,
        DerivedFault::WrongOperation | DerivedFault::Interrupted => OperatingError::WrongOperation,
    }
}
