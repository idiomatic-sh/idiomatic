//! The portable contract that checks a derived request against the schemas of its inputs.
use crate::domain::contract::types::prepared_derived::value::PreparedDerived;
use crate::domain::request::types::DerivedRequest;
use ryo_relations::domain::relations::types::CellKind;

/// Preparation never reads rows; a complete implementation compiles.
///
/// ```
/// use ryo_derived::domain::contract::types::prepared_derived::value::PreparedDerived;
/// use ryo_derived::domain::request::types::DerivedRequest;
/// use ryo_derived::inbound::use_cases::prepare_derived::interface::PrepareDerivedUseCase;
/// struct Item;
/// impl PrepareDerivedUseCase for Item {
///     fn prepare_derived(&self, _request: DerivedRequest, _schemas: Vec<Vec<ryo_relations::domain::relations::types::CellKind>>) -> PreparedDerived { unimplemented!() }
/// }
/// ```
///
/// A missing `prepare_derived` is refused.
///
/// ```compile_fail,E0046
/// use ryo_derived::inbound::use_cases::prepare_derived::interface::PrepareDerivedUseCase;
/// struct Item;
/// impl PrepareDerivedUseCase for Item {}
/// ```
pub trait PrepareDerivedUseCase: Sync {
    /// Keeps the request with its literals in canonical order and the verdict of its metadata check.
    fn prepare_derived(&self, request: DerivedRequest, schemas: Vec<Vec<CellKind>>) -> PreparedDerived;
}
