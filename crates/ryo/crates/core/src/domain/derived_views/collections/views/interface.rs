//! The domain supplies algorithms, never a scheduler, cache or population owner.
use crate::domain::{
    derived_views::types::*, operating::errors::OperatingError,
    operation_schema::types::InputSchema,
};

pub(crate) trait DerivedViewsItem: Sync {
    /// Static metadata preparation; never reads source rows or acquires resources.
    fn prepare(
        &self,
        request: &ViewRequest,
        inputs: &[InputSchema<'_>],
    ) -> Result<DerivedContract, OperatingError>;
    /// Fill engine-owned storage using finite borrowed semantic inputs and scratch.
    /// The caller controls demand gates, allocation allowance, publication and reuse.
    fn build(
        &self,
        contract: &DerivedContract,
        inputs: DerivedInputs<'_, '_>,
        storage: &mut DerivedStorage,
        scratch: &mut DerivedScratch,
    ) -> Result<(), OperatingError>;
    /// Borrow already-built data only. No source input, scratch, owner or build capability.
    fn view<'a>(
        &self,
        contract: &DerivedContract,
        storage: &'a DerivedStorage,
    ) -> Result<DerivedView<'a>, OperatingError>;
}
