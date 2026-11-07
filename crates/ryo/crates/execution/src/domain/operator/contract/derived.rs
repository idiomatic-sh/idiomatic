//! Portable meaning of derived faults.
use crate::domain::operating::errors::OperatingError;

pub(crate) fn portable_fault(
    fault: ryo_derived::domain::fault::types::DerivedFault,
) -> OperatingError {
    use ryo_derived::domain::fault::types::DerivedFault;
    match fault {
        DerivedFault::MissingInput { position } => OperatingError::MissingInput { position: position.0 },
        DerivedFault::NoSuchColumn { column } => {
            ryo_values::domain::rows::errors::RowsError::NoSuchColumn { index: column.0 }.into()
        }
        DerivedFault::DuplicateOrder => OperatingError::DuplicateOrder,
        DerivedFault::DuplicateDfsOrder => OperatingError::DuplicateDfsOrder,
        DerivedFault::ShapeMismatch => OperatingError::ShapeMismatch,
        DerivedFault::NotRun => OperatingError::NotRun,
        DerivedFault::WrongOperation | DerivedFault::Interrupted => OperatingError::WrongOperation,
    }
}
