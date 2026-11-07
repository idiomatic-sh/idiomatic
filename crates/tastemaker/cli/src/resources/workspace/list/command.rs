//! `list`: parses, checks, composes, calls the service, presents.

use tastemaker_core::inbound::use_cases::list::interface::ListUseCase as _;

use crate::outcome::Outcome;

/// The arguments of `list`.
#[derive(Debug, clap::Args)]
#[expect(
    clippy::field_scoped_visibility_modifiers,
    reason = "clap fills these fields, the command hands them to the sibling `schemas.rs` and that file's tests read them; `pub` would publish them through the library's public arguments struct, and private fields would hide them from those tests"
)]
#[command(after_help = "Examples:\n  tm list\n  tm list rules length\n  tm list relations")]
pub struct ListArguments {
    /// Which kind: `rules`, `specs`, `templates`, or ryo's `relations` or `operators`; every definition when left out.
    pub(super) kind: Option<String>,
    /// Only the definitions whose name or description holds this text.
    pub(super) term: Option<String>,
}

/// Checks the arguments, composes the service, calls it once and shapes its answer.
pub(super) fn answer(
    arguments: ListArguments,
    edge: &crate::edge::Edge,
) -> Result<impl crate::output::Encoded, Outcome> {
    let request = super::schemas::request(arguments.kind, arguments.term, edge)
        .map_err(|error| crate::output::fail(&error, Outcome::Failure))?;
    super::composition::build(&edge.store)
        .list(request)
        .map(super::schemas::answer)
        .map_err(|error| crate::output::fail(&error, Outcome::Error))
}

/// Runs `list`: the definitions of the workspace's packages, each at its latest version.
pub fn run(arguments: ListArguments, edge: &crate::edge::Edge) -> Outcome {
    let format = edge.format;
    match answer(arguments, edge) {
        Ok(answer) => crate::output::present(format, &answer, Outcome::Success),
        Err(outcome) => outcome,
    }
}
