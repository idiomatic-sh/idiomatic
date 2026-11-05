//! A read-only binding to invocation-owned handles, without copying an all-kind bundle per row.
use super::*;
use crate::domain::operator::contract::types::PrimitiveResources;

impl PrimitiveResources for InvocationInputs<'_> {
    fn validate_declared(&self) -> Result<(), OperatingError> {
        // Preserve the common lease's deferred error order, after its row/source gates.
        self.axis.as_ref().map(|axis| axis.view()).transpose()?;
        InvocationResources::dfs_view(self.dfs.as_ref())?;
        InvocationResources::cycles_view(self.cycles.as_ref())?;
        InvocationResources::ordered_view(self.ordered.as_ref())?;
        Ok(())
    }
    fn index(&self) -> Option<IndexedRows<'_>> {
        self.index.as_ref().map(|index| index.view())
    }
    fn appended(&self) -> Option<&[Value]> {
        self.appended.as_deref()
    }
    fn rank_layers(&self) -> Option<&[Vec<LiteralText>]> {
        self.rank_layers.as_deref()
    }
    fn axis(
        &self,
    ) -> Result<Option<crate::domain::operator::contract::axis::PreparedAxis<'_>>, OperatingError>
    {
        self.axis.as_ref().map(|axis| axis.view()).transpose()
    }
    fn native_join(&self) -> Option<NativeJoin<'_>> {
        self.native_join
    }
    fn dfs(&self) -> Result<Option<&PreparedDfs>, OperatingError> {
        InvocationResources::dfs_view(self.dfs.as_ref())
    }
    fn cycles(&self) -> Result<Option<&CycleGraph>, OperatingError> {
        InvocationResources::cycles_view(self.cycles.as_ref())
    }
    fn closure(&self) -> crate::domain::operator::contract::views::ClosureViews<'_> {
        match &self.partitioned_closure {
            Some(parts) => {
                crate::domain::operator::contract::views::ClosureViews::One(parts.current())
            }
            None => crate::domain::operator::contract::views::ClosureViews::Many(&self.closure),
        }
    }
    fn ordered(&self) -> Result<Option<&OrderedMatching>, OperatingError> {
        InvocationResources::ordered_view(self.ordered.as_ref())
    }
    fn groups(&self) -> Option<&[Group]> {
        self.groups.as_deref()
    }
}
