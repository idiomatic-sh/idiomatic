//! The portable contract that builds the payload of a prepared derived request from finite borrowed inputs.
use crate::domain::contract::types::derived_payload::value::DerivedPayload;
use crate::domain::contract::types::prepared_derived::value::PreparedDerived;
use crate::domain::contract::types::{BuildInputs, BuildOutcome, BuildWorkspace};
use crate::domain::fault::types::DerivedFault;

/// A build neither acquires inputs nor retains them; a complete implementation compiles.
///
/// ```
/// use ryo_derived::domain::contract::types::derived_payload::value::DerivedPayload;
/// use ryo_derived::domain::contract::types::prepared_derived::value::PreparedDerived;
/// use ryo_derived::domain::contract::types::{BuildInputs, BuildOutcome, BuildWorkspace};
/// use ryo_derived::domain::fault::types::DerivedFault;
/// use ryo_derived::inbound::use_cases::build::interface::BuildUseCase;
/// struct Item;
/// impl BuildUseCase for Item {
///     fn build(&self, _prepared: &PreparedDerived, _inputs: BuildInputs<'_>, _room: &mut BuildWorkspace, _recycled: Option<DerivedPayload>) -> Result<BuildOutcome, DerivedFault> { unimplemented!() }
/// }
/// ```
///
/// A missing `build` is refused.
///
/// ```compile_fail,E0046
/// use ryo_derived::inbound::use_cases::build::interface::BuildUseCase;
/// struct Item;
/// impl BuildUseCase for Item {}
/// ```
pub trait BuildUseCase: Sync {
    /// Builds the payload of the prepared request, reusing the buffers of a recycled payload.
    ///
    /// # Errors
    /// The fault of the request, the inputs or the build.
    fn build(
        &self,
        prepared: &PreparedDerived,
        inputs: BuildInputs<'_>,
        room: &mut BuildWorkspace,
        recycled: Option<DerivedPayload>,
    ) -> Result<BuildOutcome, DerivedFault>;
}
