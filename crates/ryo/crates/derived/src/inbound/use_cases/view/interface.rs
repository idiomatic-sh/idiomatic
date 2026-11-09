//! The portable contract that borrows the view of a built derived payload.
use crate::domain::contract::types::DerivedView;
use crate::domain::contract::types::derived_payload::value::DerivedPayload;
use crate::domain::contract::types::prepared_derived::value::PreparedDerived;
use crate::domain::fault::types::DerivedFault;

/// A view borrows an existing payload and cannot acquire inputs or build; a complete implementation compiles.
///
/// ```
/// use ryo_derived::domain::contract::types::derived_payload::value::DerivedPayload;
/// use ryo_derived::domain::contract::types::prepared_derived::value::PreparedDerived;
/// use ryo_derived::domain::contract::types::DerivedView;
/// use ryo_derived::domain::fault::types::DerivedFault;
/// use ryo_derived::inbound::use_cases::view::interface::ViewUseCase;
/// struct Item;
/// impl ViewUseCase for Item {
///     fn view<'a>(&self, _prepared: &PreparedDerived, _payload: &'a DerivedPayload) -> Result<DerivedView<'a>, DerivedFault> { unimplemented!() }
/// }
/// ```
///
/// A missing `view` is refused.
///
/// ```compile_fail,E0046
/// use ryo_derived::inbound::use_cases::view::interface::ViewUseCase;
/// struct Item;
/// impl ViewUseCase for Item {}
/// ```
///
/// A view that takes inputs is refused: it cannot acquire or build.
///
/// ```compile_fail,E0050
/// use ryo_derived::domain::contract::types::derived_payload::value::DerivedPayload;
/// use ryo_derived::domain::contract::types::prepared_derived::value::PreparedDerived;
/// use ryo_derived::domain::contract::types::{BuildInputs, DerivedView};
/// use ryo_derived::domain::fault::types::DerivedFault;
/// use ryo_derived::inbound::use_cases::view::interface::ViewUseCase;
/// struct Item;
/// impl ViewUseCase for Item {
///     fn view<'a>(&self, _prepared: &PreparedDerived, _payload: &'a DerivedPayload, _inputs: BuildInputs<'_>) -> Result<DerivedView<'a>, DerivedFault> { unimplemented!() }
/// }
/// ```
///
/// A view cannot outlive the payload it borrows.
///
/// ```compile_fail,E0515
/// use ryo_derived::domain::contract::types::derived_payload::value::DerivedPayload;
/// use ryo_derived::domain::contract::types::prepared_derived::value::PreparedDerived;
/// use ryo_derived::domain::contract::types::DerivedView;
/// use ryo_derived::inbound::use_cases::view::interface::ViewUseCase;
/// fn escape<'a>(item: &dyn ViewUseCase, prepared: &PreparedDerived, payload: DerivedPayload) -> DerivedView<'a> {
///     item.view(prepared, &payload).unwrap()
/// }
/// ```
pub trait ViewUseCase: Sync {
    /// Borrows the view of the payload, which must be of the request's kind.
    ///
    /// # Errors
    /// The stored metadata fault, or a payload of another kind.
    fn view<'a>(
        &self,
        prepared: &PreparedDerived,
        payload: &'a DerivedPayload,
    ) -> Result<DerivedView<'a>, DerivedFault>;
}
