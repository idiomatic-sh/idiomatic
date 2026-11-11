//! The `binding` application operation.
use ryo_derived::domain::cycle_adjacency::types::CycleGraph;
use ryo_derived::domain::dfs::types::PreparedDfs;
use ryo_derived::domain::ordered::types::OrderedMatching;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::operator::contract::views::types::NativeJoin;
use ryo_execution_domain::domain::table_groups::types::Group;
use ryo_execution_domain::domain::table_reach::borrowed::types::IndexedRows;
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::LiteralText;

use super::super::service::*;
use ryo_execution_domain::domain::operator::contract::types::PrimitiveResources;

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
        let index = self
            .index
            .as_ref()
            .and_then(|index| index.view(&self.progress));
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
    ) -> Result<
        Option<ryo_execution_domain::domain::operator::contract::views::types::PreparedAxis<'_>>,
        OperatingError,
    > {
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
    ) -> Result<
        ryo_execution_domain::domain::operator::contract::views::types::ClosureViews<'_>,
        OperatingError,
    > {
        Ok(match &self.partitioned_closure {
            Some(parts) => {
                ryo_execution_domain::domain::operator::contract::views::types::ClosureViews::One(
                    parts.current()?,
                )
            }
            None => {
                ryo_execution_domain::domain::operator::contract::views::types::ClosureViews::Many(
                    &self.closure,
                )
            }
        })
    }
    fn ordered(&self) -> Result<Option<&OrderedMatching>, OperatingError> {
        InvocationResources::ordered_view(self.ordered.as_ref())
    }
    fn groups(&self) -> Option<&[Group]> {
        self.groups.as_deref()
    }
}
