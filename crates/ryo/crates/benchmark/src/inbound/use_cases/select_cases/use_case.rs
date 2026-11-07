//! The flags of a verification run are read through the suite boundary that resolves selections.
use super::{
    interface::SelectCasesUseCase,
    types::{CaseArgument, CaseRun},
};
use crate::outbound::suite::interface::SuitePort;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use std::sync::Arc;
/// Selects the cases of one verification run from its command line.
pub struct SelectCasesUseCaseImpl {
    /// Resolves a selection file into the case names it lists.
    suite: Arc<dyn SuitePort>,
}
impl SelectCasesUseCaseImpl {
    /// Bind the suite boundary chosen by the composition.
    #[must_use]
    pub fn new(suite: Arc<dyn SuitePort>) -> Self {
        Self { suite }
    }
}
impl SelectCasesUseCase for SelectCasesUseCaseImpl {
    fn select_cases(&self, arguments: &[CaseArgument]) -> Result<CaseRun, Error> {
        super::service::select(self.suite.as_ref(), arguments)
    }
}
impl std::fmt::Debug for SelectCasesUseCaseImpl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}
