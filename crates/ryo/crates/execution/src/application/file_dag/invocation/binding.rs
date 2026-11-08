//! A read-only binding to invocation-owned handles, without copying an all-kind bundle per row.
use super::*;
use ryo_execution_domain::domain::operator::contract::resources::PrimitiveResources;

impl PrimitiveResources for InvocationInputs<'_> {
    fn validate_declared(&self) -> Result<(), OperatingError> {
        // Preserve the common lease's deferred error order, after its row/source gates.
        let _axis = self.axis()?;
        InvocationResources::dfs_view(self.dfs.as_ref())?;
        InvocationResources::cycles_view(self.cycles.as_ref())?;
        InvocationResources::ordered_view(self.ordered.as_ref())?;
        Ok(())
    }
    fn index(&self) -> Option<IndexedRows<'_>> {
        #[cfg(feature = "diagnostic-row-progress")]
        let index = self.index.as_ref().and_then(|index| index.view(&self.progress));
        #[cfg(not(feature = "diagnostic-row-progress"))]
        let index = self.index.as_ref().and_then(|index| index.view());
        index
    }
    fn appended(&self) -> Option<&[Value]> {
        self.appended.as_deref()
    }
    fn rank_layers(&self) -> Option<&[Vec<LiteralText>]> {
        self.rank_layers.as_deref()
    }
    fn axis(
        &self,
    ) -> Result<Option<ryo_execution_domain::domain::operator::contract::views::PreparedAxis<'_>>, OperatingError>
    {
        #[cfg(feature = "diagnostic-row-progress")]
        let axis = self.axis.as_ref().map(|axis| axis.view(&self.progress));
        #[cfg(not(feature = "diagnostic-row-progress"))]
        let axis = self.axis.as_ref().map(|axis| axis.view());
        axis.transpose()
    }
    fn native_join(&self) -> Option<NativeJoin<'_>> {
        // A native lookup is lent only while its derived acquisition is held.
        self.native_storage.as_ref().and(self.native_join)
    }
    fn dfs(&self) -> Result<Option<&PreparedDfs>, OperatingError> {
        InvocationResources::dfs_view(self.dfs.as_ref())
    }
    fn cycles(&self) -> Result<Option<&CycleGraph>, OperatingError> {
        InvocationResources::cycles_view(self.cycles.as_ref())
    }
    fn closure(
        &self,
    ) -> Result<ryo_execution_domain::domain::operator::contract::views::ClosureViews<'_>, OperatingError> {
        Ok(match &self.partitioned_closure {
            Some(parts) => {
                ryo_execution_domain::domain::operator::contract::views::ClosureViews::One(parts.current()?)
            }
            None => ryo_execution_domain::domain::operator::contract::views::ClosureViews::Many(self),
        })
    }
    fn ordered(&self) -> Result<Option<&OrderedMatching>, OperatingError> {
        InvocationResources::ordered_view(self.ordered.as_ref())
    }
    fn groups(&self) -> Option<&[Group]> {
        self.groups.as_deref()
    }
}
