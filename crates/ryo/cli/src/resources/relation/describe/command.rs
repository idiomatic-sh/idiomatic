//! `relation describe`: parses, checks, composes, calls the service, presents.

use ryo_core::inbound::use_cases::describe_relation::interface::DescribeRelationUseCase as _;

use crate::outcome::CommandOutcome;

/// The arguments of `relation describe`.
#[derive(Clone, Debug, clap::Args)]
#[expect(
    clippy::field_scoped_visibility_modifiers,
    reason = "clap fills these fields, the command hands them to the sibling `schemas.rs` and that file's tests read them; `pub` would publish them through the library's public arguments type, and private fields would hide them from those tests"
)]
#[command(after_help = "Examples:\n  ryo relation describe function")]
pub struct DescribeArguments {
    /// The relation to describe.
    pub(super) name: String,
}

/// Checks the arguments, composes the service, calls it once and shapes its answer; when it cannot
/// answer, the error is said on stderr and the outcome the command ends with is returned.
pub(super) fn answer(arguments: DescribeArguments) -> Result<impl crate::output::Encoded, CommandOutcome> {
    let request = super::schemas::request(arguments.name)
        .map_err(|error| crate::output::fail(&error, CommandOutcome::Failure))?;
    super::composition::build()
        .describe_relation(request)
        .map(super::schemas::answer)
        .map_err(|error| crate::output::fail(&error, CommandOutcome::Error))
}

/// Runs `relation describe`: checks its arguments through its schemas, connects and composes its
/// service, calls it once, and prints the answer in the format `--format` chose.
pub fn run(arguments: DescribeArguments, edge: &crate::edge::Edge) -> CommandOutcome {
    match answer(arguments) {
        Ok(answer) => crate::output::present(edge.format, &answer, CommandOutcome::Success),
        Err(outcome) => outcome,
    }
}
