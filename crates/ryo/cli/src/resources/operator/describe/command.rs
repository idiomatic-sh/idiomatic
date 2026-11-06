//! `operator describe`: parses, checks, composes, calls the service, presents.

use ryo_core::inbound::use_cases::describe_operator::interface::DescribeOperatorUseCase as _;

use crate::outcome::Outcome;

/// The arguments of `operator describe`.
#[derive(Clone, Debug, clap::Args)]
#[expect(
    clippy::field_scoped_visibility_modifiers,
    reason = "clap fills these fields, the command hands them to the sibling `schemas.rs` and that file's tests read them; `pub` would publish them through the library's public arguments type, and private fields would hide them from those tests"
)]
#[command(after_help = "Examples:\n  ryo operator describe antijoin")]
pub struct DescribeArguments {
    /// The operator to describe.
    pub(super) name: String,
}

/// Checks the arguments, composes the service, calls it once and shapes its answer; when it cannot
/// answer, the error is said on stderr and the outcome the command ends with is returned.
pub(super) fn answer(arguments: DescribeArguments) -> Result<impl crate::output::Encoded, Outcome> {
    let request = super::schemas::request(arguments.name)
        .map_err(|error| crate::output::fail(&error, Outcome::Failure))?;
    super::composition::build()
        .describe_operator(request)
        .map(super::schemas::answer)
        .map_err(|error| crate::output::fail(&error, Outcome::Error))
}

/// Runs `operator describe`: checks its arguments through its schemas, connects and composes its
/// service, calls it once, and prints the answer in the format `--format` chose.
pub fn run(arguments: DescribeArguments, edge: &crate::edge::Edge) -> Outcome {
    match answer(arguments) {
        Ok(answer) => crate::output::present(edge.format, &answer, Outcome::Success),
        Err(outcome) => outcome,
    }
}
