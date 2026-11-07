//! The native resource and its required acquisition cannot be detached.
use crate::application::file_dag::derived::owner::acquisition::admission::service::AcquiredView;
use crate::domain::{operating::errors::OperatingError, operator::contract::join::NativeJoin};
/// A borrowed acquired native resource. Its fields can be minted only by
/// InvocationResources after the required derived acquisition has succeeded.
pub(in crate::application::file_dag) struct NativeJoinLease<'a> {
    native: NativeJoin<'a>,
    acquired: &'a AcquiredView,
}
impl<'a> NativeJoinLease<'a> {
    pub(in crate::application::file_dag::invocation) fn new(
        native: NativeJoin<'a>,
        acquired: &'a AcquiredView,
    ) -> Self {
        Self { native, acquired }
    }
}
impl NativeJoinLease<'_> {
    pub(in crate::application::file_dag) fn tree(
        &self,
    ) -> crate::domain::table_cells::types::TreeRows<'_> {
        self.native.tree
    }
    pub(in crate::application::file_dag) fn finding(
        &self,
    ) -> Result<ryo_derived::domain::native::types::NativeFinding, OperatingError> {
        let ryo_derived::domain::contract::types::DerivedView::NativeLookup(finding) =
            self.acquired.view()?
        else {
            return Err(OperatingError::WrongOperation);
        };
        if *finding != self.native.finding {
            return Err(OperatingError::WrongOperation);
        }
        Ok(*finding)
    }
    pub(in crate::application::file_dag) fn contract(
        &self,
    ) -> &crate::domain::derived_views::types::DerivedContract {
        self.acquired.contract()
    }
}
