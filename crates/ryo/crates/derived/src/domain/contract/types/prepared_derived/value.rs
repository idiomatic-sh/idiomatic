//! Prepared requests: the value and its behaviour, owned together.
use crate::domain::fault::types::DerivedFault;
use crate::domain::request::types::DerivedRequest;
use ryo_relations::domain::relations::types::CellKind;

/// Preparation stores metadata failures until the original consumer demand gate.
///
/// A caller cannot construct prepared metadata; only preparation does.
///
/// ```compile_fail,E0451
/// use ryo_derived::domain::contract::types::prepared_derived::value::PreparedDerived;
/// fn forge() -> PreparedDerived { PreparedDerived { request: todo!(), schemas: todo!(), metadata: todo!() } }
/// ```
#[derive(Clone, Debug)]
pub struct PreparedDerived {
    pub(crate) request: DerivedRequest,
    pub(crate) schemas: Vec<Vec<CellKind>>,
    pub(crate) metadata: Result<(), DerivedFault>,
}

impl PreparedDerived {
    /// The request, with its literals in canonical order.
    #[must_use]
    pub fn request(&self) -> &DerivedRequest {
        &self.request
    }
    /// The schema of each input.
    #[must_use]
    pub fn schemas(&self) -> &[Vec<CellKind>] {
        &self.schemas
    }
    /// The metadata failure preparation stored, if any.
    ///
    /// # Errors
    /// The stored metadata fault.
    pub fn validate(&self) -> Result<(), DerivedFault> {
        self.metadata.clone()
    }
}
