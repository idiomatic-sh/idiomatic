//! `update`: parses, checks, composes, calls the service, presents.

use tastemaker_core::inbound::use_cases::update::interface::UpdateUseCase as _;

use crate::outcome::Outcome;

/// The arguments of `update`.
#[derive(Debug, clap::Args)]
#[expect(
    clippy::field_scoped_visibility_modifiers,
    reason = "clap fills these fields, the command hands them to the sibling `schemas.rs` and that file's tests read them; `pub` would publish them through the library's public arguments struct, and private fields would hide them from those tests"
)]
#[command(after_help = "Examples:\n  tm update -n\n  tm update <entry> --to <version>")]
pub struct UpdateArguments {
    /// The entry to move, by its alias or its definition's name; every entry when left out.
    pub(super) entry: Option<String>,
    /// The version to move to; the latest its package offers when left out.
    #[arg(long)]
    pub(super) to: Option<String>,
}

/// Checks the arguments, composes the service, calls it once and shapes its answer.
pub(super) fn answer(
    arguments: UpdateArguments,
    edge: &crate::edge::Edge,
) -> Result<impl crate::output::Encoded, Outcome> {
    let dry = edge.writing == tastemaker_core::domain::writing::types::Writing::DryRun;
    let request = super::schemas::request(arguments.entry, arguments.to, edge)
        .map_err(|error| crate::output::fail(&error, Outcome::Failure))?;
    super::composition::build(&edge.store)
        .update(request)
        .map(|report| super::schemas::answer(report, dry))
        .map_err(|error| crate::output::fail(&error, Outcome::Error))
}

/// Runs `update`: with `-n`, lists the entries behind their latest version; without, moves them.
pub fn run(arguments: UpdateArguments, edge: &crate::edge::Edge) -> Outcome {
    match answer(arguments, edge) {
        Ok(answer) => {
            crate::output::present_planned(edge.format, edge.writing, &answer, Outcome::Success)
        }
        Err(outcome) => outcome,
    }
}
