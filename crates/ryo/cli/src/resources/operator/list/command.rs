//! `operator list`: parses, checks, composes, calls the service, presents.

use ryo_core::inbound::use_cases::list_operators::interface::ListOperatorsUseCase as _;

use crate::outcome::Outcome;

/// The arguments of `operator list`.
#[derive(Clone, Copy, Debug, clap::Args)]
#[command(after_help = "Examples:\n  ryo operator list")]
pub struct ListArguments;

/// Checks the arguments, composes the service, calls it once and shapes its answer; when it cannot
/// answer, the error is said on stderr and the outcome the command ends with is returned.
pub(super) fn answer() -> Result<impl crate::output::Encoded, Outcome> {
    super::composition::build()
        .list_operators()
        .map(super::schemas::answer)
        .map_err(|error| crate::output::fail(&error, Outcome::Error))
}

/// Runs `operator list`: checks its arguments through its schemas, connects and composes its
/// service, calls it once, and prints the answer in the format `--format` chose.
pub fn run(_arguments: ListArguments, edge: &crate::edge::Edge) -> Outcome {
    match answer() {
        Ok(answer) => crate::output::present(edge.format, &answer, Outcome::Success),
        Err(outcome) => outcome,
    }
}
