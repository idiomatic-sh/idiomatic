//! The `views` application operation.
use crate::application::file_dag::derived::view::service::LeasedView;
use ryo_derived::domain::cycle_adjacency::types::CycleGraph;
use ryo_derived::domain::dfs::types::PreparedDfs;
use ryo_derived::domain::ordered::types::OrderedMatching;
use ryo_execution_domain::domain::operating::errors::OperatingError;

use super::super::types::InvocationResources;

impl InvocationResources<'_> {
    pub(crate) fn cycles_view(
        view: Option<&LeasedView>,
    ) -> Result<Option<&CycleGraph>, OperatingError> {
        match view.map(LeasedView::view).transpose()? {
            Some(ryo_derived::domain::contract::types::DerivedView::CycleAdjacency(graph)) => {
                Ok(Some(graph))
            }
            None => Ok(None),
            _ => Err(OperatingError::WrongOperation),
        }
    }
    pub(crate) fn ordered_view(
        view: Option<&LeasedView>,
    ) -> Result<Option<&OrderedMatching>, OperatingError> {
        match view.map(LeasedView::view).transpose()? {
            Some(ryo_derived::domain::contract::types::DerivedView::OrderedKeys(graph)) => {
                Ok(Some(graph))
            }
            None => Ok(None),
            _ => Err(OperatingError::WrongOperation),
        }
    }
    pub(crate) fn dfs_view(
        view: Option<&LeasedView>,
    ) -> Result<Option<&PreparedDfs>, OperatingError> {
        match view.map(LeasedView::view).transpose()? {
            Some(ryo_derived::domain::contract::types::DerivedView::OrderedDfs(graph)) => {
                Ok(Some(graph))
            }
            None => Ok(None),
            _ => Err(OperatingError::WrongOperation),
        }
    }
}
