//! Semantic lease fields originate in the mandatory borrowed derived-view contract.
use super::*;
impl InvocationResources<'_> {
    pub(crate) fn cycles_view(
        view: Option<&AcquiredView>,
    ) -> Result<Option<&CycleGraph>, OperatingError> {
        match view.map(AcquiredView::view).transpose()? {
            Some(ryo_derived::domain::contract::types::DerivedView::CycleAdjacency(graph)) => {
                Ok(Some(graph))
            }
            None => Ok(None),
            _ => Err(OperatingError::WrongOperation),
        }
    }
    pub(crate) fn ordered_view(
        view: Option<&AcquiredView>,
    ) -> Result<Option<&OrderedMatching>, OperatingError> {
        match view.map(AcquiredView::view).transpose()? {
            Some(ryo_derived::domain::contract::types::DerivedView::OrderedKeys(graph)) => {
                Ok(Some(graph))
            }
            None => Ok(None),
            _ => Err(OperatingError::WrongOperation),
        }
    }
    pub(crate) fn dfs_view(
        view: Option<&AcquiredView>,
    ) -> Result<Option<&PreparedDfs>, OperatingError> {
        match view.map(AcquiredView::view).transpose()? {
            Some(ryo_derived::domain::contract::types::DerivedView::OrderedDfs(graph)) => {
                Ok(Some(graph))
            }
            None => Ok(None),
            _ => Err(OperatingError::WrongOperation),
        }
    }
}

#[cfg(test)]
mod test;
