//! The flags of a verification run are read into its typed choices.
use super::{
    interface::SelectCasesUseCase,
    types::{CaseArgument, CaseRun},
};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
/// Selects the cases of one verification run from its command line.
#[derive(Clone, Copy, Debug, Default)]
pub struct SelectCasesUseCaseImpl;
impl SelectCasesUseCase for SelectCasesUseCaseImpl {
    fn select_cases(&self, arguments: &[CaseArgument]) -> Result<CaseRun, Error> {
        super::service::select(arguments)
    }
}
