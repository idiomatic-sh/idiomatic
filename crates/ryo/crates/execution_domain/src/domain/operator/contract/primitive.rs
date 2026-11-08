//! What one primitive evaluation is given and evaluates through: its prepared inputs, the injected operator meanings
//! the application supplies, and the operating error each portable operator fault stands for.
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::InputPlace;
use crate::domain::table_model::types::RepresentationInput;
use ryo_relations::domain::relations::types::Value;

/// Immutable inputs only. No Feed, callback, scheduler or owned/retained Arc is supplied.
#[derive(Clone, Copy)]
pub enum PreparedInputs<'a> {
    /// One row of the triggering input with its immutable side inputs.
    Row {
        /// The input the row comes from.
        input: InputPlace,
        /// The row's cells.
        row: &'a [Value],
        /// The side inputs the row is evaluated against.
        sides: &'a [RepresentationInput<'a>],
    },
    /// The whole inputs, each as one representation.
    Views(&'a [RepresentationInput<'a>]),
    /// The source the request supplies.
    RequestSource(&'a RepresentationInput<'a>),
}

/// Borrowed portable primitive meanings; the application supplies the implementation.
/// No outbound adapter, cache owner or scheduler is reachable from a primitive.
pub trait PrimitiveSemantics: Sync {
    #[cfg(feature = "diagnostic-row-progress")]
    fn progress(&self) -> &dyn super::views::ProgressObserver {
        &super::views::IgnoredProgress
    }

    /// The injected operators the primitive evaluates through.
    fn operators(
        &self,
        call: ryo_operators::inbound::use_cases::operators::types::OperatorCall<'_>,
    ) -> Result<
        ryo_operators::inbound::use_cases::operators::types::OperatorAnswer,
        ryo_operators::domain::contract::types::PortableOperatorFault,
    >;
}

/// The operating error a portable operator fault stands for.
pub fn portable_fault(
    error: ryo_operators::domain::contract::types::PortableOperatorFault,
) -> OperatingError {
    use ryo_operators::domain::contract::types::PortableOperatorFault;
    match error {
        PortableOperatorFault::Interrupted => OperatingError::WrongOperation,
        PortableOperatorFault::NoSuchColumn { index } => {
            ryo_values::domain::rows::errors::RowsError::NoSuchColumn { index: index.0 }.into()
        }
        PortableOperatorFault::ShapeMismatch => OperatingError::ShapeMismatch,
        PortableOperatorFault::InvalidSpanBounds => OperatingError::InvalidSpanBounds,
        PortableOperatorFault::InvalidSpan => OperatingError::InvalidSpan,
        PortableOperatorFault::InvalidSourceMatchSpan => OperatingError::InvalidSourceMatchSpan,
        PortableOperatorFault::UnboundKey => OperatingError::WrongBinding,
        PortableOperatorFault::WrongBinding => OperatingError::WrongBinding,
        PortableOperatorFault::Unbound => OperatingError::Unbound,
        PortableOperatorFault::NotRun => OperatingError::NotRun,
        PortableOperatorFault::MissingCapture
        | PortableOperatorFault::WrongOperation
        | PortableOperatorFault::UnsupportedRender => OperatingError::WrongOperation,
    }
}
