//! Native tree lookup metadata, preserving every existing node/span finding strategy.
use super::interface::DerivedViewsItem;
use crate::domain::{
    derived_views::{service, types::*},
    operating::errors::OperatingError,
    operation_schema::types::InputSchema,
    operator::contract::views::BorrowedRows,
    tree_join::service::tree_finding,
};
pub(crate) struct NativeLookupItem;
impl DerivedViewsItem for NativeLookupItem {
    fn prepare(
        &self,
        request: &ViewRequest,
        inputs: &[InputSchema<'_>],
    ) -> Result<DerivedContract, OperatingError> {
        service::prepare(DerivedKind::NativeLookup, request, inputs)
    }
    fn build(
        &self,
        contract: &DerivedContract,
        inputs: DerivedInputs<'_, '_>,
        storage: &mut DerivedStorage,
        _scratch: &mut DerivedScratch,
    ) -> Result<(), OperatingError> {
        service::validate(DerivedKind::NativeLookup, contract)?;
        let (
            ViewRequest::NativeLookup {
                left_on,
                right_on,
                view,
            },
            DerivedInputs::Rows(inputs),
        ) = (&contract.request, inputs)
        else {
            return Err(OperatingError::WrongOperation);
        };
        let Some(BorrowedRows::Tree(tree)) = inputs.get(1) else {
            return Err(OperatingError::WrongOperation);
        };
        let finding =
            tree_finding(tree, left_on, right_on, view).ok_or(OperatingError::WrongOperation)?;
        *storage = DerivedStorage::NativeLookup(finding);
        Ok(())
    }
    fn view<'a>(
        &self,
        contract: &DerivedContract,
        storage: &'a DerivedStorage,
    ) -> Result<DerivedView<'a>, OperatingError> {
        service::view(DerivedKind::NativeLookup, contract, storage)
    }
}
