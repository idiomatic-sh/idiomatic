//! Initialize a local workspace and asset package.
use crate::outcome::Outcome;
use tastemaker_core::inbound::use_cases::init::interface::InitUseCase as _;
/// Initialization uses the current directory, or the global `--root` directory.
#[derive(Debug, Clone, Copy, clap::Args)]
#[command(after_help = "Examples:\n  tm init\n  tm init --root my-project --dry-run")]
pub struct InitArguments;
/// Calls the initialization service and shapes its report.
pub(super) fn answer(
    _arguments: InitArguments,
    edge: &crate::edge::Edge,
) -> Result<impl crate::output::Encoded, Outcome> {
    super::composition::build(&edge.store)
        .init(super::schemas::request(edge))
        .map(|report| {
            super::schemas::answer(
                report,
                edge.writing == tastemaker_core::domain::writing::types::Writing::DryRun,
            )
        })
        .map_err(|error| crate::output::fail(&error, Outcome::Error))
}
/// Runs initialization with the shared output and dry-run controls.
pub fn run(arguments: InitArguments, edge: &crate::edge::Edge) -> Outcome {
    match answer(arguments, edge) {
        Ok(answer) => {
            crate::output::present_planned(edge.format, edge.writing, &answer, Outcome::Success)
        }
        Err(outcome) => outcome,
    }
}
